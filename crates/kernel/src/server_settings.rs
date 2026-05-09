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

// ========== Runtime Operation Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeOperationTimeoutConfig {
    pub default_timeout_ms: u64,
    pub min_timeout_ms: u64,
    pub max_timeout_ms: u64,
}

impl RuntimeOperationTimeoutConfig {
    pub fn normalize(&mut self) {
        self.min_timeout_ms = self.min_timeout_ms.max(1);
        self.max_timeout_ms = self.max_timeout_ms.max(self.min_timeout_ms);
        self.default_timeout_ms = self
            .default_timeout_ms
            .clamp(self.min_timeout_ms, self.max_timeout_ms);
    }
}

impl Default for RuntimeOperationTimeoutConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 120_000,
            min_timeout_ms: 1_000,
            max_timeout_ms: 3_600_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeOperationsConfig {
    #[serde(default)]
    pub command: RuntimeOperationTimeoutConfig,
    #[serde(default)]
    pub net: RuntimeOperationTimeoutConfig,
}

impl RuntimeOperationsConfig {
    pub fn normalize(&mut self) {
        self.command.normalize();
        self.net.normalize();
    }
}

impl Default for RuntimeOperationsConfig {
    fn default() -> Self {
        Self {
            command: RuntimeOperationTimeoutConfig::default(),
            net: RuntimeOperationTimeoutConfig::default(),
        }
    }
}

// ========== Provider Runtime Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderRuntimeConfig {
    pub default_request_timeout_ms: u64,
}

impl ProviderRuntimeConfig {
    pub fn normalize(&mut self) {
        self.default_request_timeout_ms = self.default_request_timeout_ms.clamp(1_000, 600_000);
    }
}

impl Default for ProviderRuntimeConfig {
    fn default() -> Self {
        Self {
            default_request_timeout_ms: 300_000,
        }
    }
}

// ========== Stream Runtime Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StreamRuntimeConfig {
    pub conversation_poll_ms: u64,
    pub workflow_poll_ms: u64,
    pub logs_poll_ms: u64,
}

impl StreamRuntimeConfig {
    pub fn normalize(&mut self) {
        self.conversation_poll_ms = self.conversation_poll_ms.clamp(100, 60_000);
        self.workflow_poll_ms = self.workflow_poll_ms.clamp(100, 60_000);
        self.logs_poll_ms = self.logs_poll_ms.clamp(100, 60_000);
    }
}

// ========== Background Runtime Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BackgroundRuntimeConfig {
    pub extension_refresh_ms: u64,
    pub schedule_tick_ms: u64,
    pub event_delivery_tick_ms: u64,
}

impl BackgroundRuntimeConfig {
    pub fn normalize(&mut self) {
        self.extension_refresh_ms = self.extension_refresh_ms.clamp(100, 60_000);
        self.schedule_tick_ms = self.schedule_tick_ms.clamp(100, 60_000);
        self.event_delivery_tick_ms = self.event_delivery_tick_ms.clamp(100, 60_000);
    }
}

impl Default for BackgroundRuntimeConfig {
    fn default() -> Self {
        Self {
            extension_refresh_ms: 2_000,
            schedule_tick_ms: 1_000,
            event_delivery_tick_ms: 1_000,
        }
    }
}

impl Default for StreamRuntimeConfig {
    fn default() -> Self {
        Self {
            conversation_poll_ms: 1_000,
            workflow_poll_ms: 1_000,
            logs_poll_ms: 1_000,
        }
    }
}

// ========== Extension Runtime Defaults ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRuntimeDefaultsConfig {
    pub timeout_ms: u64,
    pub memory_limit_mb: u32,
}

impl ExtensionRuntimeDefaultsConfig {
    pub fn normalize(&mut self) {
        self.timeout_ms = self.timeout_ms.clamp(1_000, 600_000);
        self.memory_limit_mb = self.memory_limit_mb.clamp(16, 8_192);
    }
}

impl Default for ExtensionRuntimeDefaultsConfig {
    fn default() -> Self {
        Self {
            timeout_ms: 30_000,
            memory_limit_mb: 128,
        }
    }
}

// ========== Schedule Runtime Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleCommandConfig {
    pub default_timeout_ms: u64,
    pub min_timeout_ms: u64,
    pub max_timeout_ms: u64,
}

impl ScheduleCommandConfig {
    pub fn normalize(&mut self) {
        self.min_timeout_ms = self.min_timeout_ms.max(1);
        self.max_timeout_ms = self.max_timeout_ms.max(self.min_timeout_ms);
        self.default_timeout_ms = self
            .default_timeout_ms
            .clamp(self.min_timeout_ms, self.max_timeout_ms);
    }
}

impl Default for ScheduleCommandConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 120_000,
            min_timeout_ms: 1_000,
            max_timeout_ms: 3_600_000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleRetryConfig {
    pub default_max_attempts: u8,
    pub max_attempts_cap: u8,
    pub default_backoff_seconds: u64,
    pub max_backoff_seconds: u64,
}

impl ScheduleRetryConfig {
    pub fn normalize(&mut self) {
        self.max_attempts_cap = self.max_attempts_cap.clamp(1, u8::MAX);
        self.default_max_attempts = self.default_max_attempts.clamp(1, self.max_attempts_cap);
        self.max_backoff_seconds = self.max_backoff_seconds.clamp(0, 86_400);
        self.default_backoff_seconds = self.default_backoff_seconds.min(self.max_backoff_seconds);
    }
}

impl Default for ScheduleRetryConfig {
    fn default() -> Self {
        Self {
            default_max_attempts: 1,
            max_attempts_cap: 10,
            default_backoff_seconds: 0,
            max_backoff_seconds: 3_600,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleRuntimeConfig {
    #[serde(default)]
    pub command: ScheduleCommandConfig,
    #[serde(default)]
    pub retry: ScheduleRetryConfig,
}

impl ScheduleRuntimeConfig {
    pub fn normalize(&mut self) {
        self.command.normalize();
        self.retry.normalize();
    }
}

impl Default for ScheduleRuntimeConfig {
    fn default() -> Self {
        Self {
            command: ScheduleCommandConfig::default(),
            retry: ScheduleRetryConfig::default(),
        }
    }
}

// ========== Dev Supervisor Config ==========

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DevSupervisorConfig {
    pub host_reload_debounce_ms: u64,
    pub watch_poll_ms: u64,
    pub api_ready_timeout_ms: u64,
    pub api_healthcheck_interval_ms: u64,
    pub api_healthcheck_grace_ms: u64,
    pub api_port_release_timeout_ms: u64,
    pub child_startup_grace_ms: u64,
    pub probe_socket_timeout_ms: u64,
}

impl DevSupervisorConfig {
    pub fn normalize(&mut self) {
        self.host_reload_debounce_ms = self.host_reload_debounce_ms.clamp(0, 60_000);
        self.watch_poll_ms = self.watch_poll_ms.clamp(50, 60_000);
        self.api_ready_timeout_ms = self.api_ready_timeout_ms.clamp(1_000, 600_000);
        self.api_healthcheck_interval_ms = self.api_healthcheck_interval_ms.clamp(250, 60_000);
        self.api_healthcheck_grace_ms = self.api_healthcheck_grace_ms.clamp(250, 600_000);
        self.api_port_release_timeout_ms = self.api_port_release_timeout_ms.clamp(250, 600_000);
        self.child_startup_grace_ms = self.child_startup_grace_ms.clamp(0, 600_000);
        self.probe_socket_timeout_ms = self.probe_socket_timeout_ms.clamp(100, 60_000);
    }
}

impl Default for DevSupervisorConfig {
    fn default() -> Self {
        Self {
            host_reload_debounce_ms: 800,
            watch_poll_ms: 250,
            api_ready_timeout_ms: 30_000,
            api_healthcheck_interval_ms: 3_000,
            api_healthcheck_grace_ms: 6_000,
            api_port_release_timeout_ms: 20_000,
            child_startup_grace_ms: 1_500,
            probe_socket_timeout_ms: 1_500,
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
