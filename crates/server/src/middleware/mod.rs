//! Middleware stack — each layer reads the latest config from SystemConfigRuntime.

pub mod auth;
pub mod body_limit;
pub mod cors;
pub mod logging;
pub mod rate_limit;
pub mod timeout;

pub use auth::{auth_middleware, AuthedUser};
pub use body_limit::body_limit_middleware;
pub use cors::cors_middleware;
pub use logging::logging_middleware;
pub use rate_limit::{rate_limit_middleware, RateLimitState};
pub use timeout::timeout_middleware;

use ennoia_kernel::GlobPattern;

/// path_matches returns true when the given path matches any of the glob patterns.
pub fn path_matches(path: &str, patterns: &[String]) -> bool {
    patterns
        .iter()
        .any(|p| GlobPattern::new(p.clone()).matches(path))
}
