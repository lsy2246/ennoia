use super::*;
use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method, Uri};
use ennoia_kernel::HookDispatchResponse;
use tracing::warn;

use crate::system_log::{SystemLogWrite, SYSTEM_LOG_COMPONENT_EXTENSION_HOST};

#[allow(dead_code)]
const HOOK_DISPATCH_ATTEMPTS: usize = 20;
#[allow(dead_code)]
const HOOK_DISPATCH_RETRY_DELAY_MS: u64 = 250;
const EXTENSION_PROXY_ATTEMPTS: usize = 40;
const EXTENSION_PROXY_RETRY_DELAY_MS: u64 = 250;

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

pub(super) async fn extension_frontend_module(
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
    let frontend = extension.frontend.ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' has no frontend entry")),
            &request,
        )
    })?;

    let body = match frontend.kind.as_str() {
        "url" => format!(
            "export {{ default }} from {url:?}; export * from {url:?};",
            url = frontend.entry
        ),
        "file" | "module" => fs::read_to_string(&frontend.entry)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?,
        _ => {
            return Err(scoped(
                ApiError::bad_request(format!("unsupported frontend kind '{}'", frontend.kind)),
                &request,
            ))
        }
    };

    Ok(([(header::CONTENT_TYPE, "application/javascript")], body))
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
    Extension(request): Extension<RequestContext>,
    Path(extension_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let log_path = state
        .runtime_paths
        .extensions_logs_dir()
        .join(format!("{extension_id}.log"));
    let body = if log_path.exists() {
        fs::read_to_string(&log_path)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
    } else {
        String::new()
    };
    Ok(([(header::CONTENT_TYPE, "text/plain; charset=utf-8")], body))
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
    let _ = state.system_log.append(SystemLogWrite {
        event: "runtime.extension.reloaded".to_string(),
        level: "info".to_string(),
        component: SYSTEM_LOG_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        summary: "extension reloaded".to_string(),
        payload: serde_json::json!({}),
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
    let _ = state.system_log.append(SystemLogWrite {
        event: "runtime.extension.restarted".to_string(),
        level: "info".to_string(),
        component: SYSTEM_LOG_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        summary: "extension restarted".to_string(),
        payload: serde_json::json!({}),
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
    let _ = state.system_log.append(SystemLogWrite {
        event: "runtime.extension.attached".to_string(),
        level: "info".to_string(),
        component: SYSTEM_LOG_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(item.id.clone()),
        summary: "extension attached".to_string(),
        payload: serde_json::json!({ "path": payload.path }),
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
    let _ = state.system_log.append(SystemLogWrite {
        event: "runtime.extension.detached".to_string(),
        level: "info".to_string(),
        component: SYSTEM_LOG_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id),
        summary: "extension detached".to_string(),
        payload: serde_json::json!({}),
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
    let _ = state.system_log.append(SystemLogWrite {
        event: if payload.enabled {
            "runtime.extension.enabled".to_string()
        } else {
            "runtime.extension.disabled".to_string()
        },
        level: "info".to_string(),
        component: SYSTEM_LOG_COMPONENT_EXTENSION_HOST.to_string(),
        source_kind: "extension".to_string(),
        source_id: Some(extension_id.clone()),
        summary: "extension enablement changed".to_string(),
        payload: serde_json::json!({ "enabled": payload.enabled }),
        created_at: None,
    });
    Ok(Json(updated))
}

pub(super) async fn extension_api_proxy(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((extension_id, path)): Path<(String, String)>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let extension = state.extensions.get(&extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            &request,
        )
    })?;
    let backend = extension.backend.ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' has no backend entry")),
            &request,
        )
    })?;
    let base_url = backend.base_url.ok_or_else(|| {
        scoped(
            ApiError::bad_request(format!(
                "extension '{extension_id}' backend does not declare a base_url"
            )),
            &request,
        )
    })?;

    let target_url = build_extension_proxy_url(&base_url, &path, uri.query());
    let client = reqwest::Client::new();
    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    let response = send_extension_proxy_request(
        &client,
        reqwest_method,
        &target_url,
        &headers,
        body,
        &request,
    )
    .await?;
    let status = response.status();
    let response_headers = response.headers().clone();
    let response_body = response
        .bytes()
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let mut builder = axum::response::Response::builder().status(status);
    if let Some(content_type) = response_headers.get(reqwest::header::CONTENT_TYPE) {
        if let Ok(value) = content_type.to_str() {
            builder = builder.header("content-type", value);
        }
    }
    if let Some(cache_control) = response_headers.get(reqwest::header::CACHE_CONTROL) {
        if let Ok(value) = cache_control.to_str() {
            builder = builder.header("cache-control", value);
        }
    }
    if let Some(content_disposition) = response_headers.get(reqwest::header::CONTENT_DISPOSITION) {
        if let Ok(value) = content_disposition.to_str() {
            builder = builder.header("content-disposition", value);
        }
    }

    builder
        .body(Body::from(response_body))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(crate) async fn send_extension_proxy_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    target_url: &str,
    headers: &HeaderMap,
    body: Bytes,
    request: &RequestContext,
) -> Result<reqwest::Response, ApiError> {
    let mut last_error = None;
    for attempt in 1..=EXTENSION_PROXY_ATTEMPTS {
        let mut proxied = client.request(method.clone(), target_url);
        for (name, value) in headers {
            if name.as_str().eq_ignore_ascii_case("host")
                || name.as_str().eq_ignore_ascii_case("content-length")
            {
                continue;
            }
            proxied = proxied.header(name, value);
        }
        if !body.is_empty() {
            proxied = proxied.body(body.to_vec());
        }

        match proxied.send().await {
            Ok(response) => return Ok(response),
            Err(error) => {
                last_error = Some(error);
                if attempt < EXTENSION_PROXY_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(EXTENSION_PROXY_RETRY_DELAY_MS)).await;
                }
            }
        }
    }

    Err(scoped(
        ApiError::internal(
            last_error
                .map(|error| error.to_string())
                .unwrap_or_else(|| "extension proxy request failed".to_string()),
        ),
        request,
    ))
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

    let client = reqwest::Client::new();
    let mut outcomes = Vec::new();
    for hook in hooks {
        let Some(extension) = state.extensions.get(&hook.extension_id) else {
            continue;
        };
        let Some(backend) = extension.backend else {
            continue;
        };
        let Some(base_url) = backend.base_url else {
            continue;
        };
        let handler = hook
            .hook
            .handler
            .clone()
            .unwrap_or_else(|| default_hook_handler_path(&hook.hook.event));
        let target_url = build_extension_proxy_url(&base_url, &handler, None);
        let Some(response) = dispatch_hook_request(&client, &target_url, &hook, event).await else {
            continue;
        };
        match response.json::<HookDispatchResponse>().await {
            Ok(payload) => outcomes.push(HookDispatchOutcome {
                extension_id: hook.extension_id,
                response: payload,
            }),
            Err(error) => warn!(
                extension_id = %hook.extension_id,
                event = %event.event,
                error = %error,
                "extension hook response decode failed"
            ),
        }
    }

    outcomes
}

#[allow(dead_code)]
async fn dispatch_hook_request(
    client: &reqwest::Client,
    target_url: &str,
    hook: &RegisteredHookContribution,
    event: &HookEventEnvelope,
) -> Option<reqwest::Response> {
    for attempt in 1..=HOOK_DISPATCH_ATTEMPTS {
        match client.post(target_url).json(event).send().await {
            Ok(response) if response.status().is_success() => return Some(response),
            Ok(response) => {
                if attempt == HOOK_DISPATCH_ATTEMPTS {
                    warn!(
                        extension_id = %hook.extension_id,
                        event = %event.event,
                        status = %response.status(),
                        "extension hook returned non-success status"
                    );
                    return None;
                }
            }
            Err(error) => {
                if attempt == HOOK_DISPATCH_ATTEMPTS {
                    warn!(
                        extension_id = %hook.extension_id,
                        event = %event.event,
                        error = %error,
                        "extension hook dispatch failed"
                    );
                    return None;
                }
            }
        }

        tokio::time::sleep(Duration::from_millis(HOOK_DISPATCH_RETRY_DELAY_MS)).await;
    }

    None
}

pub(crate) fn build_extension_proxy_url(base_url: &str, path: &str, query: Option<&str>) -> String {
    let mut url = format!(
        "{}/{}",
        base_url.trim_end_matches('/'),
        path.trim_start_matches('/')
    );
    if let Some(query) = query.filter(|value| !value.is_empty()) {
        url.push('?');
        url.push_str(query);
    }
    url
}

#[allow(dead_code)]
fn default_hook_handler_path(event: &str) -> String {
    format!("/hooks/{}", event.replace('.', "/"))
}
