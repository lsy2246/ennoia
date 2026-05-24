use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use ennoia_contract::ApiError;
use ennoia_error_utils::normalize_error_message;
use ennoia_extension_host::RegisteredProviderContribution;
use ennoia_kernel::{
    AgentDocument, ModelEndpointConfig, OperationPerformRequest, OperationPerformResponse,
    OperationRecord, OperationStatus, PermissionRequest, PermissionScope, PermissionTarget,
    PermissionTrigger, RuntimeOperationRequest, RuntimeOperationTimeoutConfig,
};
use ennoia_logs::RequestContext;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::io::AsyncWriteExt;

use crate::app::{live_server_config, AppState};
use crate::execution::{resolve_command_cwd, AgentFileAccessPaths};
use crate::logs_store::{LogEntryWrite, LOGS_COMPONENT_HOST};
use crate::realtime::RealtimeEvent;
use crate::routes::actions::dispatch_hook_event;
use crate::routes::extensions::invoke_provider_json_with_request;
use crate::routes::scoped;
use crate::skills::{load_skill_manifest, run_skill_action};

const PROVIDER_NODE_RUNNER: &str = r#"
import { pathToFileURL } from 'node:url';

const entry = process.argv[1];
const mod = await import(pathToFileURL(entry).href);
const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
const raw = Buffer.concat(chunks).toString('utf8').trim();
const request = raw ? JSON.parse(raw) : {};
const params = request.params ?? {};
const result = request.method === 'list_models'
  ? await mod.listModels(params)
  : await mod.generate(params);
process.stdout.write(JSON.stringify({ ok: true, result }));
"#;

#[derive(Debug, Clone, Serialize)]
pub struct RuntimeOperationResult {
    pub operation: String,
    pub content: JsonValue,
}

pub fn model_endpoint_runtime_request_config(model_endpoint: &ModelEndpointConfig) -> JsonValue {
    serde_json::json!({
        "id": model_endpoint.id,
        "display_name": model_endpoint.display_name,
        "kind": model_endpoint.kind,
        "description": model_endpoint.description,
        "base_url": model_endpoint.base_url,
        "api_key": model_endpoint.api_key,
        "api_key_env": model_endpoint.api_key_env,
        "default_model": model_endpoint.default_model,
        "available_models": model_endpoint.available_models,
        "model_discovery": model_endpoint.model_discovery,
        "enabled": model_endpoint.enabled,
    })
}

pub fn resolve_provider_entry_path(
    contribution: &RegisteredProviderContribution,
) -> std::io::Result<PathBuf> {
    let entry = contribution
        .provider
        .entry
        .as_deref()
        .ok_or_else(|| std::io::Error::other("provider entry missing"))?;
    let path = PathBuf::from(&contribution.install_dir).join(entry);
    std::fs::canonicalize(path)
}

pub async fn invoke_provider_method(
    entry: &PathBuf,
    payload: &JsonValue,
    provider: &ModelEndpointConfig,
) -> Result<JsonValue, String> {
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|error| format!("serialize provider request failed: {error}"))?;
    let entry_string = entry
        .to_str()
        .ok_or_else(|| "provider entry path is not valid utf-8".to_string())?
        .to_string();
    let mut command = tokio::process::Command::new("node");
    command
        .args([
            "--input-type=module",
            "-e",
            PROVIDER_NODE_RUNNER,
            &entry_string,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some((name, value)) = resolve_model_endpoint_env_binding(provider) {
        command.env(name, value);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn provider runner failed: {error}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(&payload_bytes)
            .await
            .map_err(|error| format!("write provider request failed: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .await
        .map_err(|error| format!("wait provider runner failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("provider runner exited with status {}", output.status)
        } else {
            normalize_error_message(detail)
        });
    }
    serde_json::from_slice::<JsonValue>(&output.stdout)
        .map_err(|error| format!("parse provider response failed: {error}"))
}

pub fn authorize_provider_generate(
    state: &AppState,
    request: &RequestContext,
    agent_id: &str,
    contribution: &RegisteredProviderContribution,
    model_endpoint: &ModelEndpointConfig,
    conversation_id: &str,
    run_id: &str,
    message_id: Option<&str>,
    trigger_kind: &str,
) -> Result<Option<String>, ApiError> {
    let permission_request = PermissionRequest {
        agent_id: agent_id.to_string(),
        action: "provider.generate".to_string(),
        target: PermissionTarget {
            kind: "provider".to_string(),
            id: model_endpoint.id.clone(),
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            path: None,
            host: normalize_optional_runtime_value(&model_endpoint.base_url),
        },
        scope: PermissionScope {
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            message_id: message_id.map(str::to_string),
            extension_id: Some(contribution.extension_id.clone()),
            path: None,
            host: normalize_optional_runtime_value(&model_endpoint.base_url),
        },
        trigger: PermissionTrigger {
            kind: trigger_kind.to_string(),
            user_initiated: true,
        },
    };
    authorize_permission_request(state, request, &permission_request)
}

pub fn authorize_runtime_operation(
    state: &AppState,
    request: &RequestContext,
    agent_id: &str,
    action: &str,
    target_kind: &str,
    target_id: &str,
    conversation_id: &str,
    run_id: &str,
    message_id: Option<&str>,
    path: Option<&str>,
    host: Option<&str>,
    trigger_kind: &str,
) -> Result<Option<String>, ApiError> {
    let permission_request = PermissionRequest {
        agent_id: agent_id.to_string(),
        action: action.to_string(),
        target: PermissionTarget {
            kind: target_kind.to_string(),
            id: target_id.to_string(),
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            path: path.map(str::to_string),
            host: host.map(str::to_string),
        },
        scope: PermissionScope {
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            message_id: message_id.map(str::to_string),
            extension_id: Some("runtime".to_string()),
            path: path.map(str::to_string),
            host: host.map(str::to_string),
        },
        trigger: PermissionTrigger {
            kind: trigger_kind.to_string(),
            user_initiated: true,
        },
    };
    authorize_permission_request(state, request, &permission_request)
}

pub async fn execute_runtime_operation(
    state: &AppState,
    request: &RequestContext,
    operation: &str,
    payload: RuntimeOperationRequest,
) -> Result<RuntimeOperationResult, ApiError> {
    let agent = crate::app::load_agent_document(&state.runtime_paths, &payload.agent_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("agent '{}' not found", payload.agent_id)),
                request,
            )
        })?;
    let content = match operation {
        "command.exec" => execute_command_exec(state, request, &agent, &payload).await?,
        other if other.starts_with("skill.") => {
            execute_skill_action(state, request, &agent, other, &payload).await?
        }
        other => {
            return Err(scoped(
                ApiError::bad_request(format!("unsupported runtime operation '{other}'")),
                request,
            ));
        }
    };
    Ok(RuntimeOperationResult {
        operation: operation.to_string(),
        content,
    })
}

pub async fn perform_operation(
    state: &AppState,
    request: &RequestContext,
    extension_id: &str,
    payload: OperationPerformRequest,
) -> Result<OperationPerformResponse, ApiError> {
    let queued = state
        .operations
        .create_operation(extension_id, &payload)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    publish_operation_update(state, request, &queued);
    let running = state
        .operations
        .update_operation(&queued.id, OperationStatus::Running, None, None)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    publish_operation_update(state, request, &running);
    log_operation_event(
        state,
        request,
        "runtime.operation.started",
        "info",
        &queued.id,
        "runtime operation started",
        serde_json::json!({
            "extension_id": extension_id,
            "status": running.status.as_str(),
            "kind": payload.kind,
            "name": payload.name,
            "agent_id": payload.agent_id,
            "conversation_id": payload.conversation_id,
            "branch_id": payload.branch_id,
            "lane_id": payload.lane_id,
            "run_id": payload.run_id,
            "message_id": payload.message_id,
            "input": payload.input,
        }),
    );

    if payload.deferred {
        let state_for_task = state.clone();
        let request_for_task = request.clone();
        let payload_for_task = payload.clone();
        let operation_id = queued.id.clone();
        tokio::spawn(async move {
            let _ = complete_operation_execution(
                &state_for_task,
                &request_for_task,
                &operation_id,
                &payload_for_task,
            )
            .await;
        });
        return Ok(OperationPerformResponse {
            operation: running,
            content: JsonValue::Null,
        });
    }

    complete_operation_execution(state, request, &queued.id, &payload).await
}

pub async fn resume_operation_after_approval(
    state: &AppState,
    request: &RequestContext,
    approval_id: &str,
) -> Result<Option<OperationRecord>, ApiError> {
    let Some(target) = state
        .operations
        .find_resume_target_by_approval(approval_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?
    else {
        return Ok(None);
    };
    let payload = OperationPerformRequest {
        agent_id: target.operation.agent_id.clone(),
        conversation_id: target.operation.conversation_id.clone(),
        run_id: target.operation.run_id.clone(),
        branch_id: target.operation.branch_id.clone(),
        lane_id: target.operation.lane_id.clone(),
        message_id: target.operation.message_id.clone(),
        kind: target.operation.kind.clone(),
        name: target.operation.name.clone(),
        deferred: false,
        input: target.operation.input.clone(),
    };
    let running = state
        .operations
        .update_operation(&target.operation.id, OperationStatus::Running, None, None)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    publish_operation_update(state, request, &running);
    log_operation_event(
        state,
        request,
        "runtime.operation.resumed",
        "info",
        &running.id,
        "runtime operation resumed after approval",
        serde_json::json!({
            "status": running.status.as_str(),
            "kind": running.kind,
            "name": running.name,
            "agent_id": running.agent_id,
            "conversation_id": running.conversation_id,
            "branch_id": running.branch_id,
            "lane_id": running.lane_id,
            "run_id": running.run_id,
            "message_id": running.message_id,
            "approval_id": approval_id,
        }),
    );
    match execute_operation_payload(state, request, &payload).await {
        Ok(content) => {
            let operation = state
                .operations
                .update_operation(
                    &target.operation.id,
                    OperationStatus::Succeeded,
                    Some(content),
                    None,
                )
                .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
            publish_operation_update(state, request, &operation);
            log_operation_event(
                state,
                request,
                "runtime.operation.succeeded",
                "info",
                &operation.id,
                "runtime operation completed",
                serde_json::json!({
                    "status": operation.status.as_str(),
                    "kind": operation.kind,
                    "name": operation.name,
                    "agent_id": operation.agent_id,
                    "conversation_id": operation.conversation_id,
                    "branch_id": operation.branch_id,
                    "lane_id": operation.lane_id,
                    "run_id": operation.run_id,
                    "message_id": operation.message_id,
                }),
            );
            Ok(Some(operation))
        }
        Err(error) => {
            let status = if is_permission_approval_error(&error) {
                OperationStatus::Blocked
            } else if error.code() == ennoia_contract::ErrorCode::Forbidden {
                OperationStatus::Cancelled
            } else {
                OperationStatus::Failed
            };
            let operation = state
                .operations
                .update_operation(
                    &target.operation.id,
                    status.clone(),
                    None,
                    Some(operation_error_details(&error, &target.operation.id)),
                )
                .map_err(|store_error| {
                    scoped(ApiError::internal(store_error.to_string()), request)
                })?;
            if let Some(next_approval_id) = error
                .details()
                .get("approval_id")
                .and_then(JsonValue::as_str)
            {
                state
                    .operations
                    .link_approval(&target.operation.id, next_approval_id)
                    .map_err(|store_error| {
                        scoped(ApiError::internal(store_error.to_string()), request)
                    })?;
            }
            publish_operation_update(state, request, &operation);
            log_operation_event(
                state,
                request,
                "runtime.operation.failed",
                if status == OperationStatus::Blocked {
                    "warn"
                } else {
                    "error"
                },
                &operation.id,
                "runtime operation failed",
                serde_json::json!({
                    "status": operation.status.as_str(),
                    "kind": operation.kind,
                    "name": operation.name,
                    "agent_id": operation.agent_id,
                    "conversation_id": operation.conversation_id,
                    "branch_id": operation.branch_id,
                    "lane_id": operation.lane_id,
                    "run_id": operation.run_id,
                    "message_id": operation.message_id,
                    "error": operation.error,
                }),
            );
            Ok(Some(operation))
        }
    }
}

async fn complete_operation_execution(
    state: &AppState,
    request: &RequestContext,
    operation_id: &str,
    payload: &OperationPerformRequest,
) -> Result<OperationPerformResponse, ApiError> {
    let result = execute_operation_payload(state, request, payload).await;
    match result {
        Ok(content) => {
            let operation = state
                .operations
                .update_operation(
                    operation_id,
                    OperationStatus::Succeeded,
                    Some(content.clone()),
                    None,
                )
                .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
            publish_operation_update(state, request, &operation);
            log_operation_event(
                state,
                request,
                "runtime.operation.succeeded",
                "info",
                &operation.id,
                "runtime operation completed",
                serde_json::json!({
                    "status": operation.status.as_str(),
                    "kind": operation.kind,
                    "name": operation.name,
                    "agent_id": operation.agent_id,
                    "conversation_id": operation.conversation_id,
                    "branch_id": operation.branch_id,
                    "lane_id": operation.lane_id,
                    "run_id": operation.run_id,
                    "message_id": operation.message_id,
                }),
            );
            Ok(OperationPerformResponse { operation, content })
        }
        Err(error) => {
            let error_details = operation_error_details(&error, operation_id);
            let merged_error_details = match error.details() {
                JsonValue::Object(existing) => {
                    let mut merged = existing.clone();
                    merged.insert(
                        "operation_id".to_string(),
                        JsonValue::String(operation_id.to_string()),
                    );
                    JsonValue::Object(merged)
                }
                _ => serde_json::json!({ "operation_id": operation_id }),
            };
            let approval_id = error
                .details()
                .get("approval_id")
                .and_then(JsonValue::as_str)
                .map(str::to_string);
            let status = if is_permission_approval_error(&error) {
                OperationStatus::Blocked
            } else if error.code() == ennoia_contract::ErrorCode::Forbidden {
                OperationStatus::Cancelled
            } else {
                OperationStatus::Failed
            };
            let operation = state
                .operations
                .update_operation(
                    operation_id,
                    status.clone(),
                    None,
                    Some(error_details.clone()),
                )
                .map_err(|store_error| {
                    scoped(ApiError::internal(store_error.to_string()), request)
                })?;
            if let Some(approval_id) = approval_id.as_deref() {
                state
                    .operations
                    .link_approval(operation_id, approval_id)
                    .map_err(|store_error| {
                        scoped(ApiError::internal(store_error.to_string()), request)
                    })?;
            }
            publish_operation_update(state, request, &operation);
            log_operation_event(
                state,
                request,
                "runtime.operation.failed",
                if status == OperationStatus::Blocked {
                    "warn"
                } else {
                    "error"
                },
                &operation.id,
                "runtime operation failed",
                serde_json::json!({
                    "status": operation.status.as_str(),
                    "kind": operation.kind,
                    "name": operation.name,
                    "agent_id": operation.agent_id,
                    "conversation_id": operation.conversation_id,
                    "branch_id": operation.branch_id,
                    "lane_id": operation.lane_id,
                    "run_id": operation.run_id,
                    "message_id": operation.message_id,
                    "error": error_details,
                }),
            );
            Err(error.with_details(merged_error_details))
        }
    }
}

async fn execute_operation_payload(
    state: &AppState,
    request: &RequestContext,
    payload: &OperationPerformRequest,
) -> Result<JsonValue, ApiError> {
    match (payload.kind.as_str(), payload.name.as_str()) {
        ("provider", "generate") => {
            execute_provider_generate_operation(state, request, payload).await
        }
        ("runtime", name) if name == "command.exec" || name.starts_with("skill.") => {
            let result = execute_runtime_operation(
                state,
                request,
                payload.name.as_str(),
                RuntimeOperationRequest {
                    agent_id: payload.agent_id.clone(),
                    conversation_id: payload.conversation_id.clone(),
                    run_id: payload.run_id.clone(),
                    message_id: payload.message_id.clone(),
                    arguments: payload.input.clone(),
                },
            )
            .await?;
            Ok(result.content)
        }
        _ => Err(scoped(
            ApiError::bad_request(format!(
                "unsupported operation '{}:{}'",
                payload.kind, payload.name
            )),
            request,
        )),
    }
}

async fn execute_provider_generate_operation(
    state: &AppState,
    request: &RequestContext,
    payload: &OperationPerformRequest,
) -> Result<JsonValue, ApiError> {
    let provider_kind = payload
        .input
        .get("provider_kind")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| {
            scoped(
                ApiError::bad_request("provider operation missing provider_kind"),
                request,
            )
        })?;
    let params = payload
        .input
        .get("params")
        .cloned()
        .unwrap_or(JsonValue::Null);
    invoke_provider_json_with_request(
        state,
        request,
        provider_kind,
        "generate",
        ennoia_kernel::ExtensionRpcRequest {
            params,
            context: serde_json::json!({
                "permission_actor": {
                    "agent_id": payload.agent_id,
                    "kind": "operation.provider.generate",
                    "conversation_id": payload.conversation_id,
                    "run_id": payload.run_id,
                    "message_id": payload.message_id,
                }
            }),
        },
    )
    .await
}

pub(crate) fn publish_operation_update(
    state: &AppState,
    request: &RequestContext,
    operation: &OperationRecord,
) {
    dispatch_hook_event(
        state,
        request,
        ennoia_kernel::HOOK_EVENT_OPERATION_UPDATED,
        "operation",
        &operation.id,
        serde_json::json!({
            "operation": operation,
            "conversation_id": operation.conversation_id,
            "run_id": operation.run_id,
            "message_id": operation.message_id,
            "lane_id": operation.lane_id,
        }),
    );
    state.realtime.publish(RealtimeEvent::ConversationChanged {
        conversation_id: operation.conversation_id.clone(),
    });
}

fn is_permission_approval_error(error: &ApiError) -> bool {
    error.code() == ennoia_contract::ErrorCode::Forbidden
        && error.details().get("decision").and_then(JsonValue::as_str) == Some("ask")
}

fn operation_error_details(error: &ApiError, operation_id: &str) -> JsonValue {
    match error.details() {
        JsonValue::Object(existing) => {
            let mut merged = existing.clone();
            merged.insert(
                "operation_id".to_string(),
                JsonValue::String(operation_id.to_string()),
            );
            merged.insert(
                "message".to_string(),
                JsonValue::String(error.message().to_string()),
            );
            JsonValue::Object(merged)
        }
        _ => serde_json::json!({
            "operation_id": operation_id,
            "message": error.message(),
        }),
    }
}

fn log_operation_event(
    state: &AppState,
    request: &RequestContext,
    event: &str,
    level: &str,
    operation_id: &str,
    message: &str,
    attributes: JsonValue,
) {
    let _ = state.logs.append_log_scoped(
        LogEntryWrite {
            event: event.to_string(),
            level: level.to_string(),
            component: LOGS_COMPONENT_HOST.to_string(),
            source_kind: "operation".to_string(),
            source_id: Some(operation_id.to_string()),
            message: message.to_string(),
            attributes,
            created_at: None,
        },
        Some(&request.trace_context()),
    );
}

fn resolve_command_timeout_ms(
    arguments: &JsonValue,
    config: &RuntimeOperationTimeoutConfig,
) -> u64 {
    integer_argument(arguments, "timeout_ms")
        .unwrap_or(config.default_timeout_ms as i64)
        .clamp(config.min_timeout_ms as i64, config.max_timeout_ms as i64) as u64
}

fn normalize_command_exec_invocation(
    command: &str,
    args: &[String],
) -> (String, Vec<String>, bool) {
    if cfg!(windows)
        && (command.eq_ignore_ascii_case("cmd") || command.eq_ignore_ascii_case("cmd.exe"))
    {
        let has_mode_flag = args
            .first()
            .map(|value| {
                let normalized = value.trim().to_ascii_lowercase();
                normalized == "/c" || normalized == "/k"
            })
            .unwrap_or(false);
        if !has_mode_flag && !args.is_empty() {
            let mut normalized_args = Vec::with_capacity(args.len() + 1);
            normalized_args.push("/c".to_string());
            normalized_args.extend(args.iter().cloned());
            return (command.to_string(), normalized_args, true);
        }
    }
    (command.to_string(), args.to_vec(), false)
}

fn format_command_invocation(command: &str, args: &[String]) -> String {
    std::iter::once(command)
        .chain(args.iter().map(String::as_str))
        .map(format_command_invocation_segment)
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_command_invocation_segment(value: &str) -> String {
    let normalized = value.trim().replace('\\', "/");
    if normalized.is_empty() {
        return "\"\"".to_string();
    }
    if normalized
        .chars()
        .any(|character| character.is_whitespace() || character == '"')
    {
        return format!("\"{}\"", normalized.replace('"', "\\\""));
    }
    normalized
}

fn authorize_permission_request(
    state: &AppState,
    request: &RequestContext,
    permission_request: &PermissionRequest,
) -> Result<Option<String>, ApiError> {
    let decision = state
        .agent_permissions
        .evaluate_request(permission_request, Some(request))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    state
        .realtime
        .publish(RealtimeEvent::PermissionAgentChanged {
            agent_id: permission_request.agent_id.clone(),
        });
    if let Some(conversation_id) = permission_request.scope.conversation_id.clone() {
        state
            .realtime
            .publish(RealtimeEvent::PermissionConversationChanged { conversation_id });
    }
    match decision.decision.as_str() {
        "allow" => Ok(decision.grant_id),
        "ask" => {
            let approval_id = decision
                .approval_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            Err(scoped(
                ApiError::forbidden(format!(
                    "approval required: action={}, approval_id={approval_id}",
                    permission_request.action
                ))
                .with_details(serde_json::json!({
                    "decision": decision.decision,
                    "approval_id": decision.approval_id,
                    "agent_id": permission_request.agent_id,
                    "action": permission_request.action,
                    "target": permission_request.target,
                    "scope": permission_request.scope,
                    "reason": decision.reason,
                })),
                request,
            ))
        }
        _ => Err(scoped(
            ApiError::forbidden(format!(
                "permission denied: action={}, reason={}",
                permission_request.action, decision.reason
            ))
            .with_details(serde_json::json!({
                "decision": decision.decision,
                "agent_id": permission_request.agent_id,
                "action": permission_request.action,
                "target": permission_request.target,
                "scope": permission_request.scope,
                "reason": decision.reason,
            })),
            request,
        )),
    }
}

fn consume_runtime_grant(
    state: &AppState,
    request: &RequestContext,
    grant_id: Option<String>,
) -> Result<(), ApiError> {
    if let Some(grant_id) = grant_id {
        state
            .agent_permissions
            .consume_grant(&grant_id)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    }
    Ok(())
}

async fn execute_command_exec(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentDocument,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let agent_profile = &agent.profile;
    let server_config = live_server_config(state);
    let command_name = required_string_argument(&payload.arguments, "command", request)?;
    let requested_args = string_array_argument(&payload.arguments, "args");
    let (effective_command_name, effective_args, invocation_normalized) =
        normalize_command_exec_invocation(&command_name, &requested_args);
    let formatted_invocation = format_command_invocation(&effective_command_name, &effective_args);
    let cwd_value = string_argument(&payload.arguments, "cwd");
    let file_access_paths = AgentFileAccessPaths::for_agent(state, agent_profile, &payload.run_id);
    let cwd = resolve_command_cwd(&agent.file_access, &file_access_paths, cwd_value.as_deref())
        .map_err(|error| scoped(error, request))?;
    let operation_config = &server_config.operations.command;
    let timeout_ms = resolve_command_timeout_ms(&payload.arguments, operation_config);
    fs::create_dir_all(&cwd.host_path).map_err(|error| {
        scoped(
            ApiError::internal(format!("prepare command working dir failed: {error}")),
            request,
        )
    })?;
    let grant_id = authorize_runtime_operation(
        state,
        request,
        &agent_profile.id,
        "command.exec",
        "command",
        &formatted_invocation,
        &payload.conversation_id,
        &payload.run_id,
        payload.message_id.as_deref(),
        Some(&cwd.display_path),
        None,
        "runtime.operation",
    )?;
    consume_runtime_grant(state, request, grant_id)?;
    let error_details = serde_json::json!({
        "source": "runtime.operation",
        "operation": "command.exec",
        "command": command_name,
        "args": requested_args,
        "effective_command": effective_command_name,
        "effective_args": effective_args,
        "invocation": formatted_invocation,
        "cwd": cwd.display_path,
        "timeout_ms": timeout_ms,
        "invocation_normalized": invocation_normalized,
        "agent_id": agent_profile.id,
        "conversation_id": payload.conversation_id,
        "run_id": payload.run_id,
        "message_id": payload.message_id,
    });
    let mut command = tokio::process::Command::new(&effective_command_name);
    command
        .args(&effective_args)
        .current_dir(cwd.host_path.clone());
    let output = tokio::time::timeout(Duration::from_millis(timeout_ms), command.output())
        .await
        .map_err(|_| {
            scoped(
                ApiError::timeout(format!("command exec timed out after {timeout_ms}ms"))
                    .with_details(error_details.clone()),
                request,
            )
        })?
        .map_err(|error| {
            scoped(
                ApiError::internal(format!("spawn command failed: {error}"))
                    .with_details(error_details.clone()),
                request,
            )
        })?;
    let content = serde_json::json!({
        "ok": output.status.success(),
        "tool": "command.exec",
        "command": effective_command_name,
        "args": effective_args,
        "invocation": formatted_invocation,
        "cwd": cwd.display_path,
        "status": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
        "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
    })
    .to_string();
    Ok(parse_runtime_content(&content))
}

async fn execute_skill_action(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentDocument,
    operation: &str,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let (skill_id, action_id) = parse_skill_operation(operation, request)?;
    let agent_profile = &agent.profile;
    if !agent_profile
        .skills
        .iter()
        .any(|item| item.trim() == skill_id)
    {
        return Err(scoped(
            ApiError::forbidden(format!(
                "agent '{}' is not configured for skill '{skill_id}'",
                agent_profile.id
            )),
            request,
        ));
    }
    let skill = crate::app::load_skill_configs(&state.runtime_paths, state.allow_dev_sources)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?
        .into_iter()
        .find(|item| item.id == skill_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("skill '{skill_id}' not found")),
                request,
            )
        })?;
    if !skill.enabled {
        return Err(scoped(
            ApiError::forbidden(format!("skill '{skill_id}' is disabled")),
            request,
        ));
    }
    if !skill.actions.iter().any(|item| item.id == action_id) {
        return Err(scoped(
            ApiError::not_found(format!("skill '{skill_id}' action '{action_id}' not found")),
            request,
        ));
    }
    let args = skill_action_args(skill_id, action_id, &payload.arguments, request)?;
    let grant_id = authorize_runtime_operation(
        state,
        request,
        &agent_profile.id,
        operation,
        "skill",
        &format!("{skill_id}.{action_id}"),
        &payload.conversation_id,
        &payload.run_id,
        payload.message_id.as_deref(),
        None,
        None,
        "runtime.operation",
    )?;
    consume_runtime_grant(state, request, grant_id)?;
    let manifest = load_skill_manifest(&state.runtime_paths, skill_id, state.allow_dev_sources)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    run_skill_action(
        &state.runtime_paths,
        &manifest,
        action_id,
        args,
        state.allow_dev_sources,
    )
    .await
    .map_err(|error| scoped(ApiError::internal(error.to_string()), request))
}

fn parse_skill_operation<'a>(
    operation: &'a str,
    request: &RequestContext,
) -> Result<(&'a str, &'a str), ApiError> {
    operation
        .strip_prefix("skill.")
        .and_then(|value| value.rsplit_once('.'))
        .filter(|(skill_id, action_id)| !skill_id.trim().is_empty() && !action_id.trim().is_empty())
        .ok_or_else(|| {
            scoped(
                ApiError::bad_request(format!("invalid skill operation '{operation}'")),
                request,
            )
        })
}

fn skill_action_args(
    skill_id: &str,
    action_id: &str,
    arguments: &JsonValue,
    request: &RequestContext,
) -> Result<Vec<String>, ApiError> {
    match (skill_id, action_id) {
        ("web-search", "search") => web_search_action_args(arguments, request),
        _ => Err(scoped(
            ApiError::bad_request(format!("unsupported skill action '{skill_id}.{action_id}'")),
            request,
        )),
    }
}

fn web_search_action_args(
    arguments: &JsonValue,
    request: &RequestContext,
) -> Result<Vec<String>, ApiError> {
    let query = required_string_argument(arguments, "query", request)?;
    let mut args = vec![query];
    if let Some(limit) = integer_argument(arguments, "limit") {
        args.push("--limit".to_string());
        args.push(limit.to_string());
    }
    if let Some(pages) = integer_argument(arguments, "pages") {
        args.push("--pages".to_string());
        args.push(pages.to_string());
    }
    if let Some(format) = string_argument(arguments, "format") {
        args.push("--format".to_string());
        args.push(format);
    }
    Ok(args)
}

fn required_string_argument(
    arguments: &JsonValue,
    key: &str,
    request: &RequestContext,
) -> Result<String, ApiError> {
    string_argument(arguments, key).ok_or_else(|| {
        scoped(
            ApiError::bad_request(format!("tool argument '{key}' is required")),
            request,
        )
    })
}

fn string_argument(arguments: &JsonValue, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(JsonValue::as_str)
        .map(str::to_string)
}

fn integer_argument(arguments: &JsonValue, key: &str) -> Option<i64> {
    arguments.get(key).and_then(JsonValue::as_i64)
}

fn string_array_argument(arguments: &JsonValue, key: &str) -> Vec<String> {
    arguments
        .get(key)
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::to_string)
        .collect()
}

fn parse_runtime_content(content: &str) -> JsonValue {
    serde_json::from_str(content).unwrap_or_else(|_| JsonValue::String(content.to_string()))
}

fn resolve_model_endpoint_env_binding(
    model_endpoint: &ModelEndpointConfig,
) -> Option<(String, String)> {
    if !model_endpoint.api_key.trim().is_empty() {
        return None;
    }
    let env_name = model_endpoint.api_key_env.trim();
    if env_name.is_empty() {
        return None;
    }
    resolve_env_value(env_name).map(|value| (env_name.to_string(), value))
}

fn normalize_optional_runtime_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn resolve_env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| load_os_environment_value(name))
}

#[cfg(not(windows))]
fn load_os_environment_value(_name: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn load_os_environment_value(name: &str) -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    read_windows_environment_value(&current_user, "Environment", name).or_else(|| {
        let local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
        read_windows_environment_value(
            &local_machine,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            name,
        )
    })
}

#[cfg(windows)]
fn read_windows_environment_value(hive: &winreg::RegKey, path: &str, name: &str) -> Option<String> {
    let key = hive.open_subkey(path).ok()?;
    key.get_value::<String, _>(name)
        .ok()
        .map(|value| value.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::{
        format_command_invocation, normalize_command_exec_invocation, resolve_command_timeout_ms,
        web_search_action_args,
    };
    use ennoia_kernel::RuntimeOperationTimeoutConfig;
    use ennoia_logs::RequestContext;

    #[test]
    fn format_command_invocation_normalizes_windows_paths_and_quotes_spaces() {
        assert_eq!(
            format_command_invocation(
                r#"node"#,
                &[
                    r#"C:\Program Files\Ennoia\runner.mjs"#.to_string(),
                    "--watch".to_string()
                ],
            ),
            r#"node "C:/Program Files/Ennoia/runner.mjs" --watch"#
        );
    }

    #[test]
    fn format_command_invocation_keeps_simple_global_commands_readable() {
        assert_eq!(
            format_command_invocation("git", &["status".to_string()]),
            "git status"
        );
    }

    #[test]
    fn normalize_command_exec_invocation_inserts_cmd_mode_flag() {
        let (command, args, normalized) =
            normalize_command_exec_invocation("cmd", &["echo".to_string(), "hi".to_string()]);
        assert_eq!(command, "cmd");
        if cfg!(windows) {
            assert_eq!(
                args,
                vec!["/c".to_string(), "echo".to_string(), "hi".to_string()]
            );
            assert!(normalized);
        } else {
            assert_eq!(args, vec!["echo".to_string(), "hi".to_string()]);
            assert!(!normalized);
        }
    }

    #[test]
    fn command_timeout_respects_tool_requested_value_above_configured_minimum() {
        let config = RuntimeOperationTimeoutConfig {
            default_timeout_ms: 120_000,
            min_timeout_ms: 1_000,
            max_timeout_ms: 3_600_000,
        };

        let timeout_ms =
            resolve_command_timeout_ms(&serde_json::json!({ "timeout_ms": 30_000 }), &config);

        assert_eq!(timeout_ms, 30_000);
    }

    #[test]
    fn web_search_action_args_maps_json_input_to_cli_args() {
        let request = RequestContext {
            request_id: "req-test".to_string(),
            trace_id: "trace-test".to_string(),
            span_id: "span-test".to_string(),
            parent_span_id: None,
            sampled: false,
            source: "test".to_string(),
        };

        let args = web_search_action_args(
            &serde_json::json!({
                "query": "openai responses api",
                "limit": 4,
                "pages": 2,
                "format": "markdown",
            }),
            &request,
        )
        .expect("map args");

        assert_eq!(
            args,
            vec![
                "openai responses api",
                "--limit",
                "4",
                "--pages",
                "2",
                "--format",
                "markdown"
            ]
        );
    }
}
