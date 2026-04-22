use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ennoia_contract::ApiError;
use ennoia_observability::RequestContext;

use crate::app::AppState;

/// body_limit_middleware consumes the body up to `max_bytes` and rejects oversize payloads.
/// Per-path overrides win over the global default.
pub async fn body_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = &state.server_config.body_limit;
    if !cfg.enabled {
        return next.run(req).await;
    }
    let request_id = req
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone());

    let path = req.uri().path().to_string();
    let limit = cfg
        .per_path_max
        .get(&path)
        .copied()
        .unwrap_or(cfg.max_bytes);

    let (parts, body) = req.into_parts();
    let bytes = match to_bytes(body, limit).await {
        Ok(bytes) => bytes,
        Err(_) => {
            let error = request_id
                .as_ref()
                .map(|id| {
                    ApiError::payload_too_large(format!("body exceeds {limit} bytes"))
                        .with_request_id(id)
                })
                .unwrap_or_else(|| {
                    ApiError::payload_too_large(format!("body exceeds {limit} bytes"))
                });
            return error.into_response();
        }
    };
    let rebuilt = Request::from_parts(parts, Body::from(bytes));
    next.run(rebuilt).await
}
