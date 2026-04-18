//! Auth domain: users, sessions, API keys, and their store contracts.
//!
//! All shapes and traits live in the kernel. The `ennoia-auth` crate owns
//! the concrete Sqlite implementations + password/token helpers.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ========== UserRole ==========

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UserRole {
    User,
    Admin,
}

impl Default for UserRole {
    fn default() -> Self {
        UserRole::User
    }
}

impl UserRole {
    pub fn as_str(self) -> &'static str {
        match self {
            UserRole::User => "user",
            UserRole::Admin => "admin",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "admin" => UserRole::Admin,
            _ => UserRole::User,
        }
    }

    pub fn is_admin(self) -> bool {
        matches!(self, UserRole::Admin)
    }
}

// ========== User ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct User {
    pub id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: UserRole,
    pub owner_kind: Option<String>,
    pub owner_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub last_login_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateUserRequest {
    pub username: String,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: UserRole,
    pub password_hash: String,
    pub owner_kind: Option<String>,
    pub owner_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct UpdateUserRequest {
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub role: Option<UserRole>,
}

// ========== Session ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub token_hash: String,
    pub created_at: String,
    pub expires_at: String,
    pub last_seen_at: Option<String>,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateSessionRequest {
    pub user_id: String,
    pub token_hash: String,
    pub ttl_seconds: u32,
    pub user_agent: Option<String>,
    pub ip: Option<String>,
}

// ========== ApiKey ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKey {
    pub id: String,
    pub user_id: String,
    pub key_hash: String,
    pub label: Option<String>,
    pub scopes: Vec<String>,
    pub created_at: String,
    pub expires_at: Option<String>,
    pub last_used_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CreateApiKeyRequest {
    pub user_id: String,
    pub key_hash: String,
    pub label: Option<String>,
    pub scopes: Vec<String>,
    pub expires_at: Option<String>,
}

// ========== Store traits ==========

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn create(&self, req: CreateUserRequest) -> Result<User, AuthError>;
    async fn get(&self, id: &str) -> Result<Option<User>, AuthError>;
    async fn get_by_username(&self, username: &str) -> Result<Option<(User, String)>, AuthError>;
    async fn list(&self) -> Result<Vec<User>, AuthError>;
    async fn update(&self, id: &str, update: UpdateUserRequest) -> Result<User, AuthError>;
    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), AuthError>;
    async fn delete(&self, id: &str) -> Result<(), AuthError>;
    async fn touch_login(&self, id: &str, now_iso: &str) -> Result<(), AuthError>;
    async fn count(&self) -> Result<u32, AuthError>;
}

#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn create(&self, req: CreateSessionRequest) -> Result<Session, AuthError>;
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AuthError>;
    async fn touch(&self, id: &str, now_iso: &str) -> Result<(), AuthError>;
    async fn delete(&self, id: &str) -> Result<(), AuthError>;
    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), AuthError>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Session>, AuthError>;
    async fn list_all(&self) -> Result<Vec<Session>, AuthError>;
    async fn prune_expired(&self, now_iso: &str) -> Result<u64, AuthError>;
}

#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    async fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKey, AuthError>;
    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, AuthError>;
    async fn list(&self) -> Result<Vec<ApiKey>, AuthError>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKey>, AuthError>;
    async fn touch_used(&self, id: &str, now_iso: &str) -> Result<(), AuthError>;
    async fn delete(&self, id: &str) -> Result<(), AuthError>;
}

// ========== AuthError ==========

#[derive(Debug)]
pub enum AuthError {
    Backend(String),
    Serde(String),
    NotFound(String),
    InvalidCredentials,
    Duplicate(String),
    Expired,
    Forbidden(String),
    Invalid(String),
    NotBootstrapped,
    BootstrapAlreadyDone,
}

impl std::fmt::Display for AuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthError::Backend(r) => write!(f, "auth backend error: {r}"),
            AuthError::Serde(r) => write!(f, "auth serde error: {r}"),
            AuthError::NotFound(k) => write!(f, "not found: {k}"),
            AuthError::InvalidCredentials => write!(f, "invalid credentials"),
            AuthError::Duplicate(k) => write!(f, "duplicate: {k}"),
            AuthError::Expired => write!(f, "expired"),
            AuthError::Forbidden(r) => write!(f, "forbidden: {r}"),
            AuthError::Invalid(r) => write!(f, "invalid: {r}"),
            AuthError::NotBootstrapped => write!(f, "system not bootstrapped"),
            AuthError::BootstrapAlreadyDone => write!(f, "bootstrap already completed"),
        }
    }
}

impl std::error::Error for AuthError {}
