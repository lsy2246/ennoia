use super::*;
use ennoia_kernel::{ExtensionRpcRequest, ExtensionRpcResponse, HookDispatchResponse};
use std::time::Instant;

use crate::app::record_trace_span;
use crate::observability::{
    ObservationLogWrite, ObservationSpanWrite, OBSERVABILITY_COMPONENT_EXTENSION_HOST,
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
    let extensions = state.extensions.clone();
    let stream = async_stream::stream! {
        let mut last_generation = extensions.snapshot().generation;
        loop {
            let snapshot = extensions.snapshot();
            if snapshot.generation > last_generation {
                last_generation = snapshot.generation;
                let payload = serde_json::json!({
                    "generation": snapshot.generation,
                    "updated_at": snapshot.updated_at,
                    "extensions": snapshot.extensions.len(),
                });
                yield Ok(Event::default().event("extension.graph_swapped").data(payload.to_string()));
            }
            tokio::time::sleep(Duration::from_secs(1)).await;
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
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

pub(super) async fn extension_diagnostics(
    State(state): State<AppState>,
    Path(extension_id): Path<String>,
) -> Json<Vec<ExtensionDiagnostic>> {
    Json(state.extensions.diagnostics(&extension_id))
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
    state
        .extensions
        .dispatch_rpc(
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
        .map(|response| {
            record_trace_span(
                &state,
                ObservationSpanWrite {
                    trace: span_trace.clone(),
                    kind: "extension_rpc".to_string(),
                    name: method.clone(),
                    component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
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
                ObservationSpanWrite {
                    trace: span_trace,
                    kind: "extension_rpc".to_string(),
                    name: method.clone(),
                    component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
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
    let _ = state.observability.append_log(ObservationLogWrite {
        event: "runtime.extension.reloaded".to_string(),
        level: "info".to_string(),
        component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension reloaded".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
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
    let _ = state.observability.append_log(ObservationLogWrite {
        event: "runtime.extension.restarted".to_string(),
        level: "info".to_string(),
        component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension restarted".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
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
    let _ = state.observability.append_log(ObservationLogWrite {
        event: "runtime.extension.attached".to_string(),
        level: "info".to_string(),
        component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(item.id.clone()),
        message: "extension attached".to_string(),
        attributes: serde_json::json!({ "path": payload.path }),
        created_at: None,
    });
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
    let _ = state.observability.append_log(ObservationLogWrite {
        event: "runtime.extension.detached".to_string(),
        level: "info".to_string(),
        component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        message: "extension detached".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
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
    let _ = state.observability.append_log(ObservationLogWrite {
        event: if payload.enabled {
            "runtime.extension.enabled".to_string()
        } else {
            "runtime.extension.disabled".to_string()
        },
        level: "info".to_string(),
        component: OBSERVABILITY_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id.clone()),
        message: "extension enablement changed".to_string(),
        attributes: serde_json::json!({ "enabled": payload.enabled }),
        created_at: None,
    });
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
        let Ok(response) = state
            .extensions
            .dispatch_rpc(&hook.extension_id, &handler, request)
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
