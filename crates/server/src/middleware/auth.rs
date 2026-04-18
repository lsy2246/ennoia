use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::app::AppState;
use crate::middleware::path_matches;
use ennoia_kernel::AuthMode;

/// AuthedUser is the per-request identity injected into the request extensions.
#[derive(Debug, Clone, Serialize)]
pub struct AuthedUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub auth_method: &'static str,
}

impl AuthedUser {
    pub fn anonymous() -> Self {
        Self {
            id: "anonymous".to_string(),
            username: "anonymous".to_string(),
            role: "anonymous".to_string(),
            auth_method: "none",
        }
    }
}

/// auth_middleware enforces AuthConfig.
///
/// NOTE: Batch 2 ships only the `AuthMode::None` path. Batch 3 adds ApiKey/Jwt/Session.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request<Body>,
    next: Next,
) -> Response {
    let cfg = state.system_config.auth.load();

    if !cfg.enabled || matches!(cfg.mode, AuthMode::None) {
        req.extensions_mut().insert(AuthedUser::anonymous());
        return next.run(req).await;
    }

    let path = req.uri().path().to_string();
    let is_public = path_matches(&path, &cfg.public_paths);
    let is_protected = path_matches(&path, &cfg.protected_paths);

    if is_public && !is_protected {
        req.extensions_mut().insert(AuthedUser::anonymous());
        return next.run(req).await;
    }

    // Batch 3 will implement ApiKey / Jwt / Session here. For now, deny until Auth
    // is actually wired up.
    if is_protected {
        return (
            StatusCode::UNAUTHORIZED,
            "authentication required (not yet implemented)",
        )
            .into_response();
    }

    req.extensions_mut().insert(AuthedUser::anonymous());
    next.run(req).await
}
