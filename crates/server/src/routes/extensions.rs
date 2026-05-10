use super::*;
use ennoia_kernel::{
    ExtensionRecordEntry, ExtensionRpcRequest, ExtensionRpcResponse, ExtensionSettingFieldType,
    ExtensionSettingValue, ExtensionStateEntry, HookDispatchResponse, ModelEndpointConfig,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use crate::app::{dispatch_extension_rpc, record_trace_span};
use crate::extension_runtime::{
    ExtensionRecordAppend, ExtensionRecordListQuery, ExtensionRecordUpdate, ExtensionStateGetQuery,
    ExtensionStateListQuery, ExtensionStatePut,
};
use crate::logs_store::{LogEntryWrite, LogTraceWrite, LOGS_COMPONENT_EXTENSION_HOST};
use crate::routes::resources::validate_model_endpoint_request_timeout_ms;
use crate::runtime_bridge::{
    authorize_provider_generate, invoke_provider_method, resolve_provider_entry_path,
};

#[allow(dead_code)]
const HOOK_DISPATCH_ATTEMPTS: usize = 20;
#[allow(dead_code)]
const HOOK_DISPATCH_RETRY_DELAY_MS: u64 = 250;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub(crate) struct HookDispatchOutcome {
    pub extension_id: String,
    pub response: HookDispatchResponse,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ExtensionSettingsFile {
    #[serde(default)]
    values: BTreeMap<String, ExtensionSettingValue>,
}

#[derive(Debug, Clone, Deserialize)]
struct ProviderPermissionActorContext {
    agent_id: String,
    kind: String,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    message_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ExtensionSettingsRecord {
    extension_id: String,
    values: BTreeMap<String, ExtensionSettingValue>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ExtensionSettingsPayload {
    #[serde(default)]
    values: BTreeMap<String, ExtensionSettingValue>,
}

pub(super) async fn extensions(
    State(state): State<AppState>,
) -> Json<Vec<ExtensionWorkbenchRecord>> {
    Json(list_extension_workbench_records(&state))
}

pub(super) async fn extensions_runtime(
    State(state): State<AppState>,
) -> Json<ExtensionRuntimeSnapshot> {
    Json(state.extensions.snapshot())
}

pub(super) async fn extension_pages(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredPageContribution>> {
    Json(state.extensions.snapshot().pages)
}

pub(super) async fn extension_panels(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredPanelContribution>> {
    Json(state.extensions.snapshot().panels)
}

pub(super) async fn extension_commands(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredCommandContribution>> {
    Json(state.extensions.snapshot().commands)
}

pub(super) async fn extension_providers(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredProviderContribution>> {
    Json(state.extensions.snapshot().providers)
}

pub(super) async fn extension_hooks(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredHookContribution>> {
    Json(state.extensions.snapshot().hooks)
}

pub(super) async fn extension_events(
    State(state): State<AppState>,
    Query(query): Query<ExtensionEventsQuery>,
) -> Json<Vec<ExtensionRuntimeEvent>> {
    Json(state.extensions.events(query.limit.unwrap_or(50)))
}

pub(super) async fn extension_events_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut receiver = state.realtime.subscribe();
    let extensions = state.extensions.clone();
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("extension.graph_swapped").data(extension_graph_swapped_payload(&extensions)));
        loop {
            match receiver.recv().await {
                Ok(crate::realtime::RealtimeEvent::ExtensionsChanged) => {
                    yield Ok(Event::default().event("extension.graph_swapped").data(extension_graph_swapped_payload(&extensions)));
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("extension.graph_swapped").data(extension_graph_swapped_payload(&extensions)));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn extension_graph_swapped_payload(extensions: &ennoia_extension_host::ExtensionRuntime) -> String {
    let snapshot = extensions.snapshot();
    serde_json::json!({
        "generation": snapshot.generation,
        "updated_at": snapshot.updated_at,
        "extensions": snapshot.extensions.len(),
    })
    .to_string()
}

pub(super) async fn extension_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> ApiResult<ResolvedExtensionSnapshot> {
    state
        .extensions
        .get(&extension_id)
        .map(Json)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("extension '{extension_id}' not found")),
                &request,
            )
        })
}

pub(super) async fn extension_settings(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> ApiResult<ExtensionSettingsRecord> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    Ok(Json(ExtensionSettingsRecord {
        extension_id,
        values: load_effective_extension_settings(&state, &extension),
    }))
}

pub(super) async fn extension_settings_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
    Json(payload): Json<ExtensionSettingsPayload>,
) -> ApiResult<ExtensionSettingsRecord> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    validate_extension_settings_payload(&extension, &payload)
        .map_err(|error| scoped(ApiError::bad_request(error), &request))?;

    let mut stored = read_extension_settings_file(extension_settings_path(&state, &extension_id))
        .unwrap_or_default()
        .values;
    for (key, value) in payload.values {
        stored.insert(key, value);
    }
    write_extension_settings_file(extension_settings_path(&state, &extension_id), &stored)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    Ok(Json(ExtensionSettingsRecord {
        extension_id,
        values: load_effective_extension_settings(&state, &extension),
    }))
}

pub(super) async fn extension_diagnostics(
    State(state): State<AppState>,
    Path(extension_id): Path<String>,
) -> Json<Vec<ExtensionDiagnostic>> {
    Json(state.extensions.diagnostics(&extension_id))
}

pub(super) async fn extension_state_get(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ExtensionStateGetQuery>,
) -> ApiResult<ExtensionStateEntry> {
    state
        .extension_runtime_store
        .get_state(&query)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("extension state not found"), &request))
}

pub(super) async fn extension_state_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ExtensionStatePut>,
) -> ApiResult<ExtensionStateEntry> {
    state
        .extension_runtime_store
        .put_state(&payload)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn extension_state_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ExtensionStateGetQuery>,
) -> ApiResult<JsonValue> {
    let deleted = state
        .extension_runtime_store
        .delete_state(&query)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(serde_json::json!({ "deleted": deleted })))
}

pub(super) async fn extension_state_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ExtensionStateListQuery>,
) -> ApiResult<Vec<ExtensionStateEntry>> {
    state
        .extension_runtime_store
        .list_state(&query)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn extension_record_append(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ExtensionRecordAppend>,
) -> ApiResult<ExtensionRecordEntry> {
    state
        .extension_runtime_store
        .append_record(&payload)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn extension_record_update(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ExtensionRecordUpdate>,
) -> ApiResult<ExtensionRecordEntry> {
    state
        .extension_runtime_store
        .update_record(&payload)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("extension record not found"), &request))
}

pub(super) async fn extension_record_close(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(record_id): Path<String>,
) -> ApiResult<ExtensionRecordEntry> {
    state
        .extension_runtime_store
        .close_record(&record_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("extension record not found"), &request))
}

pub(super) async fn extension_record_get(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(record_id): Path<String>,
) -> ApiResult<ExtensionRecordEntry> {
    state
        .extension_runtime_store
        .get_record(&record_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("extension record not found"), &request))
}

pub(super) async fn extension_record_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ExtensionRecordListQuery>,
) -> ApiResult<Vec<ExtensionRecordEntry>> {
    state
        .extension_runtime_store
        .list_records(&query)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

fn extension_settings_path(state: &AppState, extension_id: &str) -> PathBuf {
    state
        .runtime_paths
        .extension_state_dir(extension_id)
        .join("settings.toml")
}

fn read_extension_settings_file(path: PathBuf) -> Option<ExtensionSettingsFile> {
    let contents = fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}

fn write_extension_settings_file(
    path: PathBuf,
    values: &BTreeMap<String, ExtensionSettingValue>,
) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let payload = ExtensionSettingsFile {
        values: values.clone(),
    };
    fs::write(
        path,
        toml::to_string_pretty(&payload).map_err(std::io::Error::other)?,
    )
}

fn default_extension_settings(
    extension: &ResolvedExtensionSnapshot,
) -> BTreeMap<String, ExtensionSettingValue> {
    extension
        .settings
        .iter()
        .filter_map(|item| {
            item.default_value
                .clone()
                .map(|value| (item.key.clone(), value))
        })
        .collect()
}

fn load_effective_extension_settings(
    state: &AppState,
    extension: &ResolvedExtensionSnapshot,
) -> BTreeMap<String, ExtensionSettingValue> {
    let stored = read_extension_settings_file(extension_settings_path(state, &extension.id))
        .unwrap_or_default()
        .values;
    let mut values = default_extension_settings(extension);
    for field in &extension.settings {
        if let Some(value) = stored.get(&field.key).cloned() {
            values.insert(field.key.clone(), value);
        }
    }
    values
}

fn validate_extension_settings_payload(
    extension: &ResolvedExtensionSnapshot,
    payload: &ExtensionSettingsPayload,
) -> Result<(), String> {
    let declared = extension
        .settings
        .iter()
        .map(|item| (item.key.as_str(), item))
        .collect::<BTreeMap<_, _>>();
    let declared_keys = declared.keys().copied().collect::<BTreeSet<_>>();

    for key in payload.values.keys() {
        if !declared_keys.contains(key.as_str()) {
            return Err(format!(
                "setting '{key}' is not declared by extension '{}'",
                extension.id
            ));
        }
    }

    for field in &extension.settings {
        if field.required
            && !payload.values.contains_key(&field.key)
            && field.default_value.is_none()
        {
            return Err(format!("required setting '{}' is missing", field.key));
        }
    }

    for (key, value) in &payload.values {
        let Some(field) = declared.get(key.as_str()) else {
            continue;
        };
        match (&field.field_type, value) {
            (ExtensionSettingFieldType::Boolean, ExtensionSettingValue::Boolean(_)) => {}
            (ExtensionSettingFieldType::Number, ExtensionSettingValue::Integer(_)) => {}
            (
                ExtensionSettingFieldType::Text
                | ExtensionSettingFieldType::Textarea
                | ExtensionSettingFieldType::Select,
                ExtensionSettingValue::String(text),
            ) => {
                if field.required && text.trim().is_empty() {
                    return Err(format!("setting '{}' cannot be empty", field.key));
                }
                if matches!(field.field_type, ExtensionSettingFieldType::Select)
                    && !field.options.is_empty()
                    && !field.options.iter().any(|option| option.value == *text)
                {
                    return Err(format!(
                        "setting '{}' has unsupported value '{}'",
                        field.key, text
                    ));
                }
            }
            _ => {
                return Err(format!(
                    "setting '{}' does not match declared type '{:?}'",
                    field.key, field.field_type
                ));
            }
        }
    }

    Ok(())
}

pub(super) async fn extension_ui_module(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    let ui = extension.ui.ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' has no ui entry")),
            &request,
        )
    })?;

    let body = match ui.kind.as_str() {
        "url" => format!(
            "export {{ default }} from {url:?}; export * from {url:?};",
            url = ui.entry
        ),
        "file" | "module" => {
            let source_root = PathBuf::from(&extension.source_root);
            let entry_path = PathBuf::from(&ui.entry);
            let public_path = extension_asset_relative_path(&source_root, &entry_path)
                .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
            let import_url = format!(
                "/api/extensions/{}/ui/assets/{}?v={}",
                extension_id,
                encode_asset_url_path(&public_path),
                encode_url_query_component(&ui.version),
            );
            format!(
                "export {{ default }} from {url:?}; export * from {url:?};",
                url = import_url
            )
        }
        _ => {
            return Err(scoped(
                ApiError::bad_request(format!("unsupported ui kind '{}'", ui.kind)),
                &request,
            ))
        }
    };

    Ok((
        [
            (
                header::CONTENT_TYPE,
                "application/javascript; charset=utf-8",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        body,
    ))
}

pub(super) async fn extension_ui_asset(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((extension_id, asset_path)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    let source_root = PathBuf::from(&extension.source_root);
    let asset = resolve_safe_extension_asset(&source_root, &asset_path)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    let body = fs::read(asset.clone())
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let content_type = mime_guess::from_path(asset)
        .first_or_octet_stream()
        .to_string();

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache".to_string()),
        ],
        body,
    ))
}

pub(super) async fn extension_theme_stylesheet(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((extension_id, theme_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    let theme = extension
        .themes
        .iter()
        .find(|item| item.id == theme_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!(
                    "theme '{theme_id}' not found in extension '{extension_id}'"
                )),
                &request,
            )
        })?;
    let source_root = PathBuf::from(&extension.source_root);
    let stylesheet_path = resolve_safe_extension_asset(&source_root, &theme.tokens_entry)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    let body = fs::read_to_string(stylesheet_path)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    Ok(([(header::CONTENT_TYPE, "text/css; charset=utf-8")], body))
}

pub(super) async fn extension_logs(
    State(state): State<AppState>,
    Path(extension_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let diagnostics = state.extensions.diagnostics(&extension_id);
    let body = diagnostics
        .into_iter()
        .map(|item| {
            format!(
                "{} [{}] {}{}",
                item.at,
                item.level,
                item.summary,
                item.detail
                    .map(|detail| format!(": {detail}"))
                    .unwrap_or_default()
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body))
}

pub(super) async fn extension_rpc(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((extension_id, method)): Path<(String, String)>,
    Json(payload): Json<ExtensionRpcRequest>,
) -> ApiResult<ExtensionRpcResponse> {
    let span_trace = request.child_trace("extension_rpc");
    let started = Instant::now();
    let started_at = now_iso();
    let ExtensionRpcRequest { params, context } = payload;
    dispatch_extension_rpc(
        &state,
        &extension_id,
        &method,
        ExtensionRpcRequest {
            params,
            context: serde_json::json!({
                "upstream": context,
                "trace": {
                    "request_id": span_trace.request_id.clone(),
                    "trace_id": span_trace.trace_id.clone(),
                    "span_id": span_trace.span_id.clone(),
                    "parent_span_id": span_trace.parent_span_id.clone(),
                    "sampled": span_trace.sampled,
                    "source": span_trace.source.clone(),
                    "traceparent": span_trace.to_traceparent(),
                }
            }),
        },
    )
    .await
    .map(|response| {
        record_trace_span(
            &state,
            LogTraceWrite {
                trace: span_trace.clone(),
                kind: "extension_rpc".to_string(),
                name: method.clone(),
                component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.clone()),
                status: if response.ok {
                    "ok".to_string()
                } else {
                    "error".to_string()
                },
                attributes: serde_json::json!({
                    "extension_id": extension_id,
                    "method": method,
                }),
                started_at: started_at.clone(),
                ended_at: now_iso(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        );
        Json(response)
    })
    .map_err(|error| {
        record_trace_span(
            &state,
            LogTraceWrite {
                trace: span_trace,
                kind: "extension_rpc".to_string(),
                name: method.clone(),
                component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.clone()),
                status: "error".to_string(),
                attributes: serde_json::json!({
                    "extension_id": extension_id,
                    "method": method,
                    "error": error.to_string(),
                }),
                started_at,
                ended_at: now_iso(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        );
        scoped(ApiError::internal(error.to_string()), &request)
    })
}

pub(super) async fn extension_provider_invoke(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((provider_kind, method)): Path<(String, String)>,
    Json(payload): Json<ExtensionRpcRequest>,
) -> ApiResult<JsonValue> {
    invoke_provider_json_with_request(&state, &request, &provider_kind, &method, payload)
        .await
        .map(Json)
}

pub(crate) async fn invoke_provider_json_with_request(
    state: &AppState,
    request: &RequestContext,
    provider_kind: &str,
    method: &str,
    payload: ExtensionRpcRequest,
) -> Result<JsonValue, ApiError> {
    let contribution = resolve_provider_contribution(&state, &provider_kind, &method).ok_or_else(
        || {
            scoped(
                ApiError::not_found(format!(
                    "provider kind '{provider_kind}' has no runtime implementation for method '{method}'",
                )),
                &request,
            )
        },
    )?;
    let model_endpoint: ModelEndpointConfig = serde_json::from_value(
        payload
            .params
            .get("model_endpoint")
            .cloned()
            .unwrap_or(JsonValue::Null),
    )
    .map_err(|error| {
        scoped(
            ApiError::bad_request(format!(
                "provider invoke requires params.model_endpoint: {error}"
            ))
            .with_details(serde_json::json!({
                "source": "system",
                "reason": "model_endpoint_payload_invalid",
                "provider_kind": provider_kind,
                "method": method,
            })),
            &request,
        )
    })?;
    validate_model_endpoint_request_timeout_ms(model_endpoint.request_timeout_ms)
        .map_err(|error| scoped(error, &request))?;
    let permission_actor = provider_permission_actor_from_context(&payload.context);
    let grant_id = if method == "generate" {
        permission_actor
            .as_ref()
            .and_then(|actor| {
                Some(authorize_provider_generate(
                    &state,
                    &request,
                    &actor.agent_id,
                    &contribution,
                    &model_endpoint,
                    actor.conversation_id.as_deref()?,
                    actor.run_id.as_deref()?,
                    actor.message_id.as_deref(),
                    &actor.kind,
                ))
            })
            .transpose()?
            .flatten()
    } else {
        None
    };
    if let Some(grant_id) = grant_id.as_deref() {
        state
            .agent_permissions
            .consume_grant(grant_id)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    }
    let entry = resolve_provider_entry_path(&contribution)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let response = invoke_provider_method(
        &entry,
        &serde_json::json!({
            "method": method,
            "params": payload.params,
        }),
        &model_endpoint,
    )
    .await
    .map_err(|error| {
        let _ = state.logs.append_log_scoped(
            LogEntryWrite {
                event: "runtime.provider.invoke_failed".to_string(),
                level: "error".to_string(),
                component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
                source_kind: "provider".to_string(),
                source_id: Some(provider_kind.to_string()),
                message: "provider runtime invoke failed".to_string(),
                attributes: serde_json::json!({
                    "method": method,
                    "error": error,
                    "model_endpoint_id": model_endpoint.id,
                }),
                created_at: None,
            },
            Some(&request.trace_context()),
        );
        scoped(
            ApiError::internal(error).with_details(serde_json::json!({
                "source": "provider",
                "provider_kind": provider_kind,
                "method": method,
                "model_endpoint_id": model_endpoint.id,
            })),
            &request,
        )
    })?;

    Ok(response.get("result").cloned().unwrap_or(response))
}

fn provider_permission_actor_from_context(
    context: &JsonValue,
) -> Option<ProviderPermissionActorContext> {
    serde_json::from_value::<ProviderPermissionActorContext>(
        context.get("permission_actor")?.clone(),
    )
    .ok()
}
pub(super) async fn extension_reload(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> ApiResult<ResolvedExtensionSnapshot> {
    let item = state
        .extensions
        .reload_extension(&extension_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("extension '{extension_id}' not found")),
                &request,
            )
        })?;
    let _ = state.logs.append_log(LogEntryWrite {
        event: "runtime.extension.reloaded".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension reloaded".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
    state
        .realtime
        .publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
    Ok(Json(item))
}

pub(super) async fn extension_restart(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> ApiResult<ResolvedExtensionSnapshot> {
    let item = state
        .extensions
        .restart_extension(&extension_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("extension '{extension_id}' not found")),
                &request,
            )
        })?;
    let _ = state.logs.append_log(LogEntryWrite {
        event: "runtime.extension.restarted".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension restarted".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
    state
        .realtime
        .publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
    Ok(Json(item))
}

pub(super) async fn extension_attach(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ExtensionAttachPayload>,
) -> ApiResult<ResolvedExtensionSnapshot> {
    let item = state
        .extensions
        .attach_dev_source(&payload.path)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    let _ = state.logs.append_log(LogEntryWrite {
        event: "runtime.extension.attached".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(item.id.clone()),
        message: "extension attached".to_string(),
        attributes: serde_json::json!({ "path": payload.path }),
        created_at: None,
    });
    state
        .realtime
        .publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
    Ok(Json(item))
}

pub(super) async fn extension_detach(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let detached = state
        .extensions
        .detach_dev_source(&extension_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    if !detached {
        return Err(scoped(
            ApiError::not_found(format!("extension '{extension_id}' not attached")),
            &request,
        ));
    }
    let _ = state.logs.append_log(LogEntryWrite {
        event: "runtime.extension.detached".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension detached".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
    state
        .realtime
        .publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
    Ok(StatusCode::NO_CONTENT)
}

pub(super) async fn extension_enabled_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
    Json(payload): Json<ExtensionEnabledPayload>,
) -> ApiResult<ExtensionWorkbenchRecord> {
    let existing_records = list_extension_workbench_records(&state);
    let existing = existing_records
        .into_iter()
        .find(|item| item.id == extension_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("extension '{extension_id}' not found")),
                &request,
            )
        })?;

    state
        .extensions
        .set_extension_enabled(&extension_id, payload.enabled)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let updated = list_extension_workbench_records(&state)
        .into_iter()
        .find(|item| item.id == extension_id)
        .unwrap_or(ExtensionWorkbenchRecord {
            enabled: payload.enabled,
            status: if payload.enabled {
                "ready".to_string()
            } else {
                "disabled".to_string()
            },
            ..existing
        });
    let _ = state.logs.append_log(LogEntryWrite {
        event: if payload.enabled {
            "runtime.extension.enabled".to_string()
        } else {
            "runtime.extension.disabled".to_string()
        },
        level: "info".to_string(),
        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id.clone()),
        message: "extension enablement changed".to_string(),
        attributes: serde_json::json!({ "enabled": payload.enabled }),
        created_at: None,
    });
    state
        .realtime
        .publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
    Ok(Json(updated))
}

#[allow(dead_code)]
pub(crate) async fn dispatch_extension_hooks(
    state: &AppState,
    event: &HookEventEnvelope,
) -> Vec<HookDispatchOutcome> {
    let hooks = state.extensions.hooks_for_event(&event.event);
    if hooks.is_empty() {
        return Vec::new();
    }

    let mut outcomes = Vec::new();
    for hook in hooks {
        let handler = hook
            .hook
            .handler
            .clone()
            .unwrap_or_else(|| default_hook_handler_path(&hook.hook.event));
        let request = ExtensionRpcRequest {
            params: serde_json::to_value(event).unwrap_or(JsonValue::Null),
            context: serde_json::json!({
                "event": hook.hook.event,
                "handler": handler,
            }),
        };
        let Ok(response) =
            dispatch_extension_rpc(state, &hook.extension_id, &handler, request).await
        else {
            continue;
        };
        if !response.ok {
            continue;
        }
        if let Ok(payload) = serde_json::from_value::<HookDispatchResponse>(response.data) {
            outcomes.push(HookDispatchOutcome {
                extension_id: hook.extension_id,
                response: payload,
            });
        }
    }

    outcomes
}

#[allow(dead_code)]
fn default_hook_handler_path(event: &str) -> String {
    format!("/hooks/{}", event.replace('.', "/"))
}

fn extension_asset_relative_path(root: &StdPath, path: &StdPath) -> std::io::Result<String> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_asset = fs::canonicalize(path)?;
    if !canonical_asset.starts_with(&canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "extension ui entry must stay inside the extension root",
        ));
    }
    let relative = canonical_asset
        .strip_prefix(canonical_root)
        .map_err(std::io::Error::other)?;
    Ok(relative.to_string_lossy().replace('\\', "/"))
}

fn encode_asset_url_path(path: &str) -> String {
    path.split('/')
        .map(encode_url_query_component)
        .collect::<Vec<_>>()
        .join("/")
}

fn encode_url_query_component(value: &str) -> String {
    value
        .bytes()
        .flat_map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                vec![byte as char]
            }
            _ => format!("%{byte:02X}").chars().collect::<Vec<_>>(),
        })
        .collect()
}

fn resolve_provider_contribution(
    state: &AppState,
    provider_kind: &str,
    method: &str,
) -> Option<ennoia_extension_host::RegisteredProviderContribution> {
    let required_interface = match method.trim() {
        "list_models" => Some("models"),
        "generate" => Some("generate"),
        _ => None,
    };
    let normalized_kind = provider_kind.trim();
    let mut matches = state
        .extensions
        .snapshot()
        .providers
        .into_iter()
        .filter(|item| item.provider.kind == normalized_kind || item.provider.id == normalized_kind)
        .filter(|item| {
            required_interface.is_none_or(|required| {
                item.provider
                    .interfaces
                    .iter()
                    .any(|entry| entry == required)
            })
        })
        .collect::<Vec<_>>();
    if matches.len() == 1 {
        return matches.pop();
    }
    None
}
