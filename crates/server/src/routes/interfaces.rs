use ennoia_kernel::{
    HookEventEnvelope, HookResourceRef, InterfaceBindingConfig, InterfaceBindingsConfig, OwnerRef,
    HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};

use super::*;
use crate::event_bus::{HookEventWrite, SYSTEM_LOG_COMPONENT_EVENT_BUS};
use crate::system_log::{SystemLogWrite, SYSTEM_LOG_COMPONENT_PROXY};

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
                    "extra": context
                }),
            },
        )
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        return Ok(response.data);
    }

    let error = response
        .error
        .map(|item| format!("{}: {}", item.code, item.message))
        .unwrap_or_else(|| format!("interface '{key}' failed"));
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
            let _ = state.system_log.append(SystemLogWrite {
                event: "runtime.interface.missing".to_string(),
                level: "warn".to_string(),
                component: SYSTEM_LOG_COMPONENT_PROXY.to_string(),
                source_kind: "interface".to_string(),
                source_id: Some(key.to_string()),
                summary: "interface binding missing".to_string(),
                payload: serde_json::json!({ "interface": key }),
                created_at: None,
            });
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

    if let Err(error) = state.event_bus.publish(HookEventWrite {
        envelope,
        hooks: state.extensions.hooks_for_event(event),
    }) {
        let _ = state.system_log.append(SystemLogWrite {
            event: "runtime.event_bus.publish_failed".to_string(),
            level: "warn".to_string(),
            component: SYSTEM_LOG_COMPONENT_EVENT_BUS.to_string(),
            source_kind: "hook".to_string(),
            source_id: Some(event.to_string()),
            summary: "hook event publish failed".to_string(),
            payload: serde_json::json!({ "error": error.to_string() }),
            created_at: None,
        });
    }
}

fn payload_string_field(payload: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
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
