use ennoia_kernel::{
    CapabilityPermissionMetadata, HookEventEnvelope, HookResourceRef, InterfaceBindingConfig,
    InterfaceBindingsConfig, OwnerRef, PermissionRequest, PermissionScope, PermissionTarget,
    PermissionTrigger, HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;
use std::time::Instant;

use super::*;
use crate::app::record_trace_span;
use crate::event_bus::HookEventWrite;
use crate::observability::{
    ObservationLogWrite, ObservationSpanWrite, OBSERVABILITY_COMPONENT_EVENT_BUS,
    OBSERVABILITY_COMPONENT_PROXY,
};
#[cfg(windows)]
use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
#[cfg(windows)]
use winreg::RegKey;

const AGENT_REPLY_DELAY_MS: u64 = 700;
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

#[derive(Debug, Serialize)]
pub(super) struct InterfaceStatusRecord {
    key: String,
    implementations: Vec<InterfaceImplementationRecord>,
    #[serde(default)]
    active: Option<InterfaceImplementationRecord>,
    status: String,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct InterfaceImplementationRecord {
    extension_id: String,
    method: String,
    #[serde(default)]
    schema: Option<String>,
    extension_status: String,
}

#[derive(Debug, Serialize)]
struct AgentProviderInstructions {
    base: String,
}

#[derive(Debug, Serialize)]
struct AgentProviderContext {
    kind: &'static str,
    runtime: AgentRuntimeContext,
    conversation: AgentConversationContext,
    extensions: Vec<AgentExtensionContext>,
    skills: Vec<AgentSkillContext>,
}

#[derive(Debug, Serialize)]
struct AgentRuntimeContext {
    agent_id: String,
    agent_display_name: String,
    run_id: String,
    runtime_home: String,
    agent_working_dir: String,
    agent_artifacts_dir: String,
}

#[derive(Debug, Serialize)]
struct AgentConversationContext {
    conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentExtensionContext {
    id: String,
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<String>,
    resource_types: Vec<AgentResourceTypeContext>,
    capabilities: Vec<AgentCapabilityContext>,
}

#[derive(Debug, Serialize)]
struct AgentResourceTypeContext {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    content_kind: String,
    operations: Vec<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AgentCapabilityContext {
    id: String,
    contract: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentSkillContext {
    id: String,
    display_name: String,
    description: String,
    entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<String>,
    keywords: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionActorContext {
    agent_id: String,
    kind: String,
    #[serde(default)]
    user_initiated: bool,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
}

pub(super) async fn extension_interfaces(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredInterfaceContribution>> {
    Json(state.extensions.snapshot().interfaces)
}

pub(super) async fn extension_schedule_actions(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredScheduleActionContribution>> {
    Json(state.extensions.snapshot().schedule_actions)
}

pub(super) async fn interfaces_status(
    State(state): State<AppState>,
) -> Json<Vec<InterfaceStatusRecord>> {
    Json(list_interface_status(&state))
}

pub(super) async fn interface_bindings(
    State(state): State<AppState>,
) -> Json<InterfaceBindingsConfig> {
    Json(current_interface_bindings(&state))
}

pub(super) async fn interface_bindings_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<InterfaceBindingsConfig>,
) -> ApiResult<InterfaceBindingsConfig> {
    persist_interface_bindings(&state, &payload)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(payload))
}

pub(super) async fn conversations_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "conversation.list", JsonValue::Null).await
}

pub(super) async fn conversations_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    let response =
        dispatch_interface_value(&state, &request, "conversation.create", payload).await?;
    dispatch_hook_event(
        &state,
        &request,
        HOOK_EVENT_CONVERSATION_CREATED,
        "conversation",
        response
            .get("conversation")
            .and_then(|item| item.get("id"))
            .or_else(|| response.get("id"))
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown"),
        response.clone(),
    );
    Ok(Json(response))
}

pub(super) async fn conversation_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "conversation.get",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    let resource_id = conversation_id.clone();
    let response = dispatch_interface_value(
        &state,
        &request,
        "conversation.delete",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;
    if response
        .get("deleted")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
    {
        dispatch_hook_event(
            &state,
            &request,
            "conversation.deleted",
            "conversation",
            &resource_id,
            response.clone(),
        );
    }
    Ok(Json(response))
}

pub(super) async fn conversation_messages(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "message.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_branches(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "branch.list_by_conversation",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_branches_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "branch.create",
        serde_json::json!({
            "conversation_id": conversation_id,
            "from_branch_id": payload.get("from_branch_id").cloned(),
            "source_message_id": payload.get("source_message_id").cloned(),
            "source_checkpoint_id": payload.get("source_checkpoint_id").cloned(),
            "name": payload.get("name").cloned(),
            "mode": payload.get("mode").cloned(),
            "activate": payload.get("activate").cloned(),
        }),
    )
    .await
}

pub(super) async fn conversation_branch_switch(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((conversation_id, branch_id)): Path<(String, String)>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "branch.switch",
        serde_json::json!({
            "conversation_id": conversation_id,
            "branch_id": branch_id,
        }),
    )
    .await
}

pub(super) async fn conversation_checkpoints(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "checkpoint.list_by_conversation",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_checkpoints_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "checkpoint.create",
        serde_json::json!({
            "conversation_id": conversation_id,
            "branch_id": payload.get("branch_id").cloned(),
            "message_id": payload.get("message_id").cloned(),
            "kind": payload.get("kind").cloned(),
            "label": payload.get("label").cloned(),
        }),
    )
    .await
}

pub(super) async fn conversation_messages_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    let response = dispatch_interface_value(
        &state,
        &request,
        "message.append_user",
        serde_json::json!({
            "conversation_id": conversation_id,
            "message": payload
        }),
    )
    .await?;
    dispatch_hook_event(
        &state,
        &request,
        HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
        "message",
        response
            .get("message")
            .and_then(|item| item.get("id"))
            .or_else(|| response.get("id"))
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown"),
        response.clone(),
    );
    spawn_conversation_agent_reply(state.clone(), request.clone(), response.clone());
    Ok(Json(response))
}

pub(super) async fn conversation_lanes(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "lane.list_by_conversation",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn runs_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "run.create", payload).await
}

pub(super) async fn run_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "run.get",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

pub(super) async fn conversation_runs(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "run.list_by_conversation",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn run_tasks(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "task.list_by_run",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

pub(super) async fn run_artifacts(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "artifact.list_by_run",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

pub(super) async fn dispatch_interface_json(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
) -> ApiResult<JsonValue> {
    dispatch_interface_value(state, request, key, params)
        .await
        .map(Json)
}

pub(super) async fn dispatch_interface_value(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
) -> Result<JsonValue, ApiError> {
    dispatch_interface_value_with_context(state, request, key, params, JsonValue::Null).await
}

pub(super) async fn dispatch_interface_value_with_context(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
    context: JsonValue,
) -> Result<JsonValue, ApiError> {
    let binding = resolve_interface_binding(state, key, request)?;
    let extension = state.extensions.get(&binding.extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{}' not found", binding.extension_id)),
            request,
        )
    })?;
    if extension.worker.is_none() {
        return Err(scoped(
            ApiError::not_found(format!(
                "extension '{}' has no worker",
                binding.extension_id
            )),
            request,
        ));
    }

    let permission_grant_id =
        authorize_interface_dispatch(state, request, key, &binding, &params, &context)?;

    let span_trace = request.child_trace("interface_rpc");
    let started = Instant::now();
    let started_at = now_iso();
    let response = state
        .extensions
        .dispatch_rpc(
            &binding.extension_id,
            &binding.method,
            ennoia_kernel::ExtensionRpcRequest {
                params,
                context: serde_json::json!({
                    "interface": key,
                    "request_id": request.request_id,
                    "trace": {
                        "request_id": span_trace.request_id,
                        "trace_id": span_trace.trace_id,
                        "span_id": span_trace.span_id,
                        "parent_span_id": span_trace.parent_span_id,
                        "sampled": span_trace.sampled,
                        "source": span_trace.source,
                        "traceparent": span_trace.to_traceparent(),
                    },
                    "extra": context
                }),
            },
        )
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        if let Some(grant_id) = permission_grant_id.as_deref() {
            if let Err(error) = state.agent_permissions.consume_grant(grant_id) {
                let _ = state.observability.append_log_scoped(
                    ObservationLogWrite {
                        event: "runtime.permission.consume_grant_failed".to_string(),
                        level: "warn".to_string(),
                        component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                        source_kind: "permission".to_string(),
                        source_id: Some(grant_id.to_string()),
                        message: "permission grant consume failed".to_string(),
                        attributes: serde_json::json!({
                            "interface": key,
                            "extension_id": binding.extension_id,
                            "error": error.to_string(),
                        }),
                        created_at: None,
                    },
                    Some(&span_trace),
                );
            }
        }
        record_trace_span(
            state,
            ObservationSpanWrite {
                trace: span_trace,
                kind: "interface_rpc".to_string(),
                name: key.to_string(),
                component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(binding.extension_id.clone()),
                status: "ok".to_string(),
                attributes: serde_json::json!({
                    "interface": key,
                    "extension_id": binding.extension_id,
                    "method": binding.method,
                }),
                started_at,
                ended_at: now_iso(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        );
        return Ok(response.data);
    }

    let error = response
        .error
        .map(|item| format!("{}: {}", item.code, item.message))
        .unwrap_or_else(|| format!("interface '{key}' failed"));
    record_trace_span(
        state,
        ObservationSpanWrite {
            trace: span_trace,
            kind: "interface_rpc".to_string(),
            name: key.to_string(),
            component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
            source_kind: "extension".to_string(),
            source_id: Some(binding.extension_id.clone()),
            status: "error".to_string(),
            attributes: serde_json::json!({
                "interface": key,
                "extension_id": binding.extension_id,
                "method": binding.method,
                "error": error,
            }),
            started_at,
            ended_at: now_iso(),
            duration_ms: started.elapsed().as_millis() as i64,
        },
    );
    Err(scoped(ApiError::bad_request(error), request))
}

fn authorize_interface_dispatch(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    binding: &InterfaceBindingConfig,
    params: &JsonValue,
    context: &JsonValue,
) -> Result<Option<String>, ApiError> {
    let Some(actor) = permission_actor_from_context(context) else {
        return Ok(None);
    };
    let Some(capability) = find_interface_capability(state, &binding.extension_id, key) else {
        return Ok(None);
    };
    let Some(permission) = capability_permission_metadata(&capability.capability.metadata) else {
        return Ok(None);
    };
    let permission_request =
        build_interface_permission_request(&actor, binding, &capability, &permission, params);
    let decision = state
        .agent_permissions
        .evaluate_request(&permission_request, Some(request))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
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

pub(super) fn resolve_interface_binding(
    state: &AppState,
    key: &str,
    request: &RequestContext,
) -> Result<InterfaceBindingConfig, ApiError> {
    let config = current_interface_bindings(state);
    if let Some(binding) = config.bindings.get(key) {
        return Ok(binding.clone());
    }

    let matches = state
        .extensions
        .snapshot()
        .interfaces
        .into_iter()
        .filter(|item| item.interface.key == key)
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [only] => Ok(InterfaceBindingConfig {
            extension_id: only.extension_id.clone(),
            method: only.interface.method.clone(),
        }),
        [] => {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.interface.missing".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "interface".to_string(),
                    source_id: Some(key.to_string()),
                    message: "interface binding missing".to_string(),
                    attributes: serde_json::json!({ "interface": key }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
            Err(scoped(
                ApiError::not_found(format!("interface '{key}' is not implemented")),
                request,
            ))
        }
        _ => Err(scoped(
            ApiError::conflict(format!("interface '{key}' has multiple implementations")),
            request,
        )),
    }
}

fn list_interface_status(state: &AppState) -> Vec<InterfaceStatusRecord> {
    let config = current_interface_bindings(state);
    let mut by_key = HashMap::<String, Vec<InterfaceImplementationRecord>>::new();
    for item in state.extensions.snapshot().interfaces {
        by_key
            .entry(item.interface.key.clone())
            .or_default()
            .push(InterfaceImplementationRecord {
                extension_id: item.extension_id.clone(),
                method: item.interface.method,
                schema: item.interface.schema,
                extension_status: state
                    .extensions
                    .get(&item.extension_id)
                    .map(|extension| format!("{:?}", extension.health).to_lowercase())
                    .unwrap_or_else(|| "missing".to_string()),
            });
    }

    let mut rows = by_key
        .into_iter()
        .map(|(key, implementations)| {
            let active = config.bindings.get(&key).and_then(|binding| {
                implementations
                    .iter()
                    .find(|item| {
                        item.extension_id == binding.extension_id && item.method == binding.method
                    })
                    .cloned()
            });
            let status = if active.is_some() {
                "bound"
            } else if implementations.len() == 1 {
                "auto"
            } else {
                "conflict"
            }
            .to_string();
            InterfaceStatusRecord {
                key,
                implementations,
                active,
                status,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.key.cmp(&right.key));
    rows
}

fn current_interface_bindings(state: &AppState) -> InterfaceBindingsConfig {
    fs::read_to_string(state.runtime_paths.interfaces_config_file())
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .unwrap_or_default()
}

fn persist_interface_bindings(
    state: &AppState,
    config: &InterfaceBindingsConfig,
) -> std::io::Result<()> {
    if let Some(parent) = state.runtime_paths.interfaces_config_file().parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config).map_err(std::io::Error::other)?;
    fs::write(state.runtime_paths.interfaces_config_file(), contents)
}

fn dispatch_hook_event(
    state: &AppState,
    request: &RequestContext,
    event: &str,
    resource_kind: &str,
    resource_id: &str,
    payload: JsonValue,
) {
    let envelope = HookEventEnvelope {
        event: event.to_string(),
        occurred_at: now_iso(),
        owner: payload_owner(&payload),
        resource: HookResourceRef {
            kind: resource_kind.to_string(),
            id: resource_id.to_string(),
            conversation_id: payload_string_field(&payload, &["conversation_id"])
                .or_else(|| payload_string_field(&payload, &["conversation", "id"]))
                .or_else(|| payload_string_field(&payload, &["message", "conversation_id"])),
            lane_id: payload_string_field(&payload, &["lane_id"])
                .or_else(|| payload_string_field(&payload, &["lane", "id"]))
                .or_else(|| payload_string_field(&payload, &["message", "lane_id"])),
            run_id: payload_string_field(&payload, &["run_id"])
                .or_else(|| payload_string_field(&payload, &["run", "id"])),
            task_id: None,
            artifact_id: None,
        },
        payload,
    };

    let span_trace = request.child_trace("event_publish");
    let started = Instant::now();
    let started_at = now_iso();
    match state.event_bus.publish(HookEventWrite {
        envelope,
        hooks: state.extensions.hooks_for_event(event),
        trace: span_trace.clone(),
    }) {
        Ok(event_id) => {
            record_trace_span(
                state,
                ObservationSpanWrite {
                    trace: span_trace,
                    kind: "event_publish".to_string(),
                    name: event.to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: resource_kind.to_string(),
                    source_id: Some(resource_id.to_string()),
                    status: "ok".to_string(),
                    attributes: serde_json::json!({
                        "event": event,
                        "event_id": event_id,
                    }),
                    started_at,
                    ended_at: now_iso(),
                    duration_ms: started.elapsed().as_millis() as i64,
                },
            );
        }
        Err(error) => {
            record_trace_span(
                state,
                ObservationSpanWrite {
                    trace: span_trace.clone(),
                    kind: "event_publish".to_string(),
                    name: event.to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: resource_kind.to_string(),
                    source_id: Some(resource_id.to_string()),
                    status: "error".to_string(),
                    attributes: serde_json::json!({
                        "event": event,
                        "error": error.to_string(),
                    }),
                    started_at,
                    ended_at: now_iso(),
                    duration_ms: started.elapsed().as_millis() as i64,
                },
            );
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.event_bus.publish_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: "hook".to_string(),
                    source_id: Some(event.to_string()),
                    message: "hook event publish failed".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&span_trace),
            );
        }
    }
}

fn spawn_conversation_agent_reply(state: AppState, request: RequestContext, payload: JsonValue) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(AGENT_REPLY_DELAY_MS)).await;
        if let Err(error) = generate_conversation_agent_reply(&state, &request, &payload).await {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.conversation.agent_reply_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "conversation".to_string(),
                    source_id: payload_string_field(&payload, &["conversation", "id"]).or_else(
                        || payload_string_field(&payload, &["message", "conversation_id"]),
                    ),
                    message: "conversation agent reply generation failed".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
        }
    });
}

async fn generate_conversation_agent_reply(
    state: &AppState,
    request: &RequestContext,
    payload: &JsonValue,
) -> Result<(), ApiError> {
    let role = payload_string_field(payload, &["message", "role"])
        .unwrap_or_else(|| "operator".to_string());
    if role != "operator" {
        return Ok(());
    }

    let conversation_id = payload_string_field(payload, &["conversation", "id"])
        .or_else(|| payload_string_field(payload, &["message", "conversation_id"]))
        .ok_or_else(|| scoped(ApiError::internal("conversation id missing"), request))?;
    let lane_id = payload_string_field(payload, &["lane", "id"])
        .or_else(|| payload_string_field(payload, &["message", "lane_id"]));
    let body = payload_string_field(payload, &["message", "body"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let message_id = payload_string_field(payload, &["message", "id"]);
    let addressed_agents = {
        let explicit = payload_string_array_field(payload, &["addressed_agents"]);
        if explicit.is_empty() {
            payload_string_array_field(payload, &["message", "addressed_agents"])
        } else {
            explicit
        }
    };
    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let agent_runtime_paths = addressed_agents
        .iter()
        .filter_map(|agent_id| {
            agents
                .iter()
                .find(|agent| agent.id == *agent_id)
                .map(|agent| {
                    serde_json::json!({
                        "agent_id": agent.id,
                        "display_name": agent.display_name,
                        "working_dir": agent.working_dir,
                        "artifacts_dir": agent.artifacts_dir,
                    })
                })
        })
        .collect::<Vec<_>>();

    if body.is_empty() || addressed_agents.is_empty() {
        return Ok(());
    }

    for agent_id in &addressed_agents {
        let actor_context = permission_actor_context(
            agent_id,
            "conversation.message.created",
            true,
            Some(&conversation_id),
            None,
        );
        let conversation_messages = dispatch_interface_value_with_context(
            state,
            request,
            "message.list",
            serde_json::json!({ "conversation_id": conversation_id }),
            actor_context.clone(),
        )
        .await?;
        let run_response = dispatch_interface_value_with_context(
            state,
            request,
            "run.create",
            serde_json::json!({
                "owner": payload_owner(payload).unwrap_or_else(|| OwnerRef::global("runtime")),
                "goal": body,
                "trigger": "conversation_message",
                "participants": [agent_id.clone()],
                "addressed_agents": [agent_id.clone()],
                "source_refs": [{
                    "kind": "conversation",
                    "id": conversation_id,
                    "conversation_id": conversation_id,
                    "lane_id": lane_id,
                    "message_id": message_id,
                }],
                "metadata": {
                    "origin": "conversation.message.created",
                    "message_id": message_id,
                    "runtime_home": state.runtime_paths.display_for_user(state.runtime_paths.home()),
                    "agent_paths": agent_runtime_paths,
                }
            }),
            actor_context.clone(),
        )
        .await?;
        let reply_body = match generate_real_conversation_agent_reply(
            state,
            request,
            &agents,
            &providers,
            &conversation_id,
            lane_id.as_deref(),
            message_id.as_deref(),
            &conversation_messages,
            &run_response,
            agent_id,
        )
        .await
        {
            Ok(reply) => reply,
            Err(error) => error.to_string(),
        };
        let run_id = run_response
            .get("run")
            .and_then(|item| item.get("id"))
            .and_then(JsonValue::as_str);
        let append_response = dispatch_interface_value_with_context(
            state,
            request,
            "message.append_agent",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": reply_body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "agent",
                    "addressed_agents": ["operator"],
                }
            }),
            permission_actor_context(
                agent_id,
                "conversation.message.created",
                true,
                Some(&conversation_id),
                run_id,
            ),
        )
        .await?;
        let resource_id = append_response
            .get("message")
            .and_then(|item| item.get("id"))
            .or_else(|| append_response.get("id"))
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown")
            .to_string();

        dispatch_hook_event(
            state,
            request,
            HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
            "message",
            &resource_id,
            append_response,
        );
    }

    Ok(())
}

async fn generate_real_conversation_agent_reply(
    state: &AppState,
    request: &RequestContext,
    agents: &[AgentConfig],
    providers: &[ProviderConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    run_response: &JsonValue,
    agent_id: &str,
) -> Result<String, ApiError> {
    let agent = agents
        .iter()
        .find(|item| item.id == agent_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("agent '{agent_id}' not found")),
                request,
            )
        })?;
    let provider = providers
        .iter()
        .find(|item| item.id == agent.provider_id && item.enabled)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("provider '{}' not found", agent.provider_id)),
                request,
            )
        })?;
    let contribution = resolve_provider_contribution_for_generate(state, provider, request)?;
    let entry = resolve_provider_entry_path(&contribution)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let model_id = if agent.model_id.trim().is_empty() {
        provider.default_model.trim().to_string()
    } else {
        agent.model_id.trim().to_string()
    };
    if model_id.is_empty() {
        return Err(scoped(
            ApiError::bad_request(format!("agent '{}' has no model configured", agent.id)),
            request,
        ));
    }

    let run_id = run_response
        .get("run")
        .and_then(|item| item.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    let messages = normalize_conversation_messages_for_provider(conversation_messages, agent_id);
    let instructions = build_agent_provider_instructions(state, agent, &run_id);
    let context =
        build_agent_provider_context(state, agent, conversation_id, lane_id, message_id, &run_id);
    let request_payload = serde_json::json!({
        "method": "generate",
        "params": {
            "provider": provider_runtime_request_config(provider),
            "model": model_id,
            "instructions": instructions,
            "system_prompt": build_agent_runtime_prompt(state, agent, &run_id),
            "context": context,
            "messages": messages,
            "generation_options": agent.generation_options,
            "metadata": {
                "conversation_id": conversation_id,
                "lane_id": lane_id,
                "message_id": message_id,
                "run_id": run_id,
                "runtime_home": state.runtime_paths.display_for_user(state.runtime_paths.home()),
                "working_dir": agent.working_dir,
                "artifacts_dir": agent.artifacts_dir,
                "agent_id": agent.id,
                "agent_display_name": agent.display_name,
            }
        }
    });
    let provider_grant_id = authorize_provider_generate(
        state,
        request,
        agent,
        provider,
        &contribution,
        conversation_id,
        &run_id,
    )?;
    let response = invoke_provider_method(&entry, &request_payload, provider)
        .map_err(|error| scoped(ApiError::internal(error), request))?;
    if let Some(grant_id) = provider_grant_id.as_deref() {
        state
            .agent_permissions
            .consume_grant(grant_id)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    }
    let text = response
        .get("result")
        .and_then(|item| item.get("text"))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| scoped(ApiError::internal("provider returned empty text"), request))?;
    Ok(text)
}

fn provider_runtime_request_config(provider: &ProviderConfig) -> JsonValue {
    serde_json::json!({
        "id": provider.id,
        "kind": provider.kind,
        "base_url": provider.base_url,
        "api_key_env": provider.api_key_env,
        "default_model": provider.default_model,
    })
}

fn authorize_provider_generate(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    provider: &ProviderConfig,
    contribution: &RegisteredProviderContribution,
    conversation_id: &str,
    run_id: &str,
) -> Result<Option<String>, ApiError> {
    let permission_request = PermissionRequest {
        agent_id: agent.id.clone(),
        action: "provider.generate".to_string(),
        target: PermissionTarget {
            kind: "provider".to_string(),
            id: provider.id.clone(),
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            path: None,
            host: normalize_optional_runtime_value(&provider.base_url),
        },
        scope: PermissionScope {
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            extension_id: Some(contribution.extension_id.clone()),
            path: None,
            host: normalize_optional_runtime_value(&provider.base_url),
        },
        trigger: PermissionTrigger {
            kind: "conversation.message.created".to_string(),
            user_initiated: true,
        },
    };
    let decision = state
        .agent_permissions
        .evaluate_request(&permission_request, Some(request))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    match decision.decision.as_str() {
        "allow" => Ok(decision.grant_id),
        "ask" => Err(scoped(
            ApiError::forbidden(format!(
                "approval required: action=provider.generate, approval_id={}",
                decision
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
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
        )),
        _ => Err(scoped(
            ApiError::forbidden(format!(
                "permission denied: action=provider.generate, reason={}",
                decision.reason
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

fn resolve_provider_contribution_for_generate(
    state: &AppState,
    provider: &ProviderConfig,
    request: &RequestContext,
) -> Result<RegisteredProviderContribution, ApiError> {
    let matches = state
        .extensions
        .snapshot()
        .providers
        .into_iter()
        .filter(|item| item.provider.kind == provider.kind || item.provider.id == provider.kind)
        .filter(|item| {
            item.provider
                .interfaces
                .iter()
                .any(|name| name == "generate")
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(scoped(
            ApiError::not_found(format!(
                "provider kind '{}' has no generate implementation",
                provider.kind
            )),
            request,
        )),
        _ => Err(scoped(
            ApiError::conflict(format!(
                "provider kind '{}' has multiple generate implementations",
                provider.kind
            )),
            request,
        )),
    }
}

fn resolve_provider_entry_path(
    contribution: &RegisteredProviderContribution,
) -> std::io::Result<PathBuf> {
    let entry = contribution
        .provider
        .entry
        .as_deref()
        .ok_or_else(|| std::io::Error::other("provider entry missing"))?;
    let path = PathBuf::from(&contribution.install_dir).join(entry);
    fs::canonicalize(path)
}

fn normalize_conversation_messages_for_provider(
    conversation_messages: &JsonValue,
    agent_id: &str,
) -> Vec<JsonValue> {
    let mut messages = conversation_messages
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|message| message_visible_to_agent(message, agent_id))
        .rev()
        .take(24)
        .collect::<Vec<_>>();
    messages.reverse();
    messages
}

fn message_visible_to_agent(message: &JsonValue, agent_id: &str) -> bool {
    let role = message
        .get("role")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let sender = message
        .get("sender")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    match role {
        "operator" => {
            let mentions = message_mentions(message);
            mentions.is_empty() || mentions.iter().any(|mention| mention == agent_id)
        }
        "agent" => sender == agent_id && !looks_like_synthetic_agent_error(message),
        _ => false,
    }
}

fn message_mentions(message: &JsonValue) -> Vec<String> {
    message
        .get("mentions")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn looks_like_synthetic_agent_error(message: &JsonValue) -> bool {
    let body = message
        .get("body")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    body.starts_with("error:")
        || body.contains("request failed:")
        || body.contains("empty completion")
        || body.contains("provider returned empty text")
}

fn build_agent_runtime_prompt(state: &AppState, agent: &AgentConfig, run_id: &str) -> String {
    let mut sections = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        sections.push(agent.system_prompt.trim().to_string());
    }
    sections.push(format!(
        "你当前运行在 Ennoia 会话系统中。\nagent_id：{}\nagent_name：{}\nrun_id：{}\nruntime_home：{}\nagent_working_dir：{}\nagent_artifacts_dir：{}\n`agent_working_dir` 和 `agent_artifacts_dir` 是当前 Agent 的内部运行目录，不等同于用户项目工作区。只有在用户明确询问路径、文件位置、产物位置，或者任务确实需要读写这些目录时才使用；否则不要主动向用户复述这些内部路径。直接回答用户，不要伪装成“系统已接收”或“正在处理中”。",
        agent.id,
        agent.display_name,
        if run_id.trim().is_empty() { "unknown" } else { run_id },
        state.runtime_paths.display_for_user(state.runtime_paths.home()),
        agent.working_dir,
        agent.artifacts_dir,
    ));
    sections.push(
        "系统会额外提供一份结构化 JSON 上下文，里面包含当前运行时、会话、已注入扩展目录和已启用技能目录。按字段理解并使用，不要向用户原样复述 JSON，也不要主动枚举内部路径、目录清单或所有可用能力，除非用户明确要求。"
            .to_string(),
    );
    sections.join("\n\n")
}

fn build_agent_provider_instructions(
    state: &AppState,
    agent: &AgentConfig,
    run_id: &str,
) -> AgentProviderInstructions {
    AgentProviderInstructions {
        base: build_agent_runtime_prompt(state, agent, run_id),
    }
}

fn build_agent_provider_context(
    state: &AppState,
    agent: &AgentConfig,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    run_id: &str,
) -> AgentProviderContext {
    AgentProviderContext {
        kind: "ennoia.agent_context",
        runtime: AgentRuntimeContext {
            agent_id: agent.id.clone(),
            agent_display_name: agent.display_name.clone(),
            run_id: normalize_unknown(run_id),
            runtime_home: state
                .runtime_paths
                .display_for_user(state.runtime_paths.home()),
            agent_working_dir: agent.working_dir.clone(),
            agent_artifacts_dir: agent.artifacts_dir.clone(),
        },
        conversation: AgentConversationContext {
            conversation_id: conversation_id.to_string(),
            lane_id: lane_id.map(str::to_string),
            message_id: message_id.map(str::to_string),
        },
        extensions: build_agent_extension_contexts(state),
        skills: build_agent_skill_contexts(state, agent),
    }
}

fn build_agent_extension_contexts(state: &AppState) -> Vec<AgentExtensionContext> {
    state
        .extensions
        .snapshot()
        .extensions
        .into_iter()
        .filter(|extension| extension.conversation.inject)
        .map(|extension| {
            let resource_types = extension
                .resource_types
                .iter()
                .filter(|resource_type| {
                    extension.conversation.resource_types.is_empty()
                        || extension
                            .conversation
                            .resource_types
                            .iter()
                            .any(|id| id == &resource_type.id)
                })
                .map(|resource_type| AgentResourceTypeContext {
                    id: resource_type.id.clone(),
                    title: resource_type
                        .title
                        .as_ref()
                        .map(|item| item.fallback.clone()),
                    content_kind: resource_type.content_kind.clone(),
                    operations: resource_type.operations.clone(),
                    tags: resource_type.tags.clone(),
                })
                .collect::<Vec<_>>();
            let capabilities = extension
                .capability_rows
                .iter()
                .filter(|capability| {
                    extension.conversation.capabilities.is_empty()
                        || extension
                            .conversation
                            .capabilities
                            .iter()
                            .any(|id| id == &capability.id)
                })
                .map(|capability| AgentCapabilityContext {
                    id: capability.id.clone(),
                    contract: capability.contract.clone(),
                    kind: capability.kind.clone(),
                    title: capability.title.as_ref().map(|item| item.fallback.clone()),
                })
                .collect::<Vec<_>>();
            AgentExtensionContext {
                id: extension.id.clone(),
                name: extension.name.clone(),
                description: normalize_catalog_text(&extension.description, "无描述"),
                docs: extension
                    .docs
                    .as_deref()
                    .map(|value| resolve_catalog_path(&extension.source_root, value)),
                resource_types,
                capabilities,
            }
        })
        .collect()
}

fn build_agent_skill_contexts(state: &AppState, agent: &AgentConfig) -> Vec<AgentSkillContext> {
    agent
        .skills
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter_map(|skill_id| {
            state
                .skills
                .iter()
                .find(|skill| skill.id == skill_id && skill.enabled)
        })
        .map(|skill| AgentSkillContext {
            id: skill.id.clone(),
            display_name: skill.display_name.clone(),
            description: normalize_catalog_text(&skill.description, "无描述"),
            entry: skill.entry.clone(),
            docs: skill.docs.as_deref().map(|value| {
                resolve_catalog_path(
                    &state
                        .runtime_paths
                        .display_for_user(state.runtime_paths.skill_dir(&skill.id)),
                    value,
                )
            }),
            keywords: skill.keywords.clone(),
        })
        .collect()
}

fn resolve_catalog_path(base: &str, value: &str) -> String {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        return candidate.to_string_lossy().replace('\\', "/");
    }
    PathBuf::from(base)
        .join(value)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_catalog_text(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.replace('\n', " ")
    }
}

fn normalize_unknown(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_optional_runtime_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn permission_actor_context(
    agent_id: &str,
    kind: &str,
    user_initiated: bool,
    conversation_id: Option<&str>,
    run_id: Option<&str>,
) -> JsonValue {
    serde_json::json!({
        "permission_actor": {
            "agent_id": agent_id,
            "kind": kind,
            "user_initiated": user_initiated,
            "conversation_id": conversation_id,
            "run_id": run_id,
        }
    })
}

fn permission_actor_from_context(context: &JsonValue) -> Option<PermissionActorContext> {
    serde_json::from_value::<PermissionActorContext>(context.get("permission_actor")?.clone()).ok()
}

fn find_interface_capability(
    state: &AppState,
    extension_id: &str,
    key: &str,
) -> Option<RegisteredCapabilityContribution> {
    state
        .extensions
        .snapshot()
        .capabilities
        .into_iter()
        .find(|item| {
            item.extension_id == extension_id
                && item
                    .capability
                    .metadata
                    .get("interface")
                    .and_then(|value| value.get("key"))
                    .and_then(JsonValue::as_str)
                    .is_some_and(|interface_key| interface_key == key)
        })
}

fn capability_permission_metadata(metadata: &JsonValue) -> Option<CapabilityPermissionMetadata> {
    serde_json::from_value(metadata.get("permission")?.clone()).ok()
}

fn build_interface_permission_request(
    actor: &PermissionActorContext,
    binding: &InterfaceBindingConfig,
    capability: &RegisteredCapabilityContribution,
    permission: &CapabilityPermissionMetadata,
    params: &JsonValue,
) -> PermissionRequest {
    let conversation_id =
        permission_conversation_id(params).or_else(|| actor.conversation_id.clone());
    let run_id = permission_run_id(params).or_else(|| actor.run_id.clone());
    let path = permission_path(params);
    let host = permission_host(params);
    let target = PermissionTarget {
        kind: permission.target_kind.clone(),
        id: permission_target_id(
            permission,
            capability,
            params,
            conversation_id.as_deref(),
            run_id.as_deref(),
        ),
        conversation_id: conversation_id.clone(),
        run_id: run_id.clone(),
        path: path.clone(),
        host: host.clone(),
    };
    let scope = PermissionScope {
        conversation_id: permission_scope_conversation_id(permission, conversation_id.clone()),
        run_id: permission_scope_run_id(permission, run_id.clone()),
        extension_id: Some(binding.extension_id.clone()),
        path,
        host,
    };
    PermissionRequest {
        agent_id: actor.agent_id.clone(),
        action: permission.action.clone(),
        target,
        scope,
        trigger: PermissionTrigger {
            kind: actor.kind.clone(),
            user_initiated: actor.user_initiated,
        },
    }
}

fn permission_scope_conversation_id(
    permission: &CapabilityPermissionMetadata,
    conversation_id: Option<String>,
) -> Option<String> {
    match permission.scope_kind.trim().to_ascii_lowercase().as_str() {
        "none" => None,
        "run" | "conversation" | "extension" | "" => conversation_id,
        _ => conversation_id,
    }
}

fn permission_scope_run_id(
    permission: &CapabilityPermissionMetadata,
    run_id: Option<String>,
) -> Option<String> {
    match permission.scope_kind.trim().to_ascii_lowercase().as_str() {
        "run" => run_id,
        _ => None,
    }
}

fn permission_target_id(
    permission: &CapabilityPermissionMetadata,
    capability: &RegisteredCapabilityContribution,
    params: &JsonValue,
    conversation_id: Option<&str>,
    run_id: Option<&str>,
) -> String {
    let normalized_kind = permission.target_kind.trim().to_ascii_lowercase();
    let candidate = match normalized_kind.as_str() {
        "conversation" => json_string_at(params, &["conversation_id"])
            .or_else(|| json_string_at(params, &["conversation", "id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "branch" => json_string_at(params, &["branch_id"])
            .or_else(|| json_string_at(params, &["from_branch_id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "checkpoint" => json_string_at(params, &["checkpoint_id"])
            .or_else(|| json_string_at(params, &["source_checkpoint_id"]))
            .or_else(|| json_string_at(params, &["message_id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "run" => json_string_at(params, &["run_id"]).or_else(|| run_id.map(str::to_string)),
        "task" => json_string_at(params, &["task_id"]).or_else(|| run_id.map(str::to_string)),
        "artifact" => {
            json_string_at(params, &["artifact_id"]).or_else(|| run_id.map(str::to_string))
        }
        "memory" => json_string_at(params, &["memory_id"])
            .or_else(|| json_string_at(params, &["workspace_id"]))
            .or_else(|| Some("memory".to_string())),
        _ => None,
    };
    candidate.unwrap_or_else(|| capability.capability.id.clone())
}

fn permission_conversation_id(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["conversation_id"])
        .or_else(|| json_string_at(params, &["conversation", "id"]))
        .or_else(|| json_string_at(params, &["message", "conversation_id"]))
}

fn permission_run_id(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["run_id"]).or_else(|| json_string_at(params, &["run", "id"]))
}

fn permission_path(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["path"])
        .or_else(|| json_string_at(params, &["cwd"]))
        .or_else(|| json_string_at(params, &["file_path"]))
}

fn permission_host(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["host"]).or_else(|| json_string_at(params, &["base_url"]))
}

fn json_string_at(value: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn invoke_provider_method(
    entry: &PathBuf,
    payload: &JsonValue,
    provider: &ProviderConfig,
) -> Result<JsonValue, String> {
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|error| format!("serialize provider request failed: {error}"))?;
    let entry_string = entry
        .to_str()
        .ok_or_else(|| "provider entry path is not valid utf-8".to_string())?
        .to_string();
    let mut command = Command::new("node");
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
    if let Some((name, value)) = resolve_provider_env_binding(provider) {
        command.env(name, value);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn provider runner failed: {error}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(&payload_bytes)
            .map_err(|error| format!("write provider request failed: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("wait provider runner failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("provider runner exited with status {}", output.status)
        } else {
            detail
        });
    }
    serde_json::from_slice::<JsonValue>(&output.stdout)
        .map_err(|error| format!("parse provider response failed: {error}"))
}

fn resolve_provider_env_binding(provider: &ProviderConfig) -> Option<(String, String)> {
    let env_name = provider.api_key_env.trim();
    if env_name.is_empty() {
        return None;
    }
    resolve_env_value(env_name).map(|value| (env_name.to_string(), value))
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
fn read_windows_environment_value(hive: &RegKey, path: &str, name: &str) -> Option<String> {
    let key = hive.open_subkey(path).ok()?;
    key.get_value::<String, _>(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn payload_string_field(payload: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
}

fn payload_string_array_field(payload: &JsonValue, path: &[&str]) -> Vec<String> {
    let mut current = payload;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn payload_owner(payload: &JsonValue) -> Option<OwnerRef> {
    payload
        .get("owner")
        .cloned()
        .or_else(|| {
            payload
                .get("conversation")
                .and_then(|item| item.get("owner"))
                .cloned()
        })
        .and_then(|value| serde_json::from_value(value).ok())
}
