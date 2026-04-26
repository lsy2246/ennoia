use axum::body::Bytes;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use ennoia_contract::behavior::BehaviorStatusResponse;
use ennoia_extension_host::RegisteredBehaviorContribution;
use ennoia_observability::RequestContext;
use std::time::Instant;

use super::*;
use crate::app::record_trace_span;
use crate::observability::{
    ObservationLogWrite, ObservationSpanWrite, OBSERVABILITY_COMPONENT_BEHAVIOR,
};

#[derive(Debug, Serialize)]
pub(super) struct BehaviorProviderRecord {
    id: String,
    extension_id: String,
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
        OBSERVABILITY_COMPONENT_BEHAVIOR,
    )
    .await
}

pub(super) fn resolve_active_behavior(
    state: &AppState,
    request: &RequestContext,
) -> Result<RegisteredBehaviorContribution, ApiError> {
    let behaviors = state.extensions.snapshot().behaviors;
    match behaviors.as_slice() {
        [only] => Ok(only.clone()),
        [] => {
            let _ = state.observability.append_log(ObservationLogWrite {
                event: "runtime.behavior.resolve_failed".to_string(),
                level: "warn".to_string(),
                component: OBSERVABILITY_COMPONENT_BEHAVIOR.to_string(),
                source_kind: "system".to_string(),
                source_id: None,
                message: "active behavior not found".to_string(),
                attributes: serde_json::json!({ "reason": "no behavior contribution" }),
                created_at: None,
            });
            Err(scoped(ApiError::not_found("active behavior not found"), request))
        }
        _ => Err(scoped(
            ApiError::conflict(
                "multiple behavior implementations found; use interface bindings or explicit extension endpoints",
            ),
            request,
        )),
    }
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
        let _ = state.observability.append_log_scoped(
            ObservationLogWrite {
                event: "runtime.extension.resolve_failed".to_string(),
                level: "warn".to_string(),
                component: component.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.to_string()),
                message: "extension not found for capability request".to_string(),
                attributes: serde_json::json!({ "path": path }),
                created_at: None,
            },
            Some(&request.trace_context()),
        );
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
    let span_trace = request.child_trace("behavior_rpc");
    let started = Instant::now();
    let started_at = now_iso();
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
    };

    let response = state
        .extensions
        .dispatch_rpc(extension_id, &routed_path, rpc_request)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        record_trace_span(
            state,
            ObservationSpanWrite {
                trace: span_trace,
                kind: "behavior_rpc".to_string(),
                name: routed_path.clone(),
                component: component.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.to_string()),
                status: "ok".to_string(),
                attributes: serde_json::json!({
                    "method": method.as_str(),
                    "path": path,
                    "routed_path": routed_path,
                }),
                started_at,
                ended_at: now_iso(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        );
        return Ok(Json(response.data).into_response());
    }

    let error = response
        .error
        .map(|item| format!("{}: {}", item.code, item.message))
        .unwrap_or_else(|| "worker request failed".to_string());
    record_trace_span(
        state,
        ObservationSpanWrite {
            trace: span_trace,
            kind: "behavior_rpc".to_string(),
            name: routed_path.clone(),
            component: component.to_string(),
            source_kind: "extension".to_string(),
            source_id: Some(extension_id.to_string()),
            status: "error".to_string(),
            attributes: serde_json::json!({
                "method": method.as_str(),
                "path": path,
                "routed_path": routed_path,
                "error": error,
            }),
            started_at,
            ended_at: now_iso(),
            duration_ms: started.elapsed().as_millis() as i64,
        },
    );
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
