use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use ennoia_auth::{AuthService, SqliteApiKeyStore, SqliteSessionStore, SqliteUserStore};
use ennoia_config::SqliteConfigStore;
use ennoia_extension_host::{ExtensionRegistry, RegisteredExtension};
use ennoia_kernel::{
    AgentConfig, ApiKeyStore, AppConfig, CommandContribution, ContributionSet, ExtensionKind,
    ExtensionManifest, GatePipeline, HookContribution, MemoryStore, PageContribution,
    PanelContribution, PlatformOverview, ProviderContribution, RuntimeStore, SchedulerStore,
    ServerConfig, SessionStore, SpaceSpec, StageMachine, ThemeContribution, UiConfig, UserStore,
};
use ennoia_memory::SqliteMemoryStore;
use ennoia_observability::{self, ObservabilityGuard};
use ennoia_orchestrator::OrchestratorService;
use ennoia_paths::{default_home_dir, RuntimePaths};
use ennoia_policy::PolicySet;
use ennoia_runtime::{builtin_pipeline, PolicyStageMachine, SqliteRuntimeStore};
use ennoia_scheduler::{SqliteSchedulerStore, Worker};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing::info;

use crate::db;
use crate::routes::build_router;
use crate::system_config::SystemConfigRuntime;

type AppError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub server_config: ServerConfig,
    pub ui_config: UiConfig,
    pub overview: PlatformOverview,
    pub runtime_paths: Arc<RuntimePaths>,
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
    pub system_config: SystemConfigRuntime,
    pub user_store: Arc<dyn UserStore>,
    pub session_store: Arc<dyn SessionStore>,
    pub api_key_store: Arc<dyn ApiKeyStore>,
    pub auth_service: AuthService,
    pub observability_guard: Option<Arc<ObservabilityGuard>>,
}

pub fn default_app_state() -> AppState {
    let runtime_paths = Arc::new(RuntimePaths::new(default_home_dir()));
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
    let stage_machine: Arc<dyn StageMachine> =
        Arc::new(PolicyStageMachine::new(Arc::new(policies.stage.clone())));
    let gate_pipeline = builtin_pipeline();
    let orchestrator = OrchestratorService::new(stage_machine.clone(), gate_pipeline.clone());
    let config_store = Arc::new(SqliteConfigStore::new(pool.clone()));
    let system_config = SystemConfigRuntime::defaulted(config_store);
    let user_store: Arc<dyn UserStore> = Arc::new(SqliteUserStore::new(pool.clone()));
    let session_store: Arc<dyn SessionStore> = Arc::new(SqliteSessionStore::new(pool.clone()));
    let api_key_store: Arc<dyn ApiKeyStore> = Arc::new(SqliteApiKeyStore::new(pool.clone()));
    let auth_service = AuthService::new(
        user_store.clone(),
        session_store.clone(),
        api_key_store.clone(),
    );

    AppState {
        app_config: AppConfig::default(),
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        runtime_paths: runtime_paths.clone(),
        pool,
        extensions: fallback_extension_registry(&runtime_paths),
        agents: default_agents(&runtime_paths),
        spaces: default_spaces(),
        policies,
        memory_store,
        runtime_store,
        scheduler_store,
        stage_machine,
        gate_pipeline,
        orchestrator,
        system_config,
        user_store,
        session_store,
        api_key_store,
        auth_service,
        observability_guard: None,
    }
}

pub async fn bootstrap_app_state(home_dir: impl AsRef<Path>) -> Result<AppState, AppError> {
    let runtime_paths = Arc::new(RuntimePaths::new(home_dir.as_ref().to_path_buf()));
    runtime_paths.ensure_layout()?;

    let app_config: AppConfig = read_toml_or_default(runtime_paths.app_config_file())?;
    let server_config: ServerConfig = read_toml_or_default(runtime_paths.server_config_file())?;
    let ui_config: UiConfig = read_toml_or_default(runtime_paths.ui_config_file())?;
    let observability_guard = Some(Arc::new(ennoia_observability::init(
        "server",
        &server_config.log_level,
        runtime_paths.server_logs_dir(),
    )?));
    info!(home = %runtime_paths.home().display(), "bootstrapping app state");

    let agents = load_agent_configs(&runtime_paths)?;
    let spaces = default_spaces();
    let extensions = load_enabled_extensions(&runtime_paths)?;
    let policies = Arc::new(PolicySet::load(runtime_paths.policies_dir())?);

    let database_path = runtime_paths.sqlite_db();
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
    let stage_machine: Arc<dyn StageMachine> =
        Arc::new(PolicyStageMachine::new(Arc::new(policies.stage.clone())));
    let gate_pipeline = builtin_pipeline();
    let orchestrator = OrchestratorService::new(stage_machine.clone(), gate_pipeline.clone());
    let config_store = Arc::new(SqliteConfigStore::new(pool.clone()));
    let system_config = SystemConfigRuntime::defaulted(config_store);
    system_config.load_from_store().await?;
    let user_store: Arc<dyn UserStore> = Arc::new(SqliteUserStore::new(pool.clone()));
    let session_store: Arc<dyn SessionStore> = Arc::new(SqliteSessionStore::new(pool.clone()));
    let api_key_store: Arc<dyn ApiKeyStore> = Arc::new(SqliteApiKeyStore::new(pool.clone()));
    let auth_service = AuthService::new(
        user_store.clone(),
        session_store.clone(),
        api_key_store.clone(),
    );

    Ok(AppState {
        app_config,
        server_config,
        ui_config,
        overview: PlatformOverview::default(),
        runtime_paths,
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
        system_config,
        user_store,
        session_store,
        api_key_store,
        auth_service,
        observability_guard,
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

fn load_agent_configs(paths: &RuntimePaths) -> Result<Vec<AgentConfig>, AppError> {
    let config_dir = paths.agents_config_dir();
    if !config_dir.exists() {
        return Ok(default_agents(paths));
    }

    let mut agents = Vec::new();
    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let config: AgentConfig = toml::from_str(&contents)?;
        agents.push(config);
    }

    if agents.is_empty() {
        Ok(default_agents(paths))
    } else {
        Ok(agents)
    }
}

fn load_enabled_extensions(paths: &RuntimePaths) -> Result<ExtensionRegistry, AppError> {
    #[derive(serde::Deserialize)]
    struct ExtensionConfigFile {
        enabled: bool,
        install_dir: String,
    }

    let config_dir = paths.extensions_config_dir();
    if !config_dir.exists() {
        return Ok(ExtensionRegistry::new(vec![sample_observatory_manifest()]));
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(config_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let config: ExtensionConfigFile = toml::from_str(&contents)?;
        if !config.enabled {
            continue;
        }

        let install_dir = paths.expand_home_token(&config.install_dir);
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
        return Ok(fallback_extension_registry(paths));
    }

    Ok(ExtensionRegistry::from_registered(items))
}

fn default_spaces() -> Vec<SpaceSpec> {
    vec![SpaceSpec {
        id: "studio".to_string(),
        display_name: "Studio".to_string(),
        mention_policy: "configured".to_string(),
        default_agents: vec!["coder".to_string(), "planner".to_string()],
    }]
}

fn default_agents(paths: &RuntimePaths) -> Vec<AgentConfig> {
    vec![
        AgentConfig {
            id: "coder".to_string(),
            display_name: "Coder".to_string(),
            kind: "agent".to_string(),
            workspace_mode: "private".to_string(),
            default_model: "gpt-5.4".to_string(),
            skills_dir: paths.display_with_home_token(paths.agent_skills_dir("coder")),
            workspace_dir: paths.display_with_home_token(paths.agent_workspace_dir("coder")),
            artifacts_dir: paths.display_with_home_token(paths.agent_artifacts_dir("coder")),
        },
        AgentConfig {
            id: "planner".to_string(),
            display_name: "Planner".to_string(),
            kind: "agent".to_string(),
            workspace_mode: "private".to_string(),
            default_model: "gpt-5.4".to_string(),
            skills_dir: paths.display_with_home_token(paths.agent_skills_dir("planner")),
            workspace_dir: paths.display_with_home_token(paths.agent_workspace_dir("planner")),
            artifacts_dir: paths.display_with_home_token(paths.agent_artifacts_dir("planner")),
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

fn fallback_extension_registry(paths: &RuntimePaths) -> ExtensionRegistry {
    ExtensionRegistry::from_registered(vec![RegisteredExtension {
        install_dir: paths.display_with_home_token(paths.global_extension_dir("observatory")),
        manifest: sample_observatory_manifest(),
    }])
}
