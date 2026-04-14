use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, AppConfig, OwnerKind, OwnerRef, PlatformOverview, RunSpec, RunStatus,
    ServerConfig, SpaceSpec, UiConfig,
};

/// AppState keeps the minimal in-memory state needed by the server skeleton.
#[derive(Debug, Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub server_config: ServerConfig,
    pub ui_config: UiConfig,
    pub overview: PlatformOverview,
    pub extensions: ExtensionRegistry,
    pub agents: Vec<AgentConfig>,
    pub spaces: Vec<SpaceSpec>,
    pub runs: Vec<RunSpec>,
}

/// Builds the default application state used by the CLI and tests.
pub fn default_app_state() -> AppState {
    AppState {
        app_config: AppConfig::default(),
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        extensions: ExtensionRegistry::default(),
        agents: vec![AgentConfig {
            id: "coder".to_string(),
            display_name: "Coder".to_string(),
            kind: "agent".to_string(),
            workspace_mode: "private".to_string(),
            default_model: "gpt-5.4".to_string(),
            skills_dir: "~/.ennoia/agents/coder/skills".to_string(),
            workspace_dir: "~/.ennoia/agents/coder/workspace".to_string(),
            artifacts_dir: "~/.ennoia/agents/coder/artifacts".to_string(),
        }],
        spaces: vec![SpaceSpec {
            id: "studio".to_string(),
            display_name: "Studio".to_string(),
            mention_policy: "configured".to_string(),
            default_agents: vec!["coder".to_string()],
        }],
        runs: vec![RunSpec {
            id: "run-studio-1".to_string(),
            owner: OwnerRef {
                kind: OwnerKind::Space,
                id: "studio".to_string(),
            },
            thread_id: "thread-space-studio".to_string(),
            trigger: "space_message".to_string(),
            status: RunStatus::Pending,
        }],
    }
}
