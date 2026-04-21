use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use ennoia_assets::builtins;
use ennoia_config::SqliteConfigStore;
use ennoia_extension_host::{ExtensionRuntime, ExtensionRuntimeConfig};
use ennoia_kernel::{
    AgentConfig, AppConfig, PlatformOverview, ProviderConfig, ServerConfig, SkillConfig,
    SkillRegistryEntry, SkillRegistryFile, SpaceSpec, UiConfig,
};
use ennoia_memory::{MemoryStore, SqliteMemoryStore};
use ennoia_observability::{self, ObservabilityGuard};
use ennoia_orchestrator::OrchestratorService;
use ennoia_paths::{default_home_dir, RuntimePaths};
use ennoia_policy::PolicySet;
use ennoia_runtime::{
    builtin_pipeline, GatePipeline, PolicyStageMachine, RuntimeStore, SqliteRuntimeStore,
    StageMachine,
};
use ennoia_scheduler::{SchedulerStore, SqliteSchedulerStore, Worker};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use tracing::info;

use crate::db;
use crate::routes::build_router;
use crate::system_config::SystemConfigRuntime;

type AppError = Box<dyn std::error::Error + Send + Sync>;

const OBSERVABILITY_TARGET: &str = "server";
const DEFAULT_SPACE_ID: &str = "studio";
const DEFAULT_SPACE_NAME: &str = "Studio";
const DEFAULT_REASONING_EFFORT: &str = "high";
const EXTENSION_REFRESH_SUMMARY: &str = "polled runtime refresh";

#[derive(Clone)]
pub struct AppState {
    pub app_config: AppConfig,
    pub server_config: ServerConfig,
    pub ui_config: UiConfig,
    pub overview: PlatformOverview,
    pub runtime_paths: Arc<RuntimePaths>,
    pub pool: SqlitePool,
    pub extensions: ExtensionRuntime,
    pub agents: Vec<AgentConfig>,
    pub skills: Vec<SkillConfig>,
    pub providers: Vec<ProviderConfig>,
    pub spaces: Vec<SpaceSpec>,
    pub policies: Arc<PolicySet>,
    pub memory_store: Arc<dyn MemoryStore>,
    pub runtime_store: Arc<dyn RuntimeStore>,
    pub scheduler_store: Arc<dyn SchedulerStore>,
    pub stage_machine: Arc<dyn StageMachine>,
    pub gate_pipeline: GatePipeline,
    pub orchestrator: OrchestratorService,
    pub system_config: SystemConfigRuntime,
    pub observability_guard: Option<Arc<ObservabilityGuard>>,
}

pub fn default_app_state() -> AppState {
    let bootstrap_paths = RuntimePaths::new(default_home_dir());
    let app_config = normalize_app_config(&bootstrap_paths, AppConfig::default());
    let runtime_paths = Arc::new(bootstrap_paths.clone());
    runtime_paths.ensure_layout().expect("runtime layout");
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
    let extensions =
        ExtensionRuntime::bootstrap(extension_runtime_config(&runtime_paths)).expect("runtime");

    AppState {
        app_config,
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        runtime_paths: runtime_paths.clone(),
        pool,
        extensions,
        agents: Vec::new(),
        skills: builtin_skill_configs(),
        providers: Vec::new(),
        spaces: default_spaces(),
        policies,
        memory_store,
        runtime_store,
        scheduler_store,
        stage_machine,
        gate_pipeline,
        orchestrator,
        system_config,
        observability_guard: None,
    }
}

pub async fn bootstrap_app_state(home_dir: impl AsRef<Path>) -> Result<AppState, AppError> {
    let bootstrap_paths = RuntimePaths::new(home_dir.as_ref().to_path_buf());
    bootstrap_paths.ensure_layout()?;

    let app_config = normalize_app_config(
        &bootstrap_paths,
        read_toml_or_default(bootstrap_paths.app_config_file())?,
    );
    let runtime_paths = Arc::new(bootstrap_paths);
    runtime_paths.ensure_layout()?;
    let server_config: ServerConfig = read_toml_or_default(runtime_paths.server_config_file())?;
    let ui_config: UiConfig = read_toml_or_default(runtime_paths.ui_config_file())?;
    let observability_guard = Some(Arc::new(ennoia_observability::init(
        OBSERVABILITY_TARGET,
        &server_config.log_level,
        runtime_paths.server_logs_dir(),
    )?));
    info!(home = %runtime_paths.home().display(), "bootstrapping app state");

    let agents = load_agent_configs(&runtime_paths)?;
    let skills = load_skill_configs(&runtime_paths)?;
    let providers = load_provider_configs(&runtime_paths)?;
    let spaces = default_spaces();
    let extensions = ExtensionRuntime::bootstrap(extension_runtime_config(&runtime_paths))?;
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
    db::upsert_extensions_runtime(&pool, &extensions.snapshot()).await?;

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

    Ok(AppState {
        app_config,
        server_config,
        ui_config,
        overview: PlatformOverview::default(),
        runtime_paths,
        pool,
        extensions,
        agents,
        skills,
        providers,
        spaces,
        policies,
        memory_store,
        runtime_store,
        scheduler_store,
        stage_machine,
        gate_pipeline,
        orchestrator,
        system_config,
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

    let extensions = state.extensions.clone();
    let pool = state.pool.clone();
    let mut extension_cancel = cancel_tx.subscribe();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {
                _ = interval.tick() => {
                    if let Ok(Some(snapshot)) = extensions.refresh_from_disk(EXTENSION_REFRESH_SUMMARY) {
                        let _ = db::upsert_extensions_runtime(&pool, &snapshot).await;
                    }
                }
                changed = extension_cancel.changed() => {
                    if changed.is_err() || *extension_cancel.borrow() {
                        break;
                    }
                }
            }
        }
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

pub fn load_agent_configs(paths: &RuntimePaths) -> Result<Vec<AgentConfig>, AppError> {
    let mut agents = load_configs_from_dir::<AgentConfig>(paths.agents_config_dir())?;
    for agent in &mut agents {
        normalize_agent_config(paths, agent);
    }
    Ok(agents)
}

pub fn load_skill_configs(paths: &RuntimePaths) -> Result<Vec<SkillConfig>, AppError> {
    let mut skills = load_skill_registry(paths)?
        .skills
        .into_iter()
        .filter(|entry| entry.enabled && !entry.removed)
        .filter_map(|entry| load_skill_from_registry_entry(paths, &entry).ok())
        .collect::<Vec<_>>();
    if skills.is_empty() {
        skills = builtin_skill_configs();
    }
    skills.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(skills)
}

pub fn load_skill_registry(paths: &RuntimePaths) -> Result<SkillRegistryFile, AppError> {
    read_toml_or_default(paths.skills_registry_file())
}

pub fn write_skill_registry(
    paths: &RuntimePaths,
    registry: &SkillRegistryFile,
) -> Result<(), AppError> {
    if let Some(parent) = paths.skills_registry_file().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        paths.skills_registry_file(),
        toml::to_string_pretty(registry)?,
    )?;
    Ok(())
}

pub fn load_provider_configs(paths: &RuntimePaths) -> Result<Vec<ProviderConfig>, AppError> {
    let mut providers = load_configs_from_dir::<ProviderConfig>(paths.providers_config_dir())?;
    providers.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(providers)
}

pub fn write_config_to_dir<T: serde::Serialize>(
    dir: PathBuf,
    id: &str,
    value: &T,
) -> Result<(), AppError> {
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{id}.toml"));
    fs::write(path, toml::to_string_pretty(value)?)?;
    Ok(())
}

pub fn delete_config_from_dir(dir: PathBuf, id: &str) -> Result<bool, AppError> {
    let path = dir.join(format!("{id}.toml"));
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path)?;
    Ok(true)
}

fn load_configs_from_dir<T>(dir: PathBuf) -> Result<Vec<T>, AppError>
where
    T: serde::de::DeserializeOwned,
{
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        let item: T = toml::from_str(&contents)?;
        items.push(item);
    }
    Ok(items)
}

fn default_spaces() -> Vec<SpaceSpec> {
    vec![SpaceSpec {
        id: DEFAULT_SPACE_ID.to_string(),
        display_name: DEFAULT_SPACE_NAME.to_string(),
        description: "默认工作台空间".to_string(),
        primary_goal: "组织单操作者与多 Agent 的日常协作".to_string(),
        mention_policy: "configured".to_string(),
        default_agents: Vec::new(),
    }]
}

fn normalize_agent_config(paths: &RuntimePaths, agent: &mut AgentConfig) {
    if agent.model_id.is_empty() && !agent.default_model.is_empty() {
        agent.model_id = agent.default_model.clone();
    }
    if agent.default_model.is_empty() && !agent.model_id.is_empty() {
        agent.default_model = agent.model_id.clone();
    }
    if agent.reasoning_effort.is_empty() {
        agent.reasoning_effort = DEFAULT_REASONING_EFFORT.to_string();
    }
    if !agent.working_dir.is_empty() {
        agent.working_dir = paths.display_for_user(paths.expand_home_token(&agent.working_dir));
    } else {
        agent.working_dir = paths.display_for_user(paths.agent_working_dir(&agent.id));
    }
    if !agent.skills_dir.is_empty() {
        agent.skills_dir = paths.display_for_user(paths.expand_home_token(&agent.skills_dir));
    } else {
        agent.skills_dir = paths.display_for_user(paths.agent_skills_dir(&agent.id));
    }
    if !agent.artifacts_dir.is_empty() {
        agent.artifacts_dir = paths.display_for_user(paths.expand_home_token(&agent.artifacts_dir));
    } else {
        agent.artifacts_dir = paths.display_for_user(paths.agent_artifacts_dir(&agent.id));
    }
}

fn extension_runtime_config(paths: &RuntimePaths) -> ExtensionRuntimeConfig {
    ExtensionRuntimeConfig {
        registry_file: paths.extensions_registry_file(),
        logs_dir: paths.extensions_logs_dir(),
    }
}

pub fn normalize_app_config(paths: &RuntimePaths, mut config: AppConfig) -> AppConfig {
    if let Some(database_path) = config.database_url.strip_prefix("sqlite://") {
        config.database_url = format!(
            "sqlite://{}",
            paths.display_for_user(paths.expand_home_token(database_path)),
        );
    }

    config
}

fn builtin_skill_configs() -> Vec<SkillConfig> {
    let mut skills = builtins::skills()
        .into_iter()
        .filter(|asset| asset.logical_path.ends_with("/skill.toml"))
        .filter_map(|asset| toml::from_str::<SkillConfig>(asset.contents).ok())
        .collect::<Vec<_>>();
    skills.sort_by(|left, right| left.id.cmp(&right.id));
    skills
}

fn load_skill_from_registry_entry(
    paths: &RuntimePaths,
    entry: &SkillRegistryEntry,
) -> Result<SkillConfig, AppError> {
    let descriptor_path = resolve_skill_descriptor_path(paths, entry);
    let mut skill = toml::from_str::<SkillConfig>(&fs::read_to_string(descriptor_path)?)?;
    if skill.id.is_empty() {
        skill.id = entry.id.clone();
    }
    skill.source = entry.source.clone();
    skill.enabled = entry.enabled;
    Ok(skill)
}

fn resolve_skill_descriptor_path(paths: &RuntimePaths, entry: &SkillRegistryEntry) -> PathBuf {
    let root = paths.expand_home_token(&entry.path);
    if root.extension().and_then(|value| value.to_str()) == Some("toml") {
        root
    } else {
        root.join("skill.toml")
    }
}

pub fn upsert_skill_package(paths: &RuntimePaths, payload: &SkillConfig) -> Result<(), AppError> {
    let skill_dir = paths.skill_dir(&payload.id);
    fs::create_dir_all(&skill_dir)?;
    fs::write(
        skill_dir.join("skill.toml"),
        toml::to_string_pretty(payload)?,
    )?;

    let mut registry = load_skill_registry(paths)?;
    registry.skills.retain(|item| item.id != payload.id);
    registry.skills.push(SkillRegistryEntry {
        id: payload.id.clone(),
        source: payload.source.clone(),
        enabled: payload.enabled,
        removed: false,
        path: paths.display_for_user(&skill_dir),
    });
    sort_skill_registry_entries(&mut registry.skills);
    write_skill_registry(paths, &registry)
}

pub fn delete_skill_package(paths: &RuntimePaths, skill_id: &str) -> Result<bool, AppError> {
    let mut registry = load_skill_registry(paths)?;
    let Some(index) = registry.skills.iter().position(|item| item.id == skill_id) else {
        return Ok(false);
    };

    let entry = registry.skills[index].clone();
    let skill_root = paths.expand_home_token(&entry.path);
    if skill_root.exists() {
        fs::remove_dir_all(&skill_root)?;
    }

    if entry.source == "builtin" {
        registry.skills[index].enabled = false;
        registry.skills[index].removed = true;
    } else {
        registry.skills.remove(index);
    }

    sort_skill_registry_entries(&mut registry.skills);
    write_skill_registry(paths, &registry)?;
    Ok(true)
}

fn sort_skill_registry_entries(entries: &mut [SkillRegistryEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.path.cmp(&right.path))
    });
}
