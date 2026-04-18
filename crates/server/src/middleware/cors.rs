use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::app::AppState;

/// cors_middleware injects CORS headers from the live CorsConfig and short-circuits
/// OPTIONS preflight requests.
pub async fn cors_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.cors.load();
    if !cfg.enabled {
        return next.run(req).await;
    }

    let origin_header = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());

    let allowed_origin = origin_header
        .as_ref()
        .filter(|origin| cfg.origins.iter().any(|allowed| allowed == *origin || allowed == "*"))
        .cloned();

    let is_preflight = req.method() == Method::OPTIONS;

    let mut response = if is_preflight {
        let mut r = Response::new(Body::empty());
        *r.status_mut() = StatusCode::NO_CONTENT;
        r
    } else {
        next.run(req).await
    };

    let headers = response.headers_mut();
    if let Some(origin) = allowed_origin {
        if let Ok(v) = HeaderValue::from_str(&origin) {
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
        }
    }
    if cfg.credentials {
        headers.insert(
            header::ACCESS_CONTROL_ALLOW_CREDENTIALS,
            HeaderValue::from_static("true"),
        );
    }
    let methods = cfg.methods.join(", ");
    if let Ok(v) = HeaderValue::from_str(&methods) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, v);
    }
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type, x-api-key, x-requested-with"),
    );
    if let Ok(v) = HeaderValue::from_str(&cfg.max_age_seconds.to_string()) {
        headers.insert(header::ACCESS_CONTROL_MAX_AGE, v);
    }

    response
}
