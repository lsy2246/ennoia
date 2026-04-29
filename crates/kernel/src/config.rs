use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::server_settings::{
    default_local_dev_origins, BodyLimitConfig, BootstrapState, CorsConfig, LoggingConfig,
    RateLimitConfig, TimeoutConfig,
};
use crate::ui::LocalizedText;

/// AppConfig stores startup-wide settings loaded from `config/ennoia.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub app_name: String,
    pub mode: String,
    pub default_mention_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "ennoia".to_string(),
            mode: "development".to_string(),
            default_mention_mode: "configured".to_string(),
        }
    }
}

/// ServerConfig stores network and runtime settings loaded from `config/server.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub enable_ws: bool,
    #[serde(default)]
    pub rate_limit: RateLimitConfig,
    #[serde(default)]
    pub cors: CorsConfig,
    #[serde(default)]
    pub timeout: TimeoutConfig,
    #[serde(default)]
    pub logging: LoggingConfig,
    #[serde(default)]
    pub body_limit: BodyLimitConfig,
    #[serde(default)]
    pub bootstrap: BootstrapState,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3710,
            enable_ws: true,
            rate_limit: RateLimitConfig::default(),
            cors: CorsConfig {
                origins: default_local_dev_origins(),
                ..CorsConfig::default()
            },
            timeout: TimeoutConfig::default(),
            logging: LoggingConfig::default(),
            body_limit: BodyLimitConfig::default(),
            bootstrap: BootstrapState::default(),
        }
    }
}

/// InterfaceBindingsConfig stores fine-grained system action bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceBindingsConfig {
    #[serde(default)]
    pub bindings: BTreeMap<String, InterfaceBindingConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceBindingConfig {
    pub extension_id: String,
    pub method: String,
}

/// UiConfig stores Web workbench settings loaded from `config/ui.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    pub web_title: LocalizedText,
    pub default_theme: String,
    pub default_locale: String,
    pub fallback_locale: String,
    pub available_locales: Vec<String>,
    pub dock_persistence: bool,
    pub default_page: String,
    pub show_command_palette: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            web_title: LocalizedText::new("web.title", "Ennoia"),
            default_theme: "system".to_string(),
            default_locale: "zh-CN".to_string(),
            fallback_locale: "en-US".to_string(),
            available_locales: vec!["zh-CN".to_string(), "en-US".to_string()],
            dock_persistence: true,
            default_page: "inbox".to_string(),
            show_command_palette: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::UiConfig;

    #[test]
    fn ui_config_deserializes_web_title() {
        let config = toml::from_str::<UiConfig>(
            r#"
web_title = { key = "web.title", fallback = "Ennoia" }
default_theme = "system"
default_locale = "zh-CN"
fallback_locale = "en-US"
available_locales = ["zh-CN", "en-US"]
dock_persistence = true
default_page = "inbox"
show_command_palette = true
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
dock_persistence = true
default_page = "inbox"
show_command_palette = true
"#,
        )
        .expect_err("shell_title should no longer deserialize");

        assert!(error.to_string().contains("missing field `web_title`"));
    }
}

/// AgentConfig represents one file under `config/agents/*.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub system_prompt: String,
    #[serde(default)]
    pub provider_id: String,
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
}

/// SkillConfig represents one skill descriptor under a registered skill package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default = "default_skill_source")]
    pub source: String,
    #[serde(default)]
    pub entry: String,
    #[serde(default)]
    pub docs: Option<String>,
    #[serde(default)]
    pub keywords: Vec<String>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// ExtensionRegistryFile stores extension package registration records under `config/extensions.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRegistryFile {
    #[serde(default)]
    pub extensions: Vec<ExtensionRegistryEntry>,
}

/// ExtensionRegistryEntry records one extension source and the user's lifecycle intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRegistryEntry {
    pub id: String,
    #[serde(default = "default_registry_source")]
    pub source: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub removed: bool,
    pub path: String,
}

/// SkillRegistryFile stores skill package registration records under `config/skills.toml`.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRegistryFile {
    #[serde(default)]
    pub skills: Vec<SkillRegistryEntry>,
}

/// SkillRegistryEntry records one skill package source and the user's lifecycle intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillRegistryEntry {
    pub id: String,
    #[serde(default = "default_registry_source")]
    pub source: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub removed: bool,
    pub path: String,
}

/// ProviderConfig represents one file under `config/providers/*.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    pub id: String,
    pub display_name: String,
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub api_key_env: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub available_models: Vec<String>,
    #[serde(default)]
    pub model_discovery: ProviderModelDiscoveryConfig,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderModelDiscoveryConfig {
    #[serde(default = "default_model_discovery_mode")]
    pub mode: String,
    #[serde(default = "default_enabled")]
    pub manual_allowed: bool,
}

impl Default for ProviderModelDiscoveryConfig {
    fn default() -> Self {
        Self {
            mode: default_model_discovery_mode(),
            manual_allowed: true,
        }
    }
}

fn default_agent_kind() -> String {
    "agent".to_string()
}

fn default_skill_source() -> String {
    "builtin".to_string()
}

fn default_registry_source() -> String {
    "builtin".to_string()
}

fn default_model_discovery_mode() -> String {
    "manual".to_string()
}

fn default_enabled() -> bool {
    true
}
