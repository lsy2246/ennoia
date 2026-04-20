use axum::{
    body::Body,
    extract::{Request, State},
    http::{header, HeaderMap, HeaderValue, Method, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::app::AppState;

/// cors_middleware injects CORS headers from the live CorsConfig and short-circuits
/// OPTIONS preflight requests.
pub async fn cors_middleware(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let cfg = state.system_config.cors.load();
    if !cfg.enabled {
        return next.run(req).await;
    }

    let origin_header = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let requested_headers = req
        .headers()
        .get(header::ACCESS_CONTROL_REQUEST_HEADERS)
        .cloned();

    let allowed_origin = origin_header
        .as_ref()
        .filter(|origin| is_origin_allowed(origin, &cfg.origins))
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
    apply_vary_headers(headers);
    if let Some(origin) = allowed_origin {
        if let Ok(v) = HeaderValue::from_str(&origin) {
            headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, v);
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
        if let Some(requested) = requested_headers {
            headers.insert(header::ACCESS_CONTROL_ALLOW_HEADERS, requested);
        } else {
            headers.insert(
                header::ACCESS_CONTROL_ALLOW_HEADERS,
                HeaderValue::from_static(
                    "authorization, content-type, x-api-key, x-requested-with",
                ),
            );
        }
        if let Ok(v) = HeaderValue::from_str(&cfg.max_age_seconds.to_string()) {
            headers.insert(header::ACCESS_CONTROL_MAX_AGE, v);
        }
    }

    response
}

fn is_origin_allowed(origin: &str, allowed_origins: &[String]) -> bool {
    allowed_origins
        .iter()
        .any(|allowed| allowed == "*" || allowed.eq_ignore_ascii_case(origin))
        || allowed_origins
            .iter()
            .any(|allowed| are_loopback_origins_equivalent(allowed, origin))
}

fn apply_vary_headers(headers: &mut HeaderMap) {
    headers.insert(
        header::VARY,
        HeaderValue::from_static("Origin, Access-Control-Request-Headers"),
    );
}

fn are_loopback_origins_equivalent(left: &str, right: &str) -> bool {
    let Some(left) = ParsedOrigin::parse(left) else {
        return false;
    };
    let Some(right) = ParsedOrigin::parse(right) else {
        return false;
    };

    left.scheme.eq_ignore_ascii_case(right.scheme)
        && left.port == right.port
        && left.is_loopback_host()
        && right.is_loopback_host()
}

#[derive(Debug, PartialEq, Eq)]
struct ParsedOrigin<'a> {
    scheme: &'a str,
    host: &'a str,
    port: Option<u16>,
}

impl<'a> ParsedOrigin<'a> {
    fn parse(origin: &'a str) -> Option<Self> {
        let (scheme, remainder) = origin.split_once("://")?;
        let authority = remainder.split('/').next()?;
        if authority.is_empty() {
            return None;
        }

        if let Some(stripped) = authority.strip_prefix('[') {
            let end = stripped.find(']')?;
            let host = &stripped[..end];
            let suffix = &stripped[end + 1..];
            let port = match suffix.strip_prefix(':') {
                Some(port) => Some(port.parse().ok()?),
                None if suffix.is_empty() => None,
                None => return None,
            };
            return Some(Self { scheme, host, port });
        }

        let (host, port) = match authority.rsplit_once(':') {
            Some((host, port)) if !host.contains(':') => (host, Some(port.parse().ok()?)),
            _ => (authority, None),
        };

        Some(Self { scheme, host, port })
    }

    fn is_loopback_host(&self) -> bool {
        self.host.eq_ignore_ascii_case("localhost")
            || self.host == "127.0.0.1"
            || self.host == "::1"
    }
}

#[cfg(test)]
mod tests {
    use super::{are_loopback_origins_equivalent, is_origin_allowed, ParsedOrigin};

    #[test]
    fn parses_ipv4_and_ipv6_origins() {
        assert_eq!(
            ParsedOrigin::parse("http://127.0.0.1:5173"),
            Some(ParsedOrigin {
                scheme: "http",
                host: "127.0.0.1",
                port: Some(5173),
            })
        );
        assert_eq!(
            ParsedOrigin::parse("http://[::1]:5173"),
            Some(ParsedOrigin {
                scheme: "http",
                host: "::1",
                port: Some(5173),
            })
        );
    }

    #[test]
    fn treats_loopback_aliases_as_equivalent() {
        assert!(are_loopback_origins_equivalent(
            "http://localhost:5173",
            "http://127.0.0.1:5173"
        ));
        assert!(are_loopback_origins_equivalent(
            "http://localhost:5173",
            "http://[::1]:5173"
        ));
        assert!(!are_loopback_origins_equivalent(
            "http://localhost:5173",
            "http://127.0.0.1:4173"
        ));
    }

    #[test]
    fn allows_matching_loopback_origin() {
        let allowed = vec!["http://localhost:5173".to_string()];
        assert!(is_origin_allowed("http://127.0.0.1:5173", &allowed));
        assert!(!is_origin_allowed("http://example.com:5173", &allowed));
    }
}
