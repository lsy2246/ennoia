use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, AppConfig, ContributionSet, ExtensionKind, ExtensionManifest, PageContribution,
    PlatformOverview, ServerConfig, SpaceSpec, UiConfig,
};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::SqlitePool;
use tokio::net::TcpListener;

use crate::db;
use crate::routes::build_router;

type AppError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub server_config: ServerConfig,
    pub ui_config: UiConfig,
    pub overview: PlatformOverview,
    pub home_dir: Arc<PathBuf>,
    pub pool: SqlitePool,
    pub extensions: ExtensionRegistry,
    pub agents: Vec<AgentConfig>,
    pub spaces: Vec<SpaceSpec>,
}

pub fn default_app_state() -> AppState {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_lazy("sqlite::memory:")
        .expect("memory pool");

    AppState {
        app_config: AppConfig::default(),
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        home_dir: Arc::new(default_home_dir()),
        pool,
        extensions: ExtensionRegistry::new(vec![sample_observatory_manifest()]),
        agents: vec![
            AgentConfig {
                id: "coder".to_string(),
                display_name: "Coder".to_string(),
                kind: "agent".to_string(),
                workspace_mode: "private".to_string(),
                default_model: "gpt-5.4".to_string(),
                skills_dir: "~/.ennoia/agents/coder/skills".to_string(),
                workspace_dir: "~/.ennoia/agents/coder/workspace".to_string(),
                artifacts_dir: "~/.ennoia/agents/coder/artifacts".to_string(),
            },
            AgentConfig {
                id: "planner".to_string(),
                display_name: "Planner".to_string(),
                kind: "agent".to_string(),
                workspace_mode: "private".to_string(),
                default_model: "gpt-5.4".to_string(),
                skills_dir: "~/.ennoia/agents/planner/skills".to_string(),
                workspace_dir: "~/.ennoia/agents/planner/workspace".to_string(),
                artifacts_dir: "~/.ennoia/agents/planner/artifacts".to_string(),
            },
        ],
        spaces: default_spaces(),
    }
}

pub async fn bootstrap_app_state(home_dir: impl AsRef<Path>) -> Result<AppState, AppError> {
    let home_dir = home_dir.as_ref().to_path_buf();
    ensure_runtime_layout(&home_dir)?;

    let app_config: AppConfig = read_toml_or_default(home_dir.join("config/ennoia.toml"))?;
    let server_config: ServerConfig = read_toml_or_default(home_dir.join("config/server.toml"))?;
    let ui_config: UiConfig = read_toml_or_default(home_dir.join("config/ui.toml"))?;
    let agents = load_agent_configs(home_dir.join("config/agents"))?;
    let spaces = default_spaces();
    let extensions = load_enabled_extensions(home_dir.join("config/extensions"))?;

    let database_path = home_dir.join("state/sqlite/ennoia.db");
    let database_url = format!("sqlite://{}", database_path.display());
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    db::initialize_schema(&pool).await?;
    db::upsert_agents(&pool, &agents).await?;
    db::upsert_spaces(&pool, &spaces).await?;
    db::upsert_extensions(&pool, &extensions).await?;

    Ok(AppState {
        app_config,
        server_config,
        ui_config,
        overview: PlatformOverview::default(),
        home_dir: Arc::new(home_dir),
        pool,
        extensions,
        agents,
        spaces,
    })
}

pub async fn run_server(home_dir: impl AsRef<Path>) -> Result<(), AppError> {
    let state = bootstrap_app_state(home_dir).await?;
    let address = format!("{}:{}", state.server_config.host, state.server_config.port);
    let listener = TcpListener::bind(&address).await?;
    axum::serve(listener, build_router(state)).await?;
    Ok(())
}

fn read_toml_or_default<T>(path: PathBuf) -> Result<T, AppError>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

fn load_agent_configs(path: PathBuf) -> Result<Vec<AgentConfig>, AppError> {
    if !path.exists() {
        return Ok(default_app_state().agents);
    }

    let mut agents = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let config: AgentConfig = toml::from_str(&contents)?;
        agents.push(config);
    }

    if agents.is_empty() {
        Ok(default_app_state().agents)
    } else {
        Ok(agents)
    }
}

fn load_enabled_extensions(path: PathBuf) -> Result<ExtensionRegistry, AppError> {
    #[derive(serde::Deserialize)]
    struct ExtensionConfigFile {
        enabled: bool,
        install_dir: String,
    }

    if !path.exists() {
        return Ok(ExtensionRegistry::new(vec![sample_observatory_manifest()]));
    }

    let mut manifests = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let config: ExtensionConfigFile = toml::from_str(&contents)?;
        if !config.enabled {
            continue;
        }

        let manifest_path = expand_home_dir(&config.install_dir).join("manifest.toml");
        if manifest_path.exists() {
            let manifest_contents = fs::read_to_string(manifest_path)?;
            let manifest: ExtensionManifest = toml::from_str(&manifest_contents)?;
            manifests.push(manifest);
        }
    }

    if manifests.is_empty() {
        manifests.push(sample_observatory_manifest());
    }

    Ok(ExtensionRegistry::new(manifests))
}

fn ensure_runtime_layout(home_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(home_dir.join("config/agents"))?;
    fs::create_dir_all(home_dir.join("config/extensions"))?;
    fs::create_dir_all(home_dir.join("state/queue"))?;
    fs::create_dir_all(home_dir.join("state/runs"))?;
    fs::create_dir_all(home_dir.join("state/cache"))?;
    fs::create_dir_all(home_dir.join("state/sqlite"))?;
    fs::create_dir_all(home_dir.join("global/extensions"))?;
    fs::create_dir_all(home_dir.join("global/skills"))?;
    fs::create_dir_all(home_dir.join("agents"))?;
    fs::create_dir_all(home_dir.join("spaces"))?;
    fs::create_dir_all(home_dir.join("logs"))?;
    Ok(())
}

fn default_spaces() -> Vec<SpaceSpec> {
    vec![SpaceSpec {
        id: "studio".to_string(),
        display_name: "Studio".to_string(),
        mention_policy: "configured".to_string(),
        default_agents: vec!["coder".to_string(), "planner".to_string()],
    }]
}

fn sample_observatory_manifest() -> ExtensionManifest {
    ExtensionManifest {
        id: "observatory".to_string(),
        kind: ExtensionKind::SystemExtension,
        version: "0.1.0".to_string(),
        frontend_bundle: Some("frontend/index.js".to_string()),
        backend_entry: Some("backend/index.js".to_string()),
        contributes: ContributionSet {
            pages: vec![PageContribution {
                id: "observatory.events".to_string(),
                title: "Observatory".to_string(),
                route: "/observatory".to_string(),
            }],
            ..ContributionSet::default()
        },
    }
}

fn default_home_dir() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ennoia")
}

fn expand_home_dir(value: &str) -> PathBuf {
    if let Some(rest) = value.strip_prefix("~/.ennoia") {
        return default_home_dir().join(rest.trim_start_matches('/'));
    }
    PathBuf::from(value)
}
