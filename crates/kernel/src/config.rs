use serde::{Deserialize, Serialize};

use crate::system_config::default_local_dev_origins;
use crate::ui::LocalizedText;

/// AppConfig stores startup-wide settings loaded from `config/ennoia.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub app_name: String,
    pub mode: String,
    #[serde(default = "default_workspace_root")]
    pub workspace_root: String,
    pub database_mode: String,
    pub database_url: String,
    pub extensions_scan_dir: String,
    pub agents_scan_dir: String,
    pub scheduler_tick_ms: u64,
    pub default_mention_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app_name: "ennoia".to_string(),
            mode: "development".to_string(),
            workspace_root: "~/.ennoia/workspace".to_string(),
            database_mode: "sqlite".to_string(),
            database_url: "sqlite://~/.ennoia/data/sqlite/ennoia.db".to_string(),
            extensions_scan_dir: "~/.ennoia/extensions".to_string(),
            agents_scan_dir: "~/.ennoia/config/agents".to_string(),
            scheduler_tick_ms: 1_000,
            default_mention_mode: "configured".to_string(),
        }
    }
}

/// ServerConfig stores network and runtime settings loaded from `config/server.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub log_level: String,
    pub allow_origins: Vec<String>,
    pub enable_ws: bool,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 3710,
            log_level: "info".to_string(),
            allow_origins: default_local_dev_origins(),
            enable_ws: true,
        }
    }
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
    #[serde(default = "default_reasoning_effort")]
    pub reasoning_effort: String,
    #[serde(default)]
    pub workspace_root: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
    #[serde(default = "default_agent_kind")]
    pub kind: String,
    #[serde(default = "default_workspace_mode")]
    pub workspace_mode: String,
    #[serde(default)]
    pub default_model: String,
    #[serde(default)]
    pub skills_dir: String,
    #[serde(default)]
    pub workspace_dir: String,
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
    pub tags: Vec<String>,
    #[serde(default = "default_agent_enabled")]
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
    #[serde(default = "default_agent_enabled")]
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
    #[serde(default = "default_agent_enabled")]
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
    #[serde(default = "default_agent_enabled")]
    pub enabled: bool,
}

fn default_agent_kind() -> String {
    "agent".to_string()
}

fn default_workspace_mode() -> String {
    "private".to_string()
}

fn default_workspace_root() -> String {
    "~/.ennoia/workspace".to_string()
}

fn default_reasoning_effort() -> String {
    "high".to_string()
}

fn default_skill_source() -> String {
    "builtin".to_string()
}

fn default_registry_source() -> String {
    "builtin".to_string()
}

fn default_agent_enabled() -> bool {
    true
}
