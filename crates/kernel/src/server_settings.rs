//! Server-scoped file-backed settings shapes.

use std::collections::HashMap;
use std::env;

use serde::{Deserialize, Serialize};

// ========== RateLimitConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CorsConfig {
    pub enabled: bool,
    pub origins: Vec<String>,
    pub methods: Vec<String>,
    pub credentials: bool,
    pub max_age_seconds: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebDevConfig {
    pub host: String,
    pub port: u16,
}

impl Default for WebDevConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 5173,
        }
    }
}

pub fn default_local_dev_origins(host: &str, port: u16) -> Vec<String> {
    let mut origins = vec![
        format!("http://localhost:{port}"),
        format!("http://127.0.0.1:{port}"),
        format!("http://[::1]:{port}"),
    ];
    let normalized_host = host.trim();
    if !normalized_host.is_empty()
        && normalized_host != "localhost"
        && normalized_host != "127.0.0.1"
        && normalized_host != "::1"
        && normalized_host != "[::1]"
        && normalized_host != "0.0.0.0"
        && normalized_host != "::"
        && normalized_host != "[::]"
    {
        let formatted_host = if normalized_host.contains(':')
            && !normalized_host.starts_with('[')
            && !normalized_host.ends_with(']')
        {
            format!("[{normalized_host}]")
        } else {
            normalized_host.to_string()
        };
        origins.push(format!("http://{formatted_host}:{port}"));
    }
    origins
}

impl Default for CorsConfig {
    fn default() -> Self {
        let web_dev = WebDevConfig::default();
        Self {
            enabled: true,
            origins: default_local_dev_origins(&web_dev.host, web_dev.port),
            methods: vec![
                "GET".to_string(),
                "HEAD".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            credentials: true,
            max_age_seconds: 3600,
        }
    }
}

// ========== TimeoutConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimeoutConfig {
    pub enabled: bool,
    pub default_ms: u64,
    #[serde(default)]
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevConsoleLogConfig {
    pub enabled: bool,
    pub level: String,
}

impl Default for DevConsoleLogConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            level: "error".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoggingConfig {
    pub enabled: bool,
    pub level: String,
    pub sample_rate: f32,
    pub redact_headers: Vec<String>,
    #[serde(default)]
    pub dev_console: DevConsoleLogConfig,
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
            dev_console: DevConsoleLogConfig::default(),
        }
    }
}

pub fn apply_server_log_env_overrides(config: &mut LoggingConfig) {
    if let Some(level) = read_env_trimmed("ENNOIA_LOG_LEVEL") {
        config.level = level;
    }
}

fn read_env_trimmed(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

// ========== BodyLimitConfig ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BodyLimitConfig {
    pub enabled: bool,
    pub max_bytes: usize,
    #[serde(default)]
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
    pub is_initialized: bool,
    pub initialized_at: Option<String>,
}
