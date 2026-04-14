use serde::{Deserialize, Serialize};

/// AppConfig stores startup-wide settings loaded from `config/ennoia.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub app_name: String,
    pub mode: String,
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
            database_mode: "sqlite".to_string(),
            database_url: "sqlite://~/.ennoia/state/sqlite/ennoia.db".to_string(),
            extensions_scan_dir: "~/.ennoia/config/extensions".to_string(),
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
            allow_origins: vec!["http://localhost:5173".to_string()],
            enable_ws: true,
        }
    }
}

/// UiConfig stores shell-specific settings loaded from `config/ui.toml`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct UiConfig {
    pub shell_title: String,
    pub default_theme: String,
    pub dock_persistence: bool,
    pub default_page: String,
    pub show_command_palette: bool,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            shell_title: "Ennoia".to_string(),
            default_theme: "system".to_string(),
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
    pub kind: String,
    pub workspace_mode: String,
    pub default_model: String,
    pub skills_dir: String,
    pub workspace_dir: String,
    pub artifacts_dir: String,
}
