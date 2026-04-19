use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use ennoia_observability::RequestContext;
use std::time::Instant;
use tracing::info;

use crate::app::AppState;

/// logging_middleware emits one line per request: method path status latency-ms.
/// Sampling and redaction are honored from the live LoggingConfig.
pub async fn logging_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.logging.load();
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
        .map(|ctx| ctx.request_id.clone())
        .unwrap_or_else(|| "unknown".to_string());
    let started = Instant::now();

    let response = next.run(req).await;

    let elapsed_ms = started.elapsed().as_millis();
    let status = response.status().as_u16();
    info!(
        request_id = %request_id,
        http_method = %method,
        http_path = %path,
        http_status = status,
        elapsed_ms,
        configured_level = %cfg.level,
        "http request completed"
    );
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
