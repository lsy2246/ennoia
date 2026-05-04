use std::fs;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;

use ennoia_contract::ApiError;
use ennoia_error_utils::normalize_error_message;
use ennoia_extension_host::RegisteredProviderContribution;
use ennoia_kernel::{
    AgentConfig, ModelEndpointConfig, PermissionRequest, PermissionScope, PermissionTarget,
    PermissionTrigger,
};
use ennoia_logs::RequestContext;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::io::AsyncWriteExt;

use crate::app::{live_server_config, AppState};
use crate::execution::{
    execute_native_operation, resolve_agent_tool_path, resolve_command_cwd, AgentExecutionPaths,
    SandboxOperation,
};
use crate::realtime::RealtimeEvent;
use crate::routes::scoped;

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

#[derive(Debug, Clone, Deserialize)]
pub struct RuntimeOperationRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub run_id: String,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub arguments: JsonValue,
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
    let args = string_array_argument(&payload.arguments, "args");
    let cwd_value = string_argument(&payload.arguments, "cwd");
    let execution_paths = AgentExecutionPaths::for_agent(state, agent, &payload.run_id);
    let cwd = resolve_command_cwd(
        &agent.execution_environment,
        &execution_paths,
        cwd_value.as_deref(),
    )
    .map_err(|error| scoped(error, request))?;
    let operation_config = &server_config.operations.command;
    let timeout_ms = integer_argument(&payload.arguments, "timeout_ms")
        .unwrap_or(operation_config.default_timeout_ms as i64)
        .clamp(
            operation_config.min_timeout_ms as i64,
            operation_config.max_timeout_ms as i64,
        ) as u64;
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
    let content = if agent.execution_environment.sandbox_enabled {
        execute_native_operation(
            agent,
            &execution_paths,
            true,
            SandboxOperation::CommandExec {
                command: command_name.clone(),
                args: args.clone(),
                cwd_host_path: cwd.host_path.to_string_lossy().to_string(),
                cwd_display_path: cwd.display_path.clone(),
                timeout_ms,
            },
        )
        .await
        .map_err(|error| scoped(error, request))?
    } else {
        let mut command = tokio::process::Command::new(&command_name);
        command.args(&args).current_dir(cwd.host_path.clone());
        let output = tokio::time::timeout(Duration::from_millis(timeout_ms), command.output())
            .await
            .map_err(|_| scoped(ApiError::internal("command exec timed out"), request))?
            .map_err(|error| {
                scoped(
                    ApiError::internal(format!("spawn command failed: {error}")),
                    request,
                )
            })?;
        serde_json::json!({
            "ok": output.status.success(),
            "tool": "command.exec",
            "command": command_name,
            "args": args,
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
    let timeout_ms = integer_argument(&payload.arguments, "timeout_ms")
        .unwrap_or(operation_config.default_timeout_ms as i64)
        .clamp(
            operation_config.min_timeout_ms as i64,
            operation_config.max_timeout_ms as i64,
        ) as u64;
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
