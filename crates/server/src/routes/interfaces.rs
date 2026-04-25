use ennoia_kernel::{
    HookEventEnvelope, HookResourceRef, InterfaceBindingConfig, InterfaceBindingsConfig, OwnerRef,
    HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};
use std::io::Write;
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
    version: String,
    #[serde(default)]
    schema: Option<String>,
    extension_status: String,
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
                version: item.interface.version,
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
                    attributes: serde_json::json!({ "error": format!("{error:?}") }),
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
    let addressed_agents = payload_string_array_field(payload, &["message", "mentions"]);
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

    let conversation_messages = dispatch_interface_value(
        state,
        request,
        "message.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;
    let run_response = dispatch_interface_value(
        state,
        request,
        "run.create",
        serde_json::json!({
            "owner": payload_owner(payload).unwrap_or_else(|| OwnerRef::global("runtime")),
            "goal": body,
            "trigger": "conversation_message",
            "participants": addressed_agents,
            "addressed_agents": addressed_agents,
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
    )
    .await?;

    for agent_id in &addressed_agents {
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
            Err(error) => format!("{agent_id} 上游调用失败：{error:?}"),
        };
        let append_response = dispatch_interface_value(
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
    let messages = normalize_conversation_messages_for_provider(conversation_messages);
    let request_payload = serde_json::json!({
        "method": "generate",
        "params": {
            "provider": provider_runtime_request_config(provider),
            "model": model_id,
            "system_prompt": build_agent_runtime_prompt(state, agent, &run_id),
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
    let response = invoke_provider_method(&entry, &request_payload, provider)
        .map_err(|error| scoped(ApiError::internal(error), request))?;
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
) -> Vec<JsonValue> {
    let mut messages = conversation_messages
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(24)
        .collect::<Vec<_>>();
    messages.reverse();
    messages
}

fn build_agent_runtime_prompt(state: &AppState, agent: &AgentConfig, run_id: &str) -> String {
    let mut sections = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        sections.push(agent.system_prompt.trim().to_string());
    }
    sections.push(format!(
        "你当前运行在 Ennoia 会话系统中。\nagent_id：{}\nagent_name：{}\nrun_id：{}\nruntime_home：{}\nworking_dir：{}\nartifacts_dir：{}\n请基于以上运行时路径理解用户请求；如果用户提到文件、目录、产物或工作区，优先参考这些路径上下文。直接回答用户，不要伪装成“系统已接收”或“正在处理中”。",
        agent.id,
        agent.display_name,
        if run_id.trim().is_empty() { "unknown" } else { run_id },
        state.runtime_paths.display_for_user(state.runtime_paths.home()),
        agent.working_dir,
        agent.artifacts_dir,
    ));
    sections.join("\n\n")
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
