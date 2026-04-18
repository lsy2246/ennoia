use std::time::Duration;

use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};

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

    let path = req.uri().path().to_string();
    let ms = cfg
        .per_path_ms
        .get(&path)
        .copied()
        .unwrap_or(cfg.default_ms);
    let duration = Duration::from_millis(ms);

    match tokio::time::timeout(duration, next.run(req)).await {
        Ok(response) => response,
        Err(_) => (StatusCode::REQUEST_TIMEOUT, "request timed out").into_response(),
    }
}
