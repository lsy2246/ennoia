use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ennoia_extension_host::{ExtensionRegistry, RegisteredExtension};
use ennoia_kernel::{
    AgentConfig, AppConfig, CommandContribution, ContributionSet, ExtensionKind, ExtensionManifest,
    GatePipeline, HookContribution, MemoryStore, PageContribution, PanelContribution,
    PlatformOverview, ProviderContribution, RuntimeStore, SchedulerStore, ServerConfig, SpaceSpec,
    StageMachine, ThemeContribution, UiConfig,
};
use ennoia_memory::SqliteMemoryStore;
use ennoia_policy::PolicySet;
use ennoia_runtime::{builtin_pipeline, PolicyStageMachine, SqliteRuntimeStore};
use ennoia_scheduler::{SqliteSchedulerStore, Worker};
use ennoia_orchestrator::OrchestratorService;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::net::TcpListener;

use crate::db;
use crate::routes::build_router;

type AppError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
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
    pub policies: Arc<PolicySet>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub runtime_store: Arc<dyn RuntimeStore>,
    pub scheduler_store: Arc<dyn SchedulerStore>,
    pub stage_machine: Arc<dyn StageMachine>,
    pub gate_pipeline: GatePipeline,
    pub orchestrator: OrchestratorService,
}

pub fn default_app_state() -> AppState {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect_lazy("sqlite::memory:")
        .expect("memory pool");

    let policies = Arc::new(PolicySet::builtin());
    let memory_store: Arc<dyn MemoryStore> = Arc::new(SqliteMemoryStore::new(
        pool.clone(),
        Arc::new(policies.memory.clone()),
    ));
    let runtime_store: Arc<dyn RuntimeStore> = Arc::new(SqliteRuntimeStore::new(pool.clone()));
    let scheduler_store: Arc<dyn SchedulerStore> =
        Arc::new(SqliteSchedulerStore::new(pool.clone()));
    let stage_machine: Arc<dyn StageMachine> = Arc::new(PolicyStageMachine::new(Arc::new(
        policies.stage.clone(),
    )));
    let gate_pipeline = builtin_pipeline();
    let orchestrator = OrchestratorService::new(stage_machine.clone(), gate_pipeline.clone());

    AppState {
        app_config: AppConfig::default(),
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        home_dir: Arc::new(default_home_dir()),
        pool,
        extensions: fallback_extension_registry(),
        agents: default_agents(),
        spaces: default_spaces(),
        policies,
        memory_store,
        runtime_store,
        scheduler_store,
        stage_machine,
        gate_pipeline,
        orchestrator,
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
    let policies = Arc::new(PolicySet::load(home_dir.join("policies"))?);

    let database_path = home_dir.join("state/sqlite/ennoia.db");
    let connect_options = SqliteConnectOptions::new()
        .filename(&database_path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    db::initialize_schema(&pool).await?;
    db::upsert_agents(&pool, &agents).await?;
    db::upsert_spaces(&pool, &spaces).await?;
    db::upsert_extensions(&pool, &extensions).await?;

    let memory_store: Arc<dyn MemoryStore> = Arc::new(SqliteMemoryStore::new(
        pool.clone(),
        Arc::new(policies.memory.clone()),
    ));
    let runtime_store: Arc<dyn RuntimeStore> = Arc::new(SqliteRuntimeStore::new(pool.clone()));
    let scheduler_store: Arc<dyn SchedulerStore> =
        Arc::new(SqliteSchedulerStore::new(pool.clone()));
    let stage_machine: Arc<dyn StageMachine> = Arc::new(PolicyStageMachine::new(Arc::new(
        policies.stage.clone(),
    )));
    let gate_pipeline = builtin_pipeline();
    let orchestrator = OrchestratorService::new(stage_machine.clone(), gate_pipeline.clone());

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
        policies,
        memory_store,
        runtime_store,
        scheduler_store,
        stage_machine,
        gate_pipeline,
        orchestrator,
    })
}

pub async fn run_server(home_dir: impl AsRef<Path>) -> Result<(), AppError> {
    let state = bootstrap_app_state(home_dir).await?;

    let scheduler_store = state.scheduler_store.clone();
    let tick_ms = state.app_config.scheduler_tick_ms;
    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let worker = Worker::new(scheduler_store, tick_ms);
    tokio::spawn(async move {
        worker.run_forever(cancel_rx).await;
    });

    let address = format!("{}:{}", state.server_config.host, state.server_config.port);
    let listener = TcpListener::bind(&address).await?;
    let serve_result = axum::serve(listener, build_router(state)).await;
    let _ = cancel_tx.send(true);
    serve_result?;
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
        return Ok(default_agents());
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
        Ok(default_agents())
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

    let mut items = Vec::new();
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

        let install_dir = expand_home_dir(&config.install_dir);
        let manifest_path = install_dir.join("manifest.toml");
        if manifest_path.exists() {
            let manifest_contents = fs::read_to_string(manifest_path)?;
            let manifest: ExtensionManifest = toml::from_str(&manifest_contents)?;
            items.push(RegisteredExtension {
                manifest,
                install_dir: install_dir.display().to_string(),
            });
        }
    }

    if items.is_empty() {
        return Ok(fallback_extension_registry());
    }

    Ok(ExtensionRegistry::from_registered(items))
}

fn ensure_runtime_layout(home_dir: &Path) -> Result<(), AppError> {
    fs::create_dir_all(home_dir.join("config/agents"))?;
    fs::create_dir_all(home_dir.join("config/extensions"))?;
    fs::create_dir_all(home_dir.join("policies"))?;
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

fn default_agents() -> Vec<AgentConfig> {
    vec![
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
    ]
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
                mount: "observatory.events.page".to_string(),
                icon: Some("activity".to_string()),
            }],
            panels: vec![PanelContribution {
                id: "observatory.timeline".to_string(),
                title: "Event Timeline".to_string(),
                mount: "observatory.timeline.panel".to_string(),
                slot: "right".to_string(),
                icon: Some("panel-right".to_string()),
            }],
            themes: vec![ThemeContribution {
                id: "observatory.daybreak".to_string(),
                label: "Daybreak".to_string(),
                entry: Some("frontend/themes/daybreak.css".to_string()),
            }],
            commands: vec![CommandContribution {
                id: "observatory.open".to_string(),
                title: "Open Observatory".to_string(),
                action: "open-page".to_string(),
                shortcut: Some("Ctrl+Shift+O".to_string()),
            }],
            providers: vec![ProviderContribution {
                id: "observatory.feed".to_string(),
                kind: "activity-feed".to_string(),
                entry: Some("backend/providers/activity-feed.js".to_string()),
            }],
            hooks: vec![HookContribution {
                event: "run.completed".to_string(),
                handler: Some("backend/hooks/run-completed.js".to_string()),
            }],
            ..ContributionSet::default()
        },
    }
}

fn fallback_extension_registry() -> ExtensionRegistry {
    ExtensionRegistry::from_registered(vec![RegisteredExtension {
        install_dir: "~/.ennoia/global/extensions/observatory".to_string(),
        manifest: sample_observatory_manifest(),
    }])
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
