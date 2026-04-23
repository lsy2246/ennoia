use std::fs;

use axum::body::Bytes;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use ennoia_contract::behavior::BehaviorStatusResponse;
use ennoia_extension_host::RegisteredBehaviorContribution;
use ennoia_observability::RequestContext;

use super::*;
use crate::system_log::{SystemLogWrite, SYSTEM_LOG_COMPONENT_BEHAVIOR};

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
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let resolved = resolve_active_behavior(&state, &request)?;
    let extension_id = resolved.extension_id.clone();
    dispatch_worker_capability_request(
        &state,
        &request,
        &extension_id,
        resolved.behavior.entry.as_deref(),
        &path,
        method,
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

pub(super) async fn dispatch_worker_capability_request(
    state: &AppState,
    request: &RequestContext,
    extension_id: &str,
    entry: Option<&str>,
    path: &str,
    method: Method,
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
            summary: "extension not found for capability request".to_string(),
            payload: serde_json::json!({ "path": path }),
            created_at: None,
        });
        scoped(
            ApiError::not_found(format!("extension '{extension_id}' not found")),
            request,
        )
    })?;

    if extension.worker.is_none() {
        return Err(scoped(
            ApiError::not_found(format!("extension '{extension_id}' has no worker entry")),
            request,
        ));
    }

    let routed_path = join_entry_path(entry, path);
    let params = if body.is_empty() {
        JsonValue::Null
    } else {
        serde_json::from_slice(&body).unwrap_or_else(
            |_| serde_json::json!({ "raw": String::from_utf8_lossy(&body).to_string() }),
        )
    };
    let rpc_request = ennoia_kernel::ExtensionRpcRequest {
        params,
        context: serde_json::json!({
            "component": component,
            "method": method.as_str(),
            "path": path,
            "routed_path": routed_path,
        }),
    };

    let response = state
        .extensions
        .dispatch_rpc(extension_id, &routed_path, rpc_request)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        return Ok(Json(response.data).into_response());
    }

    let error = response
        .error
        .map(|item| format!("{}: {}", item.code, item.message))
        .unwrap_or_else(|| "worker request failed".to_string());
    Err(scoped(ApiError::bad_request(error), request))
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
