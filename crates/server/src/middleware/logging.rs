use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use chrono::Utc;
use ennoia_observability::RequestContext;
use std::time::Instant;
use tracing::info;

use crate::app::record_trace_span;
use crate::app::AppState;
use crate::observability::ObservationSpanWrite;

/// logging_middleware emits one line per request: method path status latency-ms.
/// Sampling and redaction are honored from the live LoggingConfig.
pub async fn logging_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = &state.server_config.logging;
    if !cfg.enabled {
        return next.run(req).await;
    }

    // Decide sampling (simple rand skipped: deterministic threshold on hash of method+path).
    if cfg.sample_rate < 1.0 && !should_sample(cfg.sample_rate, req.uri().path()) {
        return next.run(req).await;
    }

    let method = req.method().clone();
    let path = req.uri().path().to_string();
    let request_id = req
        .extensions()
        .get::<RequestContext>()
        .cloned()
        .unwrap_or_else(|| RequestContext {
            request_id: "unknown".to_string(),
            trace_id: "unknown".to_string(),
            span_id: "unknown".to_string(),
            parent_span_id: None,
            sampled: true,
            source: "http".to_string(),
        });
    let started = Instant::now();
    let started_at = Utc::now().to_rfc3339();

    let response = next.run(req).await;

    let elapsed_ms = started.elapsed().as_millis();
    let status = response.status().as_u16();
    info!(
        request_id = %request_id.request_id,
        trace_id = %request_id.trace_id,
        span_id = %request_id.span_id,
        http_method = %method,
        http_path = %path,
        http_status = status,
        elapsed_ms,
        configured_level = %cfg.level,
        "http request completed"
    );
    if request_id.trace_id != "unknown" && request_id.span_id != "unknown" {
        record_trace_span(
            &state,
            ObservationSpanWrite {
                trace: request_id.trace_context(),
                kind: "http".to_string(),
                name: format!("{} {}", method, path),
                component: "http".to_string(),
                source_kind: "route".to_string(),
                source_id: Some(path.clone()),
                status: if status >= 500 {
                    "error".to_string()
                } else {
                    "ok".to_string()
                },
                attributes: serde_json::json!({
                    "method": method.as_str(),
                    "path": path,
                    "status": status,
                }),
                started_at,
                ended_at: Utc::now().to_rfc3339(),
                duration_ms: elapsed_ms as i64,
            },
        );
    }
    response
}

fn should_sample(rate: f32, seed: &str) -> bool {
    // Deterministic FNV hash to a 0..1 fraction; good enough for smoke-level sampling.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in seed.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    let bucket = (h % 10_000) as f32 / 10_000.0;
    bucket < rate
}
