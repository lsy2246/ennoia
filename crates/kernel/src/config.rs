use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::extension::ProviderModelDescriptor;
use crate::permission::{AgentExecutionEnvironment, AgentPermissionProfile};
use crate::server_settings::{
    default_local_dev_origins, BackgroundRuntimeConfig, BodyLimitConfig, BootstrapState,
    CorsConfig, DevSupervisorConfig, ExtensionRuntimeDefaultsConfig, LoggingConfig,
    ProviderRuntimeConfig, RateLimitConfig, RuntimeOperationsConfig, ScheduleRuntimeConfig,
    StreamRuntimeConfig, TimeoutConfig, WebDevConfig,
};
use crate::ui::LocalizedText;

/// ServerConfig stores network and runtime settings loaded from `config/server.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub web_dev: WebDevConfig,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub timeout: TimeoutConfig,
    #[serde(default)]
    pub operations: RuntimeOperationsConfig,
    #[serde(default)]
    pub providers: ProviderRuntimeConfig,
    #[serde(default)]
    pub streams: StreamRuntimeConfig,
    #[serde(default)]
    pub background: BackgroundRuntimeConfig,
    #[serde(default)]
    pub extension_runtime: ExtensionRuntimeDefaultsConfig,
    #[serde(default)]
    pub schedules: ScheduleRuntimeConfig,
    #[serde(default)]
    pub dev_supervisor: DevSupervisorConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub body_limit: BodyLimitConfig,
    #[serde(default)]
    pub bootstrap: BootstrapState,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let web_dev = WebDevConfig::default();
        Self {
            host: "127.0.0.1".to_string(),
            port: 3710,
            web_dev: web_dev.clone(),
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig {
                origins: default_local_dev_origins(&web_dev.host, web_dev.port),
                ..CorsConfig::default()
            },
            timeout: TimeoutConfig::default(),
            operations: RuntimeOperationsConfig::default(),
            providers: ProviderRuntimeConfig::default(),
            streams: StreamRuntimeConfig::default(),
            background: BackgroundRuntimeConfig::default(),
            extension_runtime: ExtensionRuntimeDefaultsConfig::default(),
            schedules: ScheduleRuntimeConfig::default(),
            dev_supervisor: DevSupervisorConfig::default(),
            logging: LoggingConfig::default(),
            body_limit: BodyLimitConfig::default(),
            bootstrap: BootstrapState::default(),
        }
    }
}

impl ServerConfig {
    pub fn normalize(mut self) -> Self {
        self.sync_web_dev_origins();
        self.operations.normalize();
        self.providers.normalize();
        self.streams.normalize();
        self.background.normalize();
        self.extension_runtime.normalize();
        self.schedules.normalize();
        self.dev_supervisor.normalize();
        self
    }

    pub fn sync_web_dev_origins(&mut self) {
        let dev_origins = default_local_dev_origins(&self.web_dev.host, self.web_dev.port);
        let custom_origins = self
            .cors
            .origins
            .iter()
            .filter(|origin| {
                !origin.trim().is_empty() && !is_managed_web_dev_origin(origin, &self.web_dev.host)
            })
            .cloned()
            .collect::<Vec<_>>();
        self.cors.origins = dev_origins;
        self.cors.origins.extend(custom_origins);
    }
}

fn is_managed_web_dev_origin(origin: &str, web_dev_host: &str) -> bool {
    let normalized_origin = origin.trim();
    let Some(remainder) = normalized_origin.strip_prefix("http://") else {
        return false;
    };
    let authority = remainder.split('/').next().unwrap_or_default();
    let host = if let Some(stripped) = authority.strip_prefix('[') {
        stripped.split(']').next().unwrap_or_default()
    } else {
        authority
            .rsplit_once(':')
            .map(|(host, _)| host)
            .unwrap_or(authority)
    };
    let normalized_host = host.trim_matches(&['[', ']'][..]);
    let configured_host = web_dev_host.trim_matches(&['[', ']'][..]);

    matches!(normalized_host, "localhost" | "127.0.0.1" | "::1")
        || normalized_host == configured_host
}

/// UiConfig stores Web workbench settings loaded from `config/ui.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    pub web_title: LocalizedText,
    pub default_theme: String,
    pub default_locale: String,
    pub fallback_locale: String,
    pub available_locales: Vec<String>,
    #[serde(default = "default_ui_display_name")]
    pub default_display_name: String,
    #[serde(default = "default_ui_time_zone")]
    pub default_time_zone: String,
    pub show_command_palette: bool,
    #[serde(default)]
    pub api: UiApiConfig,
    #[serde(default)]
    pub notifications: UiNotificationConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            web_title: LocalizedText::new("web.title", "Ennoia"),
            default_theme: "system".to_string(),
            default_locale: "zh-CN".to_string(),
            fallback_locale: "en-US".to_string(),
            available_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
            default_display_name: default_ui_display_name(),
            default_time_zone: default_ui_time_zone(),
            show_command_palette: true,
            api: UiApiConfig::default(),
            notifications: UiNotificationConfig::default(),
        }
    }
}

impl UiConfig {
    pub fn normalize(mut self) -> Self {
        self.api.normalize();
        self.notifications.normalize();
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiApiConfig {
    pub default_request_timeout_ms: Option<u64>,
}

impl UiApiConfig {
    pub fn normalize(&mut self) {
        self.default_request_timeout_ms = self
            .default_request_timeout_ms
            .map(|value| value.clamp(1_000, 300_000));
    }
}

impl Default for UiApiConfig {
    fn default() -> Self {
        Self {
            default_request_timeout_ms: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiNotificationConfig {
    pub success_auto_dismiss_ms: u64,
    pub error_auto_dismiss_ms: u64,
    pub pause_on_hover: bool,
}

impl UiNotificationConfig {
    pub fn normalize(&mut self) {
        self.success_auto_dismiss_ms = self.success_auto_dismiss_ms.clamp(500, 60_000);
        self.error_auto_dismiss_ms = self.error_auto_dismiss_ms.clamp(500, 60_000);
    }
}

impl Default for UiNotificationConfig {
    fn default() -> Self {
        Self {
            success_auto_dismiss_ms: 3_200,
            error_auto_dismiss_ms: 6_000,
            pause_on_hover: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ServerConfig, UiConfig};

    #[test]
    fn ui_config_deserializes_web_title() {
        let config = toml::from_str::<UiConfig>(
            r#"
web_title = { key = "web.title", fallback = "Ennoia" }
default_theme = "system"
default_locale = "zh-CN"
fallback_locale = "en-US"
available_locales = ["zh-CN", "en-US"]
default_display_name = "Operator"
default_time_zone = "Asia/Shanghai"
show_command_palette = true

[notifications]
success_auto_dismiss_ms = 3200
error_auto_dismiss_ms = 6000
pause_on_hover = true
"#,
        )
        .expect("web_title should deserialize");

        assert_eq!(config.web_title.key, "web.title");
        assert_eq!(config.web_title.fallback, "Ennoia");
    }

    #[test]
    fn ui_config_rejects_legacy_shell_title() {
        let error = toml::from_str::<UiConfig>(
            r#"
shell_title = { key = "web.title", fallback = "Ennoia" }
default_theme = "system"
default_locale = "zh-CN"
fallback_locale = "en-US"
available_locales = ["zh-CN", "en-US"]
default_display_name = "Operator"
default_time_zone = "Asia/Shanghai"
show_command_palette = true

[notifications]
success_auto_dismiss_ms = 3200
error_auto_dismiss_ms = 6000
pause_on_hover = true
"#,
        )
        .expect_err("shell_title should no longer deserialize");

        assert!(error.to_string().contains("missing field `web_title`"));
    }

    #[test]
    fn server_config_syncs_web_dev_origins_and_preserves_custom_items() {
        let mut config = ServerConfig::default();
        config.web_dev.host = "192.168.1.20".to_string();
        config.web_dev.port = 4173;
        config.cors.origins = vec![
            "http://localhost:5173".to_string(),
            "https://example.com".to_string(),
        ];

        config.sync_web_dev_origins();

        assert_eq!(
            config.cors.origins,
            vec![
                "http://localhost:4173".to_string(),
                "http://127.0.0.1:4173".to_string(),
                "http://[::1]:4173".to_string(),
                "http://192.168.1.20:4173".to_string(),
                "https://example.com".to_string(),
            ]
        );
    }
}

/// AgentConfig represents the editable Agent profile fields exposed through the API.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub model_endpoint_id: String,
    #[serde(default)]
    pub model_id: String,
    #[serde(default)]
    pub generation_options: BTreeMap<String, String>,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default = "default_agent_kind")]
    pub kind: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub skills_dir: String,
    #[serde(default)]
    pub working_dir: String,
    #[serde(default)]
    pub artifacts_dir: String,
    #[serde(default)]
    pub execution_environment: AgentExecutionEnvironment,
}

/// AgentDocument stores one complete Agent package under `agents/<id>/agent.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentDocument {
    #[serde(flatten)]
    pub profile: AgentConfig,
    #[serde(default)]
    pub permission_profile: AgentPermissionProfile,
}

/// SkillManifest represents one installed skill package manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManifest {
    pub id: String,
    #[serde(default = "default_skill_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub mount: SkillMountConfig,
    #[serde(default)]
    pub actions: Vec<SkillActionConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillMountConfig {
    #[serde(default = "default_skill_mount_mode")]
    pub mode: String,
}

impl Default for SkillMountConfig {
    fn default() -> Self {
        Self {
            mode: default_skill_mount_mode(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillActionConfig {
    pub id: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub entry: String,
    #[serde(default = "default_skill_invoke_mode")]
    pub invoke_mode: String,
    #[serde(default)]
    pub requires: Vec<String>,
}

/// SkillConfig represents one resolved skill returned to the UI/runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillConfig {
    pub id: String,
    #[serde(default = "default_skill_version")]
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub mount: SkillMountConfig,
    #[serde(default)]
    pub actions: Vec<SkillActionConfig>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub builtin_sync_blocked: bool,
}

/// ExtensionRegistryFile stores extension package registration records under `config/extensions.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRegistryFile {
    #[serde(default)]
    pub blocked_builtin_sync: Vec<String>,
    #[serde(default)]
    pub extensions: Vec<ExtensionRegistryEntry>,
    #[serde(default)]
    pub dev_sources: Vec<ExtensionDevSourceEntry>,
}

/// ExtensionRegistryEntry records one installed extension's runtime state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRegistryEntry {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// ExtensionDevSourceEntry records one attached development source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDevSourceEntry {
    pub id: String,
    pub path: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// SkillRegistryFile stores skill package registration records under `config/skills.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRegistryFile {
    #[serde(default)]
    pub blocked_builtin_sync: Vec<String>,
    #[serde(default)]
    pub skills: Vec<SkillRegistryEntry>,
}

/// SkillRegistryEntry records one installed skill's runtime state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRegistryEntry {
    pub id: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// ModelEndpointConfig represents one file under `config/model-endpoints/*.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelEndpointConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub available_models: Vec<ProviderModelDescriptor>,
    #[serde(default)]
    pub model_discovery: ModelEndpointModelDiscoveryConfig,
    #[serde(default)]
    pub request_timeout_ms: Option<u64>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelEndpointModelDiscoveryConfig {
    #[serde(default = "default_enabled")]
    pub manual_allowed: bool,
}

impl Default for ModelEndpointModelDiscoveryConfig {
    fn default() -> Self {
        Self {
            manual_allowed: true,
        }
    }
}

fn default_agent_kind() -> String {
    "agent".to_string()
}

fn default_ui_display_name() -> String {
    "Operator".to_string()
}

fn default_ui_time_zone() -> String {
    "Asia/Shanghai".to_string()
}

fn default_enabled() -> bool {
    true
}

fn default_skill_version() -> String {
    "1.0.0".to_string()
}

fn default_skill_mount_mode() -> String {
    "auto".to_string()
}

fn default_skill_invoke_mode() -> String {
    "manual".to_string()
}

#[cfg(test)]
mod provider_config_tests {
    use super::ModelEndpointConfig;

    #[test]
    fn provider_config_deserializes_model_descriptors() {
        let config = toml::from_str::<ModelEndpointConfig>(
            r#"
id = "openai"
display_name = "OpenAI"
kind = "openai"
base_url = "https://api.openai.com/v1"
api_key = ""
api_key_env = "OPENAI_API_KEY"
default_model = "gpt-5.4"
available_models = [
  { id = "gpt-5.4", max_context_tokens = 128000, max_input_tokens = 32000 }
]
enabled = true

[model_discovery]
manual_allowed = true
"#,
        )
        .expect("provider config should deserialize");

        assert_eq!(config.available_models.len(), 1);
        assert_eq!(config.available_models[0].id, "gpt-5.4");
        assert_eq!(config.available_models[0].max_context_tokens, Some(128000));
        assert_eq!(config.available_models[0].max_input_tokens, Some(32000));
    }

    #[test]
    fn provider_config_deserializes_legacy_string_model_list() {
        let config = toml::from_str::<ModelEndpointConfig>(
            r#"
id = "openai"
display_name = "OpenAI"
kind = "openai"
base_url = "https://api.openai.com/v1"
api_key = ""
api_key_env = "OPENAI_API_KEY"
default_model = "gpt-5.4"
available_models = ["gpt-5.4"]
enabled = true

[model_discovery]
manual_allowed = true
"#,
        )
        .expect("legacy string model list should deserialize");

        assert_eq!(config.available_models.len(), 1);
        assert_eq!(config.available_models[0].id, "gpt-5.4");
        assert_eq!(config.available_models[0].max_context_tokens, None);
        assert_eq!(config.available_models[0].max_input_tokens, None);
    }
}
