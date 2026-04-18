use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::app::AppState;
use crate::middleware::path_matches;
use ennoia_kernel::AuthMode;

/// AuthedUser is the per-request identity injected into request extensions.
#[derive(Debug, Clone, Serialize)]
pub struct AuthedUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub auth_method: String,
}

impl AuthedUser {
    pub fn anonymous() -> Self {
        Self {
            id: "anonymous".to_string(),
            username: "anonymous".to_string(),
            role: "anonymous".to_string(),
            auth_method: "none".to_string(),
        }
    }

    pub fn is_admin(&self) -> bool {
        self.role == "admin"
    }
}

/// auth_middleware enforces AuthConfig. Handles all 4 modes (None/ApiKey/Jwt/Session).
///
/// IMPORTANT: axum's `Body` is `!Sync`, so we must NOT hold `&Request` across any
/// `.await`. We extract every needed header up front as owned Strings, then do
/// the async work without referencing the request.
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut req: Request,
    next: Next,
) -> Response {
    let cfg = state.system_config.auth.load_full();

    if !cfg.enabled || matches!(cfg.mode, AuthMode::None) {
        req.extensions_mut().insert(AuthedUser::anonymous());
        return next.run(req).await;
    }

    // Step 1: extract everything we need from the request into owned values.
    let path = req.uri().path().to_string();
    let api_key_value = header_string(&req, "x-api-key");
    let bearer_value = bearer_token(&req);
    let cookie_session = cookie_session_token(&req);

    // Step 2: snapshot config fields we'll need after the drop.
    let is_public = path_matches(&path, &cfg.public_paths);
    let is_protected = path_matches(&path, &cfg.protected_paths);
    let mode = cfg.mode;
    let jwt_secret = cfg.jwt_secret.clone().unwrap_or_default();
    drop(cfg);

    // Step 3: dispatch. No `&req` is held across these awaits.
    let authed: Option<AuthedUser> = match mode {
        AuthMode::None => None,
        AuthMode::ApiKey => match api_key_value.or_else(|| bearer_value.clone()) {
            Some(key) => lookup_api_key(&state, &key).await,
            None => None,
        },
        AuthMode::Session => match bearer_value.or(cookie_session) {
            Some(token) => lookup_session(&state, &token).await,
            None => None,
        },
        AuthMode::Jwt => match bearer_value {
            Some(token) => lookup_jwt(&state, &token, &jwt_secret).await,
            None => None,
        },
    };

    match authed {
        Some(user) => {
            req.extensions_mut().insert(user);
            next.run(req).await
        }
        None => {
            if is_public && !is_protected {
                req.extensions_mut().insert(AuthedUser::anonymous());
                return next.run(req).await;
            }
            if is_protected {
                return (StatusCode::UNAUTHORIZED, "authentication required").into_response();
            }
            (StatusCode::UNAUTHORIZED, "authentication required").into_response()
        }
    }
}

async fn lookup_api_key(state: &AppState, key: &str) -> Option<AuthedUser> {
    match state.auth_service.authenticate_api_key(key).await {
        Ok((user, _key)) => Some(AuthedUser {
            id: user.id,
            username: user.username,
            role: user.role.as_str().to_string(),
            auth_method: "api_key".to_string(),
        }),
        Err(_) => None,
    }
}

async fn lookup_session(state: &AppState, token: &str) -> Option<AuthedUser> {
    match state.auth_service.authenticate_session(token).await {
        Ok((user, _sess)) => Some(AuthedUser {
            id: user.id,
            username: user.username,
            role: user.role.as_str().to_string(),
            auth_method: "session".to_string(),
        }),
        Err(_) => None,
    }
}

async fn lookup_jwt(state: &AppState, token: &str, secret: &str) -> Option<AuthedUser> {
    match state.auth_service.authenticate_jwt(token, secret).await {
        Ok((user, _claims)) => Some(AuthedUser {
            id: user.id,
            username: user.username,
            role: user.role.as_str().to_string(),
            auth_method: "jwt".to_string(),
        }),
        Err(_) => None,
    }
}

fn header_string(req: &Request, name: &str) -> Option<String> {
    req.headers()
        .get(name)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
}

fn bearer_token(req: &Request) -> Option<String> {
    let value = req.headers().get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ").map(|s| s.trim().to_string())
}

fn cookie_session_token(req: &Request) -> Option<String> {
    let cookie = req
        .headers()
        .get(header::COOKIE)
        .and_then(|v| v.to_str().ok())?;
    for part in cookie.split(';') {
        let part = part.trim();
        if let Some(value) = part.strip_prefix("ennoia_session=") {
            return Some(value.to_string());
        }
    }
    None
}

/// require_admin is a small helper for admin-only handlers.
pub fn require_admin(user: &AuthedUser, state: &AppState) -> Result<(), (StatusCode, String)> {
    let cfg = state.system_config.auth.load_full();
    if !cfg.enabled {
        return Ok(());
    }
    if user.is_admin() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "admin role required".to_string()))
    }
}
