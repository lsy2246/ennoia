use std::fs;

use axum::body::{Body, Bytes};
use axum::http::{HeaderMap, Method, Uri};
use axum::response::{IntoResponse, Response};
use ennoia_contract::behavior::BehaviorStatusResponse;
use ennoia_extension_host::RegisteredBehaviorContribution;
use ennoia_observability::RequestContext;

use super::*;
use crate::system_log::{
    SystemLogWrite, SYSTEM_LOG_COMPONENT_BEHAVIOR, SYSTEM_LOG_COMPONENT_PROXY,
};

#[derive(Debug, Serialize)]
pub(super) struct BehaviorProviderRecord {
    id: String,
    extension_id: String,
    version: String,
    interfaces: Vec<String>,
    entry: Option<String>,
    extension_status: String,
}

pub(super) async fn extension_behaviors(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredBehaviorContribution>> {
    Json(state.extensions.snapshot().behaviors)
}

pub(super) async fn behaviors(State(state): State<AppState>) -> Json<Vec<BehaviorProviderRecord>> {
    Json(
        state
            .extensions
            .snapshot()
            .behaviors
            .into_iter()
            .map(|item| {
                let extension_id = item.extension_id.clone();
                BehaviorProviderRecord {
                    id: item.behavior.id,
                    extension_id: extension_id.clone(),
                    version: item.behavior.version,
                    interfaces: item.behavior.interfaces,
                    entry: item.behavior.entry,
                    extension_status: state
                        .extensions
                        .get(&extension_id)
                        .map(|extension| extension.health)
                        .map(|health| format!("{health:?}").to_lowercase())
                        .unwrap_or_else(|| "missing".to_string()),
                }
            })
            .collect(),
    )
}

pub(super) async fn active_behavior(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<BehaviorProviderRecord> {
    let resolved = resolve_active_behavior(&state, &request)?;
    Ok(Json(BehaviorProviderRecord {
        id: resolved.behavior.id.clone(),
        extension_id: resolved.extension_id.clone(),
        version: resolved.behavior.version.clone(),
        interfaces: resolved.behavior.interfaces.clone(),
        entry: resolved.behavior.entry.clone(),
        extension_status: state
            .extensions
            .get(&resolved.extension_id)
            .map(|extension| extension.health)
            .map(|health| format!("{health:?}").to_lowercase())
            .unwrap_or_else(|| "missing".to_string()),
    }))
}

pub(super) async fn behavior_status(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<BehaviorStatusResponse> {
    let resolved = resolve_active_behavior(&state, &request)?;
    let extension = state
        .extensions
        .get(&resolved.extension_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found("active behavior extension not found"),
                &request,
            )
        })?;

    Ok(Json(BehaviorStatusResponse {
        extension_id: resolved.extension_id,
        behavior_id: resolved.behavior.id,
        healthy: matches!(extension.health, ennoia_kernel::ExtensionHealth::Ready),
        version: resolved.behavior.version,
        interfaces: resolved.behavior.interfaces,
    }))
}

pub(super) async fn behavior_api_proxy(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(path): Path<String>,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let resolved = resolve_active_behavior(&state, &request)?;
    let extension_id = resolved.extension_id.clone();
    proxy_capability_request(
        &state,
        &request,
        &extension_id,
        resolved.behavior.entry.as_deref(),
        &path,
        method,
        headers,
        uri,
        body,
        SYSTEM_LOG_COMPONENT_BEHAVIOR,
    )
    .await
}

pub(super) fn resolve_active_behavior(
    state: &AppState,
    request: &RequestContext,
) -> Result<RegisteredBehaviorContribution, ApiError> {
    let config = current_behavior_config(state);
    let behavior = state
        .extensions
        .snapshot()
        .behaviors
        .into_iter()
        .find(|item| {
            item.extension_id == config.active_extension
                && item.behavior.id == config.active_behavior
        })
        .ok_or_else(|| {
            let _ = state.system_log.append(SystemLogWrite {
                event: "runtime.behavior.resolve_failed".to_string(),
                level: "warn".to_string(),
                component: SYSTEM_LOG_COMPONENT_BEHAVIOR.to_string(),
                source_kind: "system".to_string(),
                source_id: Some(config.active_extension.clone()),
                summary: "active behavior not found".to_string(),
                payload: serde_json::json!({
                    "active_extension": config.active_extension,
                    "active_behavior": config.active_behavior,
                }),
                created_at: None,
            });
            scoped(ApiError::not_found("active behavior not found"), request)
        })?;
    Ok(behavior)
}

pub(super) async fn proxy_capability_request(
    state: &AppState,
    request: &RequestContext,
    extension_id: &str,
    entry: Option<&str>,
    path: &str,
    method: Method,
    headers: HeaderMap,
    uri: Uri,
    body: Bytes,
    component: &str,
) -> Result<Response, ApiError> {
    let extension = state.extensions.get(extension_id).ok_or_else(|| {
        let _ = state.system_log.append(SystemLogWrite {
            event: "runtime.extension.resolve_failed".to_string(),
            level: "warn".to_string(),
            component: component.to_string(),
            source_kind: "extension".to_string(),
            source_id: Some(extension_id.to_string()),
            summary: "extension not found for capability proxy".to_string(),
            payload: serde_json::json!({ "path": path }),
            created_at: None,
        });
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            request,
        )
    })?;
    let backend = extension.backend.ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' has no backend entry")),
            request,
        )
    })?;
    let base_url = backend.base_url.ok_or_else(|| {
        scoped(
            ApiError::bad_request(format!(
                "extension '{extension_id}' backend does not declare a base_url"
            )),
            request,
        )
    })?;

    let routed_path = join_entry_path(entry, path);
    let target_url = build_extension_proxy_url(&base_url, &routed_path, uri.query());
    let client = reqwest::Client::new();
    let reqwest_method = reqwest::Method::from_bytes(method.as_str().as_bytes())
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), request))?;
    let response = match send_extension_proxy_request(
        &client,
        reqwest_method,
        &target_url,
        &headers,
        body,
        request,
    )
    .await
    {
        Ok(response) => response,
        Err(error) => {
            let _ = state.system_log.append(SystemLogWrite {
                event: "runtime.extension.proxy.failed".to_string(),
                level: "warn".to_string(),
                component: SYSTEM_LOG_COMPONENT_PROXY.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.to_string()),
                summary: "capability proxy failed".to_string(),
                payload: serde_json::json!({
                    "path": routed_path,
                    "component": component,
                }),
                created_at: None,
            });
            return Err(error);
        }
    };
    let status = response.status();
    let response_headers = response.headers().clone();
    let response_body = response
        .bytes()
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

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
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))
}

fn join_entry_path(entry: Option<&str>, path: &str) -> String {
    let normalized_entry = entry
        .map(|value| value.trim_matches('/'))
        .filter(|value| !value.is_empty());
    let normalized_path = path.trim_matches('/');
    match (normalized_entry, normalized_path.is_empty()) {
        (Some(prefix), false) => format!("{prefix}/{normalized_path}"),
        (Some(prefix), true) => prefix.to_string(),
        (None, false) => normalized_path.to_string(),
        (None, true) => String::new(),
    }
}

fn current_behavior_config(state: &AppState) -> ennoia_kernel::BehaviorConfig {
    fs::read_to_string(state.runtime_paths.behavior_config_file())
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .unwrap_or_else(|| state.behavior_config.clone())
}
