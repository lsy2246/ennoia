use std::time::Duration;

use axum::{
    extract::{Request, State},
    http::header,
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
    let cfg = &state.server_config.timeout;
    if !cfg.enabled || is_event_stream_request(&req) {
        return next.run(req).await;
    }
    let request_id = req.extensions().get::<RequestContext>().cloned();

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
            .map(|id| {
                ApiError::timeout("request timed out")
                    .with_request_id(&id.request_id)
                    .with_trace_id(&id.trace_id)
            })
            .unwrap_or_else(|| ApiError::timeout("request timed out"))
            .into_response(),
    }
}

fn is_event_stream_request(req: &Request) -> bool {
    req.headers()
        .get(header::ACCEPT)
        .and_then(|value| value.to_str().ok())
        .map(|value| {
            value
                .split(',')
                .any(|item| item.trim().eq_ignore_ascii_case("text/event-stream"))
        })
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;

    use super::is_event_stream_request;

    #[test]
    fn detects_sse_accept_header() {
        let request = Request::builder()
            .uri("/api/logs/entries/stream")
            .header("accept", "text/event-stream")
            .body(Body::empty())
            .expect("request");

        assert!(is_event_stream_request(&request));
    }

    #[test]
    fn ignores_normal_json_request() {
        let request = Request::builder()
            .uri("/api/logs/entries")
            .header("accept", "application/json")
            .body(Body::empty())
            .expect("request");

        assert!(!is_event_stream_request(&request));
    }
}
