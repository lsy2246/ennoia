//! Server-scoped file-backed settings shapes.

use std::collections::HashMap;

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

pub fn default_local_dev_origins() -> Vec<String> {
    vec![
        "http://localhost:5173".to_string(),
        "http://127.0.0.1:5173".to_string(),
        "http://[::1]:5173".to_string(),
    ]
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            origins: default_local_dev_origins(),
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
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
    pub is_initialized: bool,
    pub initialized_at: Option<String>,
}
