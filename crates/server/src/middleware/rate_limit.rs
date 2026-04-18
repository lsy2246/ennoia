use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

use crate::app::AppState;
use crate::middleware::path_matches;

/// RateLimitState is a per-process, per-IP fixed-window counter.
#[derive(Clone, Default)]
pub struct RateLimitState {
    windows: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

impl RateLimitState {
    pub fn new() -> Self {
        Self::default()
    }

    fn check(&self, key: &str, limit_rpm: u32) -> bool {
        if limit_rpm == 0 {
            return true;
        }
        let mut windows = self.windows.lock().unwrap();
        let now = Instant::now();
        let window_size = Duration::from_secs(60);

        let entry = windows
            .entry(key.to_string())
            .or_insert_with(|| (0, now));

        if now.duration_since(entry.1) > window_size {
            *entry = (1, now);
            return true;
        }

        if entry.0 >= limit_rpm {
            return false;
        }
        entry.0 += 1;
        true
    }
}

/// rate_limit_middleware enforces the RateLimitConfig against the caller's IP.
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.rate_limit.load();
    if !cfg.enabled {
        return next.run(req).await;
    }

    let path = req.uri().path();
    if path_matches(path, &cfg.exempt_paths) {
        return next.run(req).await;
    }

    let ip = extract_client_ip(&req).unwrap_or_else(|| "unknown".to_string());
    let key = format!("ip:{ip}");

    let allowed = state
        .system_config
        .rate_limit_state
        .check(&key, cfg.per_ip_rpm);

    if !allowed {
        return (StatusCode::TOO_MANY_REQUESTS, "rate limit exceeded").into_response();
    }

    next.run(req).await
}

fn extract_client_ip(req: &Request) -> Option<String> {
    if let Some(value) = req.headers().get("x-forwarded-for") {
        if let Ok(s) = value.to_str() {
            return s.split(',').next().map(|s| s.trim().to_string());
        }
    }
    if let Some(value) = req.headers().get("x-real-ip") {
        if let Ok(s) = value.to_str() {
            return Some(s.to_string());
        }
    }
    let _ = header::HOST;
    None
}
