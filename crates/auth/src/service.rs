use std::sync::Arc;

use chrono::{DateTime, Utc};
use ennoia_kernel::{
    ApiKey, ApiKeyStore, AuthError, CreateApiKeyRequest, CreateSessionRequest, CreateUserRequest,
    Session, SessionStore, User, UserRole, UserStore,
};

use crate::hashing::{hash_password, verify_password};
use crate::tokens::{
    generate_api_key, generate_session_token, hash_token, mint_jwt, verify_jwt, JwtClaims,
};

/// LoginOutcome bundles the created session + raw token handed to the caller.
#[derive(Debug, Clone)]
pub struct LoginOutcome {
    pub user: User,
    pub session: Session,
    pub raw_token: String,
}

/// AuthService wraps the three stores and provides high-level auth flows.
#[derive(Clone)]
pub struct AuthService {
    pub users: Arc<dyn UserStore>,
    pub sessions: Arc<dyn SessionStore>,
    pub api_keys: Arc<dyn ApiKeyStore>,
}

impl AuthService {
    pub fn new(
        users: Arc<dyn UserStore>,
        sessions: Arc<dyn SessionStore>,
        api_keys: Arc<dyn ApiKeyStore>,
    ) -> Self {
        Self {
            users,
            sessions,
            api_keys,
        }
    }

    pub async fn register(
        &self,
        username: &str,
        password: &str,
        display_name: Option<String>,
        email: Option<String>,
        role: UserRole,
    ) -> Result<User, AuthError> {
        if username.trim().is_empty() {
            return Err(AuthError::Invalid("username must be non-empty".to_string()));
        }
        if password.len() < 6 {
            return Err(AuthError::Invalid(
                "password must be at least 6 characters".to_string(),
            ));
        }
        let hash = hash_password(password)?;
        self.users
            .create(CreateUserRequest {
                username: username.to_string(),
                display_name,
                email,
                role,
                password_hash: hash,
                owner_kind: None,
                owner_id: None,
            })
            .await
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
        ttl_seconds: u32,
        user_agent: Option<String>,
        ip: Option<String>,
    ) -> Result<LoginOutcome, AuthError> {
        let (user, password_hash) = self
            .users
            .get_by_username(username)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if !verify_password(password, &password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        let raw_token = generate_session_token()?;
        let token_hash = hash_token(&raw_token);

        let session = self
            .sessions
            .create(CreateSessionRequest {
                user_id: user.id.clone(),
                token_hash,
                ttl_seconds,
                user_agent,
                ip,
            })
            .await?;

        let now = Utc::now().to_rfc3339();
        self.users.touch_login(&user.id, &now).await?;

        Ok(LoginOutcome {
            user,
            session,
            raw_token,
        })
    }

    pub async fn logout(&self, raw_token: &str) -> Result<(), AuthError> {
        let hash = hash_token(raw_token);
        self.sessions.delete_by_token_hash(&hash).await
    }

    pub async fn authenticate_session(
        &self,
        raw_token: &str,
    ) -> Result<(User, Session), AuthError> {
        let hash = hash_token(raw_token);
        let session = self
            .sessions
            .find_by_token_hash(&hash)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if is_expired(&session.expires_at) {
            let _ = self.sessions.delete(&session.id).await;
            return Err(AuthError::Expired);
        }

        let user = self
            .users
            .get(&session.user_id)
            .await?
            .ok_or_else(|| AuthError::NotFound(format!("user {}", session.user_id)))?;

        let now = Utc::now().to_rfc3339();
        let _ = self.sessions.touch(&session.id, &now).await;

        Ok((user, session))
    }

    pub async fn authenticate_api_key(&self, raw_key: &str) -> Result<(User, ApiKey), AuthError> {
        let hash = hash_token(raw_key);
        let key = self
            .api_keys
            .find_by_key_hash(&hash)
            .await?
            .ok_or(AuthError::InvalidCredentials)?;

        if let Some(exp) = &key.expires_at {
            if is_expired(exp) {
                return Err(AuthError::Expired);
            }
        }

        let user = self
            .users
            .get(&key.user_id)
            .await?
            .ok_or_else(|| AuthError::NotFound(format!("user {}", key.user_id)))?;

        let now = Utc::now().to_rfc3339();
        let _ = self.api_keys.touch_used(&key.id, &now).await;

        Ok((user, key))
    }

    pub async fn authenticate_jwt(
        &self,
        token: &str,
        secret: &str,
    ) -> Result<(User, JwtClaims), AuthError> {
        let claims = verify_jwt(token, secret)?;
        let user = self
            .users
            .get(&claims.sub)
            .await?
            .ok_or_else(|| AuthError::NotFound(format!("user {}", claims.sub)))?;
        Ok((user, claims))
    }

    pub fn mint_jwt(
        &self,
        user: &User,
        secret: &str,
        ttl_seconds: u32,
    ) -> Result<String, AuthError> {
        mint_jwt(user, secret, ttl_seconds)
    }

    pub async fn create_api_key(
        &self,
        user_id: &str,
        label: Option<String>,
        scopes: Vec<String>,
        expires_at: Option<String>,
    ) -> Result<(ApiKey, String), AuthError> {
        let raw = generate_api_key()?;
        let hash = hash_token(&raw);
        let key = self
            .api_keys
            .create(CreateApiKeyRequest {
                user_id: user_id.to_string(),
                key_hash: hash,
                label,
                scopes,
                expires_at,
            })
            .await?;
        Ok((key, raw))
    }
}

fn is_expired(iso: &str) -> bool {
    match DateTime::parse_from_rfc3339(iso) {
        Ok(dt) => Utc::now().timestamp() > dt.timestamp(),
        Err(_) => true,
    }
}
