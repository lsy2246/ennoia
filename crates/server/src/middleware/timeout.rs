use std::time::Duration;

use axum::{
    extract::{Request, State},
    middleware::Next,
    response::{IntoResponse, Response},
};
use ennoia_contract::ApiError;
use ennoia_observability::RequestContext;

use crate::app::AppState;

/// timeout_middleware enforces a per-path or default-ms timeout from the live TimeoutConfig.
pub async fn timeout_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.timeout.load();
    if !cfg.enabled {
        return next.run(req).await;
    }
    let request_id = req
        .extensions()
        .get::<RequestContext>()
        .map(|ctx| ctx.request_id.clone());

    let path = req.uri().path().to_string();
    let ms = cfg
        .per_path_ms
        .get(&path)
        .copied()
        .unwrap_or(cfg.default_ms);
    let duration = Duration::from_millis(ms);

    match tokio::time::timeout(duration, next.run(req)).await {
        Ok(response) => response,
        Err(_) => request_id
            .as_ref()
            .map(|id| ApiError::timeout("request timed out").with_request_id(id))
            .unwrap_or_else(|| ApiError::timeout("request timed out"))
            .into_response(),
    }
}
