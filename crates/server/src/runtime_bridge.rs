use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use ennoia_contract::ApiError;
use ennoia_error_utils::normalize_error_message;
use ennoia_extension_host::RegisteredProviderContribution;
use ennoia_kernel::{
    AgentConfig, ModelEndpointConfig, OperationPerformRequest, OperationPerformResponse,
    OperationRecord, OperationStatus, PermissionRequest, PermissionScope, PermissionTarget,
    PermissionTrigger, RuntimeOperationRequest, RuntimeOperationTimeoutConfig,
};
use ennoia_logs::RequestContext;
use serde::Serialize;
use serde_json::Value as JsonValue;
use tokio::io::AsyncWriteExt;

use crate::app::{live_server_config, AppState};
use crate::execution::{
    execute_native_operation, resolve_agent_tool_path, resolve_command_cwd, AgentExecutionPaths,
    SandboxOperation,
};
use crate::logs_store::{LogEntryWrite, LOGS_COMPONENT_HOST};
use crate::realtime::RealtimeEvent;
use crate::routes::actions::dispatch_hook_event;
use crate::routes::extensions::invoke_provider_json_with_request;
use crate::routes::scoped;

const MIN_COMMAND_TIMEOUT_MS: u64 = 120_000;

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
    let agent = crate::app::load_agent_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?
        .into_iter()
        .find(|agent| agent.id == payload.agent_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("agent '{}' not found", payload.agent_id)),
                request,
            )
        })?;
    let content = match operation {
        "fs.read" => execute_fs_read(state, request, &agent, &payload).await?,
        "fs.write" => execute_fs_write(state, request, &agent, &payload).await?,
        "command.exec" => execute_command_exec(state, request, &agent, &payload).await?,
        "net.fetch" => execute_net_fetch(state, request, &agent, &payload).await?,
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
        ("runtime", "fs.read")
        | ("runtime", "fs.write")
        | ("runtime", "command.exec")
        | ("runtime", "net.fetch") => {
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

fn resolve_runtime_timeout_ms(
    arguments: &JsonValue,
    config: &RuntimeOperationTimeoutConfig,
) -> u64 {
    integer_argument(arguments, "timeout_ms")
        .unwrap_or(config.default_timeout_ms as i64)
        .clamp(
            config.default_timeout_ms as i64,
            config.max_timeout_ms as i64,
        ) as u64
}

fn resolve_command_timeout_ms(
    arguments: &JsonValue,
    config: &RuntimeOperationTimeoutConfig,
) -> u64 {
    integer_argument(arguments, "timeout_ms")
        .unwrap_or(config.default_timeout_ms as i64)
        .clamp(
            config.default_timeout_ms.max(MIN_COMMAND_TIMEOUT_MS) as i64,
            config.max_timeout_ms.max(MIN_COMMAND_TIMEOUT_MS) as i64,
        ) as u64
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

async fn execute_fs_read(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let path = required_string_argument(&payload.arguments, "path", request)?;
    let max_bytes = integer_argument(&payload.arguments, "max_bytes")
        .unwrap_or(32_768)
        .clamp(256, 262_144) as usize;
    let execution_paths = AgentExecutionPaths::for_agent(state, agent, &payload.run_id);
    let resolved_path =
        resolve_agent_tool_path(&agent.execution_environment, &execution_paths, &path)
            .map_err(|error| scoped(error, request))?;
    let grant_id = authorize_runtime_operation(
        state,
        request,
        &agent.id,
        "fs.read",
        "file",
        &resolved_path.display_path,
        &payload.conversation_id,
        &payload.run_id,
        payload.message_id.as_deref(),
        Some(&resolved_path.display_path),
        None,
        "runtime.operation",
    )?;
    consume_runtime_grant(state, request, grant_id)?;
    let content = if agent.execution_environment.sandbox_enabled {
        execute_native_operation(
            agent,
            &execution_paths,
            false,
            SandboxOperation::FsRead {
                host_path: resolved_path.host_path.to_string_lossy().to_string(),
                display_path: resolved_path.display_path.clone(),
                max_bytes,
            },
        )
        .await
        .map_err(|error| scoped(error, request))?
    } else {
        let bytes = fs::read(&resolved_path.host_path).map_err(|error| {
            scoped(
                ApiError::internal(format!("read file failed: {error}")),
                request,
            )
        })?;
        let truncated = bytes.len() > max_bytes;
        let visible = if truncated {
            &bytes[..max_bytes]
        } else {
            &bytes[..]
        };
        serde_json::json!({
            "ok": true,
            "tool": "fs.read",
            "path": resolved_path.display_path,
            "bytes_read": visible.len(),
            "truncated": truncated,
            "content": String::from_utf8_lossy(visible).to_string(),
        })
        .to_string()
    };
    Ok(parse_runtime_content(&content))
}

async fn execute_fs_write(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let path = required_string_argument(&payload.arguments, "path", request)?;
    let content = required_string_argument(&payload.arguments, "content", request)?;
    let append = boolean_argument(&payload.arguments, "append").unwrap_or(false);
    let execution_paths = AgentExecutionPaths::for_agent(state, agent, &payload.run_id);
    let resolved_path =
        resolve_agent_tool_path(&agent.execution_environment, &execution_paths, &path)
            .map_err(|error| scoped(error, request))?;
    let grant_id = authorize_runtime_operation(
        state,
        request,
        &agent.id,
        "fs.write",
        "file",
        &resolved_path.display_path,
        &payload.conversation_id,
        &payload.run_id,
        payload.message_id.as_deref(),
        Some(&resolved_path.display_path),
        None,
        "runtime.operation",
    )?;
    consume_runtime_grant(state, request, grant_id)?;
    let response = if agent.execution_environment.sandbox_enabled {
        execute_native_operation(
            agent,
            &execution_paths,
            false,
            SandboxOperation::FsWrite {
                host_path: resolved_path.host_path.to_string_lossy().to_string(),
                display_path: resolved_path.display_path.clone(),
                content,
                append,
            },
        )
        .await
        .map_err(|error| scoped(error, request))?
    } else {
        if let Some(parent) = resolved_path.host_path.parent() {
            fs::create_dir_all(parent).map_err(|error| {
                scoped(
                    ApiError::internal(format!("create parent dir failed: {error}")),
                    request,
                )
            })?;
        }
        if append {
            use std::io::Write as _;
            let mut file = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&resolved_path.host_path)
                .map_err(|error| {
                    scoped(
                        ApiError::internal(format!("open file for append failed: {error}")),
                        request,
                    )
                })?;
            file.write_all(content.as_bytes()).map_err(|error| {
                scoped(
                    ApiError::internal(format!("append file failed: {error}")),
                    request,
                )
            })?;
        } else {
            fs::write(&resolved_path.host_path, content.as_bytes()).map_err(|error| {
                scoped(
                    ApiError::internal(format!("write file failed: {error}")),
                    request,
                )
            })?;
        }
        serde_json::json!({
            "ok": true,
            "tool": "fs.write",
            "path": resolved_path.display_path,
            "bytes_written": content.len(),
            "append": append,
        })
        .to_string()
    };
    Ok(parse_runtime_content(&response))
}

async fn execute_command_exec(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let server_config = live_server_config(state);
    let command_name = required_string_argument(&payload.arguments, "command", request)?;
    let requested_args = string_array_argument(&payload.arguments, "args");
    let (effective_command_name, effective_args, invocation_normalized) =
        normalize_command_exec_invocation(&command_name, &requested_args);
    let cwd_value = string_argument(&payload.arguments, "cwd");
    let execution_paths = AgentExecutionPaths::for_agent(state, agent, &payload.run_id);
    let cwd = resolve_command_cwd(
        &agent.execution_environment,
        &execution_paths,
        cwd_value.as_deref(),
    )
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
        &agent.id,
        "command.exec",
        "command",
        &command_name,
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
        "cwd": cwd.display_path,
        "timeout_ms": timeout_ms,
        "sandbox_enabled": agent.execution_environment.sandbox_enabled,
        "invocation_normalized": invocation_normalized,
        "agent_id": agent.id,
        "conversation_id": payload.conversation_id,
        "run_id": payload.run_id,
        "message_id": payload.message_id,
    });
    let content = if agent.execution_environment.sandbox_enabled {
        execute_native_operation(
            agent,
            &execution_paths,
            true,
            SandboxOperation::CommandExec {
                command: effective_command_name.clone(),
                args: effective_args.clone(),
                cwd_host_path: cwd.host_path.to_string_lossy().to_string(),
                cwd_display_path: cwd.display_path.clone(),
                timeout_ms,
            },
        )
        .await
        .map_err(|error| {
            let message = error.message().to_string();
            let api_error = if message.contains("timed out") {
                ApiError::timeout(message)
            } else {
                ApiError::internal(message)
            };
            scoped(api_error.with_details(error_details.clone()), request)
        })?
    } else {
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
        serde_json::json!({
            "ok": output.status.success(),
            "tool": "command.exec",
            "command": effective_command_name,
            "args": effective_args,
            "cwd": cwd.display_path,
            "status": output.status.code(),
            "stdout": String::from_utf8_lossy(&output.stdout).to_string(),
            "stderr": String::from_utf8_lossy(&output.stderr).to_string(),
        })
        .to_string()
    };
    Ok(parse_runtime_content(&content))
}

async fn execute_net_fetch(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    payload: &RuntimeOperationRequest,
) -> Result<JsonValue, ApiError> {
    let server_config = live_server_config(state);
    let url = required_string_argument(&payload.arguments, "url", request)?;
    let method = string_argument(&payload.arguments, "method").unwrap_or_else(|| "GET".to_string());
    let body = string_argument(&payload.arguments, "body");
    let operation_config = &server_config.operations.net;
    let timeout_ms = resolve_runtime_timeout_ms(&payload.arguments, operation_config);
    let host = reqwest::Url::parse(&url)
        .ok()
        .and_then(|parsed| parsed.host_str().map(str::to_string));
    let grant_id = authorize_runtime_operation(
        state,
        request,
        &agent.id,
        "net.fetch",
        "network",
        &url,
        &payload.conversation_id,
        &payload.run_id,
        payload.message_id.as_deref(),
        None,
        host.as_deref(),
        "runtime.operation",
    )?;
    consume_runtime_grant(state, request, grant_id)?;
    let execution_paths = AgentExecutionPaths::for_agent(state, agent, &payload.run_id);
    let content = if agent.execution_environment.sandbox_enabled {
        let mut headers = std::collections::BTreeMap::new();
        if let Some(raw_headers) = payload
            .arguments
            .get("headers")
            .and_then(JsonValue::as_object)
        {
            for (key, value) in raw_headers {
                if let Some(value) = value.as_str() {
                    headers.insert(key.clone(), value.to_string());
                }
            }
        }
        execute_native_operation(
            agent,
            &execution_paths,
            true,
            SandboxOperation::NetFetch {
                url: url.clone(),
                method: method.clone(),
                headers,
                body: body.clone(),
                timeout_ms,
            },
        )
        .await
        .map_err(|error| scoped(error, request))?
    } else {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .map_err(|error| {
                scoped(
                    ApiError::internal(format!("build http client failed: {error}")),
                    request,
                )
            })?;
        let mut request_builder = client.request(
            reqwest::Method::from_bytes(method.as_bytes()).map_err(|error| {
                scoped(
                    ApiError::bad_request(format!("invalid http method: {error}")),
                    request,
                )
            })?,
            &url,
        );
        if let Some(headers) = payload
            .arguments
            .get("headers")
            .and_then(JsonValue::as_object)
        {
            for (key, value) in headers {
                if let Some(value) = value.as_str() {
                    request_builder = request_builder.header(key, value);
                }
            }
        }
        if let Some(body) = body {
            request_builder = request_builder.body(body);
        }
        let response = request_builder.send().await.map_err(|error| {
            scoped(
                ApiError::internal(format!("http request failed: {error}")),
                request,
            )
        })?;
        let status = response.status().as_u16();
        let headers = response
            .headers()
            .iter()
            .map(|(key, value)| {
                (
                    key.as_str().to_string(),
                    JsonValue::String(value.to_str().unwrap_or_default().to_string()),
                )
            })
            .collect::<serde_json::Map<String, JsonValue>>();
        let text = response.text().await.map_err(|error| {
            scoped(
                ApiError::internal(format!("read http response failed: {error}")),
                request,
            )
        })?;
        serde_json::json!({
            "ok": true,
            "tool": "net.fetch",
            "url": url,
            "status": status,
            "headers": headers,
            "body": text,
        })
        .to_string()
    };
    Ok(parse_runtime_content(&content))
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

fn boolean_argument(arguments: &JsonValue, key: &str) -> Option<bool> {
    arguments.get(key).and_then(JsonValue::as_bool)
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
