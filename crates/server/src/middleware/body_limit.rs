use axum::{
    body::{to_bytes, Body},
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::app::AppState;

/// body_limit_middleware consumes the body up to `max_bytes` and rejects oversize payloads.
/// Per-path overrides win over the global default.
pub async fn body_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.body_limit.load();
    if !cfg.enabled {
        return next.run(req).await;
    }

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
            return (
                StatusCode::PAYLOAD_TOO_LARGE,
                format!("body exceeds {limit} bytes"),
            )
                .into_response()
        }
    };
    let rebuilt = Request::from_parts(parts, Body::from(bytes));
    next.run(rebuilt).await
}
