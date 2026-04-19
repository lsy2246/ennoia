//! System-wide runtime configuration (middleware + auth + bootstrap state).
//!
//! All config shapes live in the kernel. The `ennoia-config` crate provides
//! the SqliteConfigStore. The server layer wraps each sub-config in
//! `Arc<ArcSwap<...>>` for hot reload.

use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

// ========== Top-level SystemConfig ==========

/// SystemConfig bundles every runtime-configurable middleware/auth/bootstrap state.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SystemConfig {
    pub auth: AuthConfig,
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
    pub timeout: TimeoutConfig,
    pub logging: LoggingConfig,
    pub body_limit: BodyLimitConfig,
    pub bootstrap: BootstrapState,
}

/// Stable config keys used in the `system_config` table.
pub const CONFIG_KEY_AUTH: &str = "auth";
pub const CONFIG_KEY_RATE_LIMIT: &str = "rate_limit";
pub const CONFIG_KEY_CORS: &str = "cors";
pub const CONFIG_KEY_TIMEOUT: &str = "timeout";
pub const CONFIG_KEY_LOGGING: &str = "logging";
pub const CONFIG_KEY_BODY_LIMIT: &str = "body_limit";
pub const CONFIG_KEY_BOOTSTRAP: &str = "bootstrap";

pub const ALL_CONFIG_KEYS: &[&str] = &[
    CONFIG_KEY_AUTH,
    CONFIG_KEY_RATE_LIMIT,
    CONFIG_KEY_CORS,
    CONFIG_KEY_TIMEOUT,
    CONFIG_KEY_LOGGING,
    CONFIG_KEY_BODY_LIMIT,
    CONFIG_KEY_BOOTSTRAP,
];

// ========== AuthConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AuthConfig {
    pub enabled: bool,
    pub mode: AuthMode,
    pub jwt_secret: Option<String>,
    pub session_ttl_seconds: u32,
    pub protected_paths: Vec<String>,
    pub public_paths: Vec<String>,
    pub allow_registration: bool,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            mode: AuthMode::None,
            jwt_secret: None,
            session_ttl_seconds: 60 * 60 * 24 * 7, // 7 days
            protected_paths: vec!["/api/v1/admin/**".to_string()],
            public_paths: vec![
                "/health".to_string(),
                "/api/v1/auth/**".to_string(),
                "/api/v1/bootstrap/**".to_string(),
            ],
            allow_registration: false,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthMode {
    None,
    ApiKey,
    Jwt,
    Session,
}

impl Default for AuthMode {
    fn default() -> Self {
        AuthMode::None
    }
}

impl AuthMode {
    pub fn as_str(self) -> &'static str {
        match self {
            AuthMode::None => "none",
            AuthMode::ApiKey => "api_key",
            AuthMode::Jwt => "jwt",
            AuthMode::Session => "session",
        }
    }
}

// ========== RateLimitConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RateLimitConfig {
    pub enabled: bool,
    pub per_ip_rpm: u32,
    pub per_user_rpm: u32,
    pub burst: u32,
    pub exempt_paths: Vec<String>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            per_ip_rpm: 300,
            per_user_rpm: 600,
            burst: 60,
            exempt_paths: vec!["/health".to_string()],
        }
    }
}

// ========== CorsConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CorsConfig {
    pub enabled: bool,
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub credentials: bool,
    pub max_age_seconds: u32,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            origins: vec!["http://localhost:5173".to_string()],
            methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
            ],
            credentials: true,
            max_age_seconds: 3600,
        }
    }
}

// ========== TimeoutConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimeoutConfig {
    pub enabled: bool,
    pub default_ms: u64,
    pub per_path_ms: HashMap<String, u64>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            default_ms: 30_000,
            per_path_ms: HashMap::new(),
        }
    }
}

// ========== LoggingConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: String,
    pub sample_rate: f32,
    pub redact_headers: Vec<String>,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "info".to_string(),
            sample_rate: 1.0,
            redact_headers: vec![
                "authorization".to_string(),
                "cookie".to_string(),
                "x-api-key".to_string(),
            ],
        }
    }
}

// ========== BodyLimitConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BodyLimitConfig {
    pub enabled: bool,
    pub max_bytes: usize,
    pub per_path_max: HashMap<String, usize>,
}

impl Default for BodyLimitConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            max_bytes: 1024 * 1024, // 1 MB
            per_path_max: HashMap::new(),
        }
    }
}

// ========== BootstrapState ==========

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct BootstrapState {
    pub completed: bool,
    pub admin_created_at: Option<String>,
}

// ========== ConfigStore trait ==========

/// ConfigEntry is one row of the `system_config` table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigEntry {
    pub key: String,
    pub payload_json: String,
    pub enabled: bool,
    pub version: u32,
    pub updated_by: Option<String>,
    pub updated_at: String,
}

/// ConfigChangeRecord is one row of `system_config_history`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConfigChangeRecord {
    pub id: String,
    pub config_key: String,
    pub old_payload_json: Option<String>,
    pub new_payload_json: String,
    pub changed_by: Option<String>,
    pub changed_at: String,
}

#[async_trait]
pub trait ConfigStore: Send + Sync {
    async fn list(&self) -> Result<Vec<ConfigEntry>, ConfigError>;
    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigError>;
    async fn put(
        &self,
        key: &str,
        payload: &serde_json::Value,
        updated_by: Option<&str>,
    ) -> Result<ConfigEntry, ConfigError>;
    async fn history(&self, key: &str, limit: u32) -> Result<Vec<ConfigChangeRecord>, ConfigError>;
}

// ========== Error ==========

#[derive(Debug)]
pub enum ConfigError {
    Backend(String),
    Serde(String),
    Invalid(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Backend(reason) => write!(f, "config backend error: {reason}"),
            ConfigError::Serde(reason) => write!(f, "config serde error: {reason}"),
            ConfigError::Invalid(reason) => write!(f, "config invalid input: {reason}"),
        }
    }
}

impl std::error::Error for ConfigError {}
