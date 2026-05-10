use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::time::Instant;

use ennoia_assets::builtins;
use ennoia_extension_host::{ExtensionRuntime, ExtensionRuntimeConfig};
use ennoia_kernel::{
    apply_server_log_env_overrides, AgentConfig, AgentDocument, AgentPermissionProfile,
    ModelEndpointConfig, PlatformOverview, ServerConfig, SkillConfig, SkillManifest,
    SkillRegistryEntry, SkillRegistryFile, SpaceSpec, UiConfig,
};
use ennoia_logs::{self, next_span_id, LogsGuard, TraceContext};
use ennoia_paths::{default_home_dir, RuntimePaths};
use tokio::net::TcpListener;
use tracing::info;

use crate::agent_permissions::AgentPermissionStore;
use crate::event_bus::EventBusStore;
use crate::extension_runtime::ExtensionRuntimeStore;
use crate::host_capabilities::ServerHostCapabilityDispatcher;
use crate::logs_store::{
    LogEntryWrite, LogTraceLinkWrite, LogTraceWrite, LogsStore, LOGS_COMPONENT_EVENT_BUS,
    LOGS_COMPONENT_EXTENSION_HOST, LOGS_COMPONENT_HOST,
};
use crate::middleware::RateLimitState;
use crate::operations::OperationStore;
use crate::realtime::RealtimeHub;
use crate::routes::{build_router, run_due_schedules_once};

type AppError = Box<dyn std::error::Error + Send + Sync>;

const LOGS_TARGET: &str = "server";
const DEFAULT_SPACE_ID: &str = "studio";
const DEFAULT_SPACE_NAME: &str = "Studio";
const EXTENSION_REFRESH_SUMMARY: &str = "polled runtime refresh";

#[derive(Clone)]
pub struct AppState {
    pub server_config: ServerConfig,
    pub ui_config: UiConfig,
    pub overview: PlatformOverview,
    pub runtime_paths: Arc<RuntimePaths>,
    pub extensions: ExtensionRuntime,
    pub agents: Vec<AgentConfig>,
    pub skills: Vec<SkillConfig>,
    pub model_endpoints: Vec<ModelEndpointConfig>,
    pub spaces: Vec<SpaceSpec>,
    pub rate_limit_state: RateLimitState,
    pub schedule_lock: Arc<tokio::sync::Mutex<()>>,
    pub logs: Arc<LogsStore>,
    pub event_bus: Arc<EventBusStore>,
    pub agent_permissions: Arc<AgentPermissionStore>,
    pub operations: Arc<OperationStore>,
    pub extension_runtime_store: Arc<ExtensionRuntimeStore>,
    pub realtime: Arc<RealtimeHub>,
    pub logs_guard: Option<Arc<LogsGuard>>,
}

pub fn default_app_state() -> AppState {
    let bootstrap_paths = RuntimePaths::new(default_home_dir());
    let runtime_paths = Arc::new(bootstrap_paths.clone());
    runtime_paths.ensure_layout().expect("runtime layout");
    let extensions = ExtensionRuntime::bootstrap(extension_runtime_config(
        &runtime_paths,
        &ServerConfig::default(),
    ))
    .expect("runtime");
    let logs = Arc::new(LogsStore::new(&runtime_paths).expect("logs"));
    let event_bus = Arc::new(EventBusStore::new(&runtime_paths).expect("event bus"));
    let agent_permissions =
        Arc::new(AgentPermissionStore::new(&runtime_paths).expect("agent permissions"));
    let operations_store = OperationStore::new(&runtime_paths).expect("operations");
    let _ = operations_store.cancel_abandoned_operations();
    let operations = Arc::new(operations_store);
    let extension_runtime_store =
        Arc::new(ExtensionRuntimeStore::new(&runtime_paths).expect("extension runtime store"));
    let realtime = Arc::new(RealtimeHub::new());

    AppState {
        server_config: ServerConfig::default(),
        ui_config: UiConfig::default(),
        overview: PlatformOverview::default(),
        runtime_paths: runtime_paths.clone(),
        extensions,
        agents: Vec::new(),
        skills: Vec::new(),
        model_endpoints: Vec::new(),
        spaces: default_spaces(),
        rate_limit_state: RateLimitState::new(),
        schedule_lock: Arc::new(tokio::sync::Mutex::new(())),
        logs,
        event_bus,
        agent_permissions,
        operations,
        extension_runtime_store,
        realtime,
        logs_guard: None,
    }
}

pub async fn bootstrap_app_state(home_dir: impl AsRef<Path>) -> Result<AppState, AppError> {
    let bootstrap_paths = RuntimePaths::new(home_dir.as_ref().to_path_buf());
    bootstrap_paths.ensure_layout()?;

    let runtime_paths = Arc::new(bootstrap_paths);
    runtime_paths.ensure_layout()?;
    let mut server_config: ServerConfig = read_toml_or_default(runtime_paths.server_config_file())?;
    server_config = server_config.normalize();
    apply_server_log_env_overrides(&mut server_config.logging);
    let ui_config: UiConfig = read_toml_or_default(runtime_paths.ui_config_file())?;
    let ui_config = ui_config.normalize();
    let logs_guard = Some(Arc::new(ennoia_logs::init(
        LOGS_TARGET,
        &server_config.logging.level,
        runtime_paths.server_logs_dir(),
    )?));
    info!(home = %runtime_paths.home().display(), "bootstrapping app state");

    let agents = load_agent_configs(&runtime_paths)?;
    let skills = load_skill_configs(&runtime_paths)?;
    let model_endpoints = load_model_endpoint_configs(&runtime_paths)?;
    let spaces = default_spaces();
    let extensions =
        ExtensionRuntime::bootstrap(extension_runtime_config(&runtime_paths, &server_config))?;
    let logs = Arc::new(LogsStore::new(&runtime_paths)?);
    let event_bus = Arc::new(EventBusStore::new(&runtime_paths)?);
    let agent_permissions = Arc::new(AgentPermissionStore::new(&runtime_paths)?);
    let operations_store = OperationStore::new(&runtime_paths)?;
    let recovered_operations = operations_store.cancel_abandoned_operations()?;
    let operations = Arc::new(operations_store);
    let extension_runtime_store = Arc::new(ExtensionRuntimeStore::new(&runtime_paths)?);
    let realtime = Arc::new(RealtimeHub::new());

    if !recovered_operations.is_empty() {
        let recovered_ids = recovered_operations
            .iter()
            .map(|operation| operation.id.clone())
            .collect::<Vec<_>>();
        let _ = logs.append_log(LogEntryWrite {
            event: "runtime.operation.recovered".to_string(),
            level: "warn".to_string(),
            component: LOGS_COMPONENT_HOST.to_string(),
            source_kind: "operation".to_string(),
            source_id: None,
            message: "cancelled abandoned operations during startup".to_string(),
            attributes: serde_json::json!({
                "count": recovered_operations.len(),
                "operation_ids": recovered_ids,
            }),
            created_at: None,
        });
    }

    Ok(AppState {
        server_config,
        ui_config,
        overview: PlatformOverview::default(),
        runtime_paths,
        extensions,
        agents,
        skills,
        model_endpoints,
        spaces,
        rate_limit_state: RateLimitState::new(),
        schedule_lock: Arc::new(tokio::sync::Mutex::new(())),
        logs,
        event_bus,
        agent_permissions,
        operations,
        extension_runtime_store,
        realtime,
        logs_guard,
    })
}

pub async fn run_server(home_dir: impl AsRef<Path>) -> Result<(), AppError> {
    let state = bootstrap_app_state(home_dir).await?;
    let logs = state.logs.clone();
    let _ = state.logs.append_log(LogEntryWrite {
        event: "runtime.host.started".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_HOST.to_string(),
        source_kind: "system".to_string(),
        source_id: None,
        message: "server started".to_string(),
        attributes: serde_json::json!({
            "host": state.server_config.host,
            "port": state.server_config.port,
        }),
        created_at: None,
    });

    let (cancel_tx, cancel_rx) = tokio::sync::watch::channel(false);
    let extension_state = state.clone();
    let extensions = state.extensions.clone();
    let realtime = state.realtime.clone();
    let refresh_log = state.logs.clone();
    let mut extension_cancel = cancel_rx.clone();
    tokio::spawn(async move {
        let mut first_cycle = true;
        loop {
            if !first_cycle
                && !wait_for_background_tick(
                    &extension_state,
                    &mut extension_cancel,
                    |background| background.extension_refresh_ms,
                )
                .await
            {
                break;
            }
            first_cycle = false;
            match extensions.refresh_from_disk(EXTENSION_REFRESH_SUMMARY) {
                Ok(Some(_)) => {
                    realtime.publish(crate::realtime::RealtimeEvent::ExtensionsChanged);
                }
                Ok(None) => {}
                Err(_) => {
                    let _ = refresh_log.append_log(LogEntryWrite {
                        event: "runtime.extension.refresh_failed".to_string(),
                        level: "warn".to_string(),
                        component: LOGS_COMPONENT_EXTENSION_HOST.to_string(),
                        source_kind: "system".to_string(),
                        source_id: None,
                        message: "extension refresh failed".to_string(),
                        attributes: serde_json::json!({}),
                        created_at: None,
                    });
                }
            }
        }
    });

    let schedule_state = state.clone();
    let mut schedule_cancel = cancel_rx.clone();
    tokio::spawn(async move {
        let mut first_cycle = true;
        loop {
            if !first_cycle
                && !wait_for_background_tick(&schedule_state, &mut schedule_cancel, |background| {
                    background.schedule_tick_ms
                })
                .await
            {
                break;
            }
            first_cycle = false;
            run_due_schedules_once(&schedule_state).await;
        }
    });

    let event_state = state.clone();
    let mut event_cancel = cancel_rx.clone();
    tokio::spawn(async move {
        let mut first_cycle = true;
        loop {
            if !first_cycle
                && !wait_for_background_tick(&event_state, &mut event_cancel, |background| {
                    background.event_delivery_tick_ms
                })
                .await
            {
                break;
            }
            first_cycle = false;
            drain_hook_deliveries_once(&event_state).await;
        }
    });

    let address = format!("{}:{}", state.server_config.host, state.server_config.port);
    let listener = TcpListener::bind(&address).await?;
    let serve_result = axum::serve(listener, build_router(state)).await;
    let _ = cancel_tx.send(true);
    let _ = logs.append_log(LogEntryWrite {
        event: "runtime.host.stopping".to_string(),
        level: "info".to_string(),
        component: LOGS_COMPONENT_HOST.to_string(),
        source_kind: "system".to_string(),
        source_id: None,
        message: "server stopping".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
    serve_result?;
    Ok(())
}

async fn wait_for_background_tick<F>(
    state: &AppState,
    cancel: &mut tokio::sync::watch::Receiver<bool>,
    resolve_delay_ms: F,
) -> bool
where
    F: Fn(&ennoia_kernel::BackgroundRuntimeConfig) -> u64,
{
    let delay_ms = resolve_delay_ms(&live_server_config(state).background);
    tokio::select! {
        _ = tokio::time::sleep(Duration::from_millis(delay_ms)) => true,
        changed = cancel.changed() => !(changed.is_err() || *cancel.borrow()),
    }
}

async fn drain_hook_deliveries_once(state: &AppState) {
    let pending = match state.event_bus.list_pending_deliveries(32) {
        Ok(items) => items,
        Err(error) => {
            let _ = state.logs.append_log(LogEntryWrite {
                event: "runtime.event_bus.pending_failed".to_string(),
                level: "warn".to_string(),
                component: LOGS_COMPONENT_EVENT_BUS.to_string(),
                source_kind: "system".to_string(),
                source_id: None,
                message: "event bus pending delivery scan failed".to_string(),
                attributes: serde_json::json!({ "error": error.to_string() }),
                created_at: None,
            });
            return;
        }
    };

    for delivery in pending {
        let span_trace = TraceContext {
            request_id: delivery.trace.request_id.clone(),
            trace_id: delivery.trace.trace_id.clone(),
            span_id: next_span_id(),
            parent_span_id: None,
            sampled: delivery.trace.sampled,
            source: "event_bus.delivery".to_string(),
        };
        let started = Instant::now();
        let started_at = chrono::Utc::now().to_rfc3339();
        let request = ennoia_kernel::ExtensionRpcRequest {
            params: serde_json::to_value(&delivery.envelope).unwrap_or(serde_json::Value::Null),
            context: serde_json::json!({
                "source": "event_bus",
                "delivery_id": delivery.id,
                "event_id": delivery.event_id,
                "trace": {
                    "request_id": span_trace.request_id,
                    "trace_id": span_trace.trace_id,
                    "span_id": span_trace.span_id,
                    "parent_span_id": span_trace.parent_span_id,
                    "sampled": span_trace.sampled,
                    "source": span_trace.source,
                    "traceparent": span_trace.to_traceparent(),
                }
            }),
        };
        let response =
            dispatch_extension_rpc(state, &delivery.extension_id, &delivery.handler, request).await;

        match response {
            Ok(response) if response.ok => {
                let _ = state.event_bus.mark_delivery_succeeded(&delivery.id);
                let delivery_span_id = span_trace.span_id.clone();
                record_trace_span(
                    state,
                    LogTraceWrite {
                        trace: span_trace,
                        kind: "hook_delivery".to_string(),
                        name: "event_bus.delivery".to_string(),
                        component: LOGS_COMPONENT_EVENT_BUS.to_string(),
                        source_kind: "extension".to_string(),
                        source_id: Some(delivery.extension_id.clone()),
                        status: "ok".to_string(),
                        attributes: serde_json::json!({
                            "event_id": delivery.event_id,
                            "delivery_id": delivery.id,
                            "handler": delivery.handler,
                            "event": delivery.envelope.event,
                        }),
                        started_at,
                        ended_at: chrono::Utc::now().to_rfc3339(),
                        duration_ms: started.elapsed().as_millis() as i64,
                    },
                );
                let _ = state.logs.append_span_link(LogTraceLinkWrite {
                    trace_id: delivery.trace.trace_id.clone(),
                    span_id: delivery_span_id,
                    linked_trace_id: delivery.trace.trace_id.clone(),
                    linked_span_id: delivery.trace.span_id.clone(),
                    link_type: "follows_from".to_string(),
                    attributes: serde_json::json!({}),
                    created_at: None,
                });
            }
            Ok(response) => {
                let error = response
                    .error
                    .map(|item| format!("{}: {}", item.code, item.message))
                    .unwrap_or_else(|| "hook delivery failed".to_string());
                handle_delivery_failure(
                    state,
                    &span_trace,
                    &delivery.trace.span_id,
                    started_at,
                    started,
                    &delivery.id,
                    delivery.attempt_count,
                    &delivery.extension_id,
                    &delivery.event_id,
                    &delivery.handler,
                    &delivery.envelope.event,
                    &error,
                );
            }
            Err(error) => {
                handle_delivery_failure(
                    state,
                    &span_trace,
                    &delivery.trace.span_id,
                    started_at,
                    started,
                    &delivery.id,
                    delivery.attempt_count,
                    &delivery.extension_id,
                    &delivery.event_id,
                    &delivery.handler,
                    &delivery.envelope.event,
                    &error.to_string(),
                );
            }
        }
    }
}

pub async fn dispatch_extension_rpc(
    state: &AppState,
    extension_id: &str,
    method: &str,
    request: ennoia_kernel::ExtensionRpcRequest,
) -> io::Result<ennoia_kernel::ExtensionRpcResponse> {
    let timeout_ms = state
        .extensions
        .get(extension_id)
        .map(|extension| extension.runtime.timeout_ms)
        .unwrap_or_else(|| live_server_config(state).extension_runtime.timeout_ms);
    let runtime_handle = tokio::runtime::Handle::current();
    let extensions = state.extensions.clone();
    let state_for_dispatcher = state.clone();
    let extension_id_owned = extension_id.to_string();
    let method_owned = method.to_string();
    let response = tokio::time::timeout(
        Duration::from_millis(timeout_ms),
        tokio::task::spawn_blocking(move || {
            let dispatcher =
                ServerHostCapabilityDispatcher::new(state_for_dispatcher, runtime_handle);
            extensions.dispatch_rpc(
                &extension_id_owned,
                &method_owned,
                request,
                Some(&dispatcher),
            )
        }),
    )
    .await;

    match response {
        Ok(Ok(result)) => result,
        Ok(Err(join_error)) => Err(io::Error::other(format!(
            "extension rpc task failed: {join_error}"
        ))),
        Err(_) => {
            state.extensions.terminate_worker(extension_id);
            Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!(
                    "extension rpc timed out: extension={extension_id}, method={method}, timeout_ms={timeout_ms}"
                ),
            ))
        }
    }
}

fn handle_delivery_failure(
    state: &AppState,
    trace: &TraceContext,
    producer_span_id: &str,
    started_at: String,
    started: Instant,
    delivery_id: &str,
    attempt_count: u32,
    extension_id: &str,
    event_id: &str,
    handler: &str,
    event: &str,
    error: &str,
) {
    record_trace_span(
        state,
        LogTraceWrite {
            trace: trace.clone(),
            kind: "hook_delivery".to_string(),
            name: "event_bus.delivery".to_string(),
            component: LOGS_COMPONENT_EVENT_BUS.to_string(),
            source_kind: "extension".to_string(),
            source_id: Some(extension_id.to_string()),
            status: "error".to_string(),
            attributes: serde_json::json!({
                "event_id": event_id,
                "delivery_id": delivery_id,
                "handler": handler,
                "event": event,
                "error": error,
            }),
            started_at,
            ended_at: chrono::Utc::now().to_rfc3339(),
            duration_ms: started.elapsed().as_millis() as i64,
        },
    );
    let _ = state.logs.append_span_link(LogTraceLinkWrite {
        trace_id: trace.trace_id.clone(),
        span_id: trace.span_id.clone(),
        linked_trace_id: trace.trace_id.clone(),
        linked_span_id: producer_span_id.to_string(),
        link_type: "follows_from".to_string(),
        attributes: serde_json::json!({}),
        created_at: None,
    });
    let terminal = state
        .event_bus
        .mark_delivery_retry(delivery_id, error, attempt_count)
        .unwrap_or(false);
    if terminal {
        let _ = state.logs.append_log_scoped(
            LogEntryWrite {
                event: "runtime.event_bus.delivery_failed".to_string(),
                level: "warn".to_string(),
                component: LOGS_COMPONENT_EVENT_BUS.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(extension_id.to_string()),
                message: "hook delivery exhausted retries".to_string(),
                attributes: serde_json::json!({
                    "delivery_id": delivery_id,
                    "error": error,
                }),
                created_at: None,
            },
            Some(trace),
        );
    }
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

pub fn live_server_config(state: &AppState) -> ServerConfig {
    let mut config = read_toml_or_default(state.runtime_paths.server_config_file())
        .unwrap_or_else(|_| state.server_config.clone())
        .normalize();
    apply_server_log_env_overrides(&mut config.logging);
    config
}

pub fn live_ui_config(state: &AppState) -> UiConfig {
    read_toml_or_default(state.runtime_paths.ui_config_file())
        .unwrap_or_else(|_| state.ui_config.clone())
        .normalize()
}

pub fn load_agent_configs(paths: &RuntimePaths) -> Result<Vec<AgentConfig>, AppError> {
    let mut agents = load_agent_documents(paths)?
        .into_iter()
        .map(|document| document.profile)
        .collect::<Vec<_>>();
    for agent in &mut agents {
        normalize_agent_config(paths, agent);
    }
    agents.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(agents)
}

pub fn load_agent_document(
    paths: &RuntimePaths,
    agent_id: &str,
) -> Result<Option<AgentDocument>, AppError> {
    migrate_legacy_agent_layout(paths)?;
    let path = paths.agent_config_file(agent_id);
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)?;
    Ok(Some(toml::from_str(&contents)?))
}

pub fn normalize_agent_document(
    paths: &RuntimePaths,
    mut document: AgentDocument,
) -> AgentDocument {
    normalize_agent_config(paths, &mut document.profile);
    document
}

pub fn write_agent_config(paths: &RuntimePaths, payload: &AgentConfig) -> Result<(), AppError> {
    let permission_profile = load_agent_document(paths, &payload.id)?
        .map(|document| document.permission_profile)
        .unwrap_or_else(AgentPermissionProfile::default_profile);
    write_agent_document(
        paths,
        &AgentDocument {
            profile: payload.clone(),
            permission_profile,
        },
    )
}

pub fn delete_agent_config(paths: &RuntimePaths, agent_id: &str) -> Result<bool, AppError> {
    migrate_legacy_agent_layout(paths)?;
    let path = paths.agent_config_file(agent_id);
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(&path)?;
    let agent_dir = paths.agent_dir(agent_id);
    if agent_dir.exists() && fs::read_dir(&agent_dir)?.next().is_none() {
        fs::remove_dir(agent_dir)?;
    }
    Ok(true)
}

pub fn load_skill_configs(paths: &RuntimePaths) -> Result<Vec<SkillConfig>, AppError> {
    let registry = load_skill_registry(paths)?;
    let blocked = registry
        .blocked_builtin_sync
        .into_iter()
        .collect::<std::collections::BTreeSet<_>>();
    let enabled_by_id = registry
        .skills
        .into_iter()
        .map(|entry| (entry.id, entry.enabled))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut skills = Vec::new();
    if paths.skills_dir().exists() {
        for entry in fs::read_dir(paths.skills_dir())? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let root = entry.path();
            let descriptor_path = root.join("skill.toml");
            if !descriptor_path.exists() {
                continue;
            }
            let manifest = load_skill_manifest_from_path(&descriptor_path)?;
            let enabled = enabled_by_id.get(&manifest.id).copied().unwrap_or(true);
            skills.push(SkillConfig {
                id: manifest.id.clone(),
                version: manifest.version,
                description: manifest.description,
                mount: manifest.mount,
                actions: manifest.actions,
                enabled,
                builtin_sync_blocked: blocked.contains(&manifest.id),
            });
        }
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

pub fn load_model_endpoint_configs(
    paths: &RuntimePaths,
) -> Result<Vec<ModelEndpointConfig>, AppError> {
    let mut model_endpoints =
        load_configs_from_dir::<ModelEndpointConfig>(paths.model_endpoints_config_dir())?;
    model_endpoints.sort_by(|left, right| left.id.cmp(&right.id));
    Ok(model_endpoints)
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

fn load_skill_manifest_from_path(path: &Path) -> Result<SkillManifest, AppError> {
    let contents = fs::read_to_string(path)?;
    Ok(toml::from_str(&contents)?)
}

pub fn load_agent_documents(paths: &RuntimePaths) -> Result<Vec<AgentDocument>, AppError> {
    migrate_legacy_agent_layout(paths)?;
    if !paths.agents_dir().exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(paths.agents_dir())? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let path = entry.path().join("agent.toml");
        if !path.exists() {
            continue;
        }
        let contents = fs::read_to_string(path)?;
        let item: AgentDocument = toml::from_str(&contents)?;
        items.push(item);
    }
    items.sort_by(|left, right| left.profile.id.cmp(&right.profile.id));
    Ok(items)
}

pub fn write_agent_document(
    paths: &RuntimePaths,
    document: &AgentDocument,
) -> Result<(), AppError> {
    let agent_dir = paths.agent_dir(&document.profile.id);
    fs::create_dir_all(&agent_dir)?;
    fs::write(
        paths.agent_config_file(&document.profile.id),
        toml::to_string_pretty(document)?,
    )?;
    Ok(())
}

fn migrate_legacy_agent_layout(paths: &RuntimePaths) -> Result<(), AppError> {
    let legacy_agents_dir = paths.config_dir().join("agents");
    let legacy_policies_dir = paths.config_dir().join("agent-policies");
    if !legacy_agents_dir.exists() && !legacy_policies_dir.exists() {
        return Ok(());
    }

    if legacy_agents_dir.exists() {
        for entry in fs::read_dir(&legacy_agents_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let contents = fs::read_to_string(entry.path())?;
            let agent = toml::from_str::<AgentConfig>(&contents)?;
            let legacy_policy_file = legacy_policies_dir.join(format!("{}.toml", agent.id));
            write_agent_document(
                paths,
                &AgentDocument {
                    profile: agent,
                    permission_profile: AgentPermissionProfile::default_profile(),
                },
            )?;
            fs::remove_file(entry.path())?;
            if legacy_policy_file.exists() {
                fs::remove_file(legacy_policy_file)?;
            }
        }
    }

    remove_dir_if_empty(&legacy_agents_dir)?;
    remove_dir_if_empty(&legacy_policies_dir)?;
    Ok(())
}

fn remove_dir_if_empty(path: &Path) -> Result<(), AppError> {
    if path.exists() && fs::read_dir(path)?.next().is_none() {
        fs::remove_dir(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn sample_agent(id: &str) -> AgentConfig {
        AgentConfig {
            id: id.to_string(),
            display_name: format!("{id}-display"),
            description: "desc".to_string(),
            system_prompt: "prompt".to_string(),
            model_endpoint_id: "openai".to_string(),
            model_id: "model".to_string(),
            generation_options: Default::default(),
            skills: vec!["skill-a".to_string()],
            enabled: true,
            kind: "agent".to_string(),
            default_model: "model".to_string(),
            skills_dir: String::new(),
            working_dir: String::new(),
            artifacts_dir: String::new(),
            execution_environment: ennoia_kernel::AgentExecutionEnvironment::default(),
        }
    }

    #[test]
    fn load_agent_configs_migrates_legacy_layout_into_agent_directory() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path());
        let legacy_agents_dir = paths.config_dir().join("agents");
        let legacy_policies_dir = paths.config_dir().join("agent-policies");
        fs::create_dir_all(&legacy_agents_dir).expect("legacy agents dir");
        fs::create_dir_all(&legacy_policies_dir).expect("legacy policies dir");

        let agent = sample_agent("writer");
        fs::write(
            legacy_agents_dir.join("writer.toml"),
            toml::to_string_pretty(&agent).expect("serialize agent"),
        )
        .expect("write legacy agent");
        fs::write(
            legacy_policies_dir.join("writer.toml"),
            toml::to_string_pretty(&AgentPermissionProfile {
                mode: "blacklist".to_string(),
                ..AgentPermissionProfile::default_profile()
            })
            .expect("serialize profile"),
        )
        .expect("write legacy policy");

        let agents = load_agent_configs(&paths).expect("load migrated agents");
        assert_eq!(agents.len(), 1);
        assert_eq!(agents[0].id, "writer");

        let document = load_agent_document(&paths, "writer")
            .expect("load agent document")
            .expect("agent document exists");
        assert_eq!(document.permission_profile.mode, "whitelist");
        assert!(paths.agent_config_file("writer").exists());
        assert!(!legacy_agents_dir.join("writer.toml").exists());
        assert!(!legacy_policies_dir.join("writer.toml").exists());
    }

    #[test]
    fn write_agent_config_preserves_existing_permission_profile() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path());
        let agent = sample_agent("planner");
        write_agent_document(
            &paths,
            &AgentDocument {
                profile: agent.clone(),
                permission_profile: AgentPermissionProfile {
                    mode: "blacklist".to_string(),
                    ..AgentPermissionProfile::default_profile()
                },
            },
        )
        .expect("seed agent document");

        let mut updated = agent.clone();
        updated.display_name = "Planner Prime".to_string();
        write_agent_config(&paths, &updated).expect("write updated agent");

        let document = load_agent_document(&paths, "planner")
            .expect("load updated document")
            .expect("document exists");
        assert_eq!(document.profile.display_name, "Planner Prime");
        assert_eq!(document.permission_profile.mode, "blacklist");
    }
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
    if agent.execution_environment.sandbox_enabled {
        agent.working_dir = "/workspace".to_string();
        agent.artifacts_dir = "/artifacts".to_string();
    } else {
        if !agent.working_dir.is_empty() {
            agent.working_dir = paths.display_for_user(paths.expand_home_token(&agent.working_dir));
        } else {
            agent.working_dir = paths.display_for_user(paths.agent_working_dir(&agent.id));
        }
        if !agent.artifacts_dir.is_empty() {
            agent.artifacts_dir =
                paths.display_for_user(paths.expand_home_token(&agent.artifacts_dir));
        } else {
            agent.artifacts_dir = paths.display_for_user(paths.agent_artifacts_dir(&agent.id));
        }
    }
    if !agent.skills_dir.is_empty() {
        agent.skills_dir = paths.display_for_user(paths.expand_home_token(&agent.skills_dir));
    } else {
        agent.skills_dir = paths.display_for_user(paths.agent_skills_dir(&agent.id));
    }
}

fn extension_runtime_config(
    paths: &RuntimePaths,
    server_config: &ServerConfig,
) -> ExtensionRuntimeConfig {
    ExtensionRuntimeConfig {
        registry_file: paths.extensions_registry_file(),
        logs_dir: paths.extensions_logs_dir(),
        home_dir: paths.home().to_path_buf(),
        runtime_defaults: ennoia_kernel::ExtensionRuntimeSpec {
            timeout_ms: server_config.extension_runtime.timeout_ms,
            memory_limit_mb: server_config.extension_runtime.memory_limit_mb,
            ..ennoia_kernel::ExtensionRuntimeSpec::default()
        },
    }
}

pub fn upsert_skill_package(paths: &RuntimePaths, payload: &SkillConfig) -> Result<(), AppError> {
    let skill_dir = paths.skill_dir(&payload.id);
    fs::create_dir_all(&skill_dir)?;
    fs::write(
        skill_dir.join("skill.toml"),
        toml::to_string_pretty(&SkillManifest {
            id: payload.id.clone(),
            version: payload.version.clone(),
            description: payload.description.clone(),
            mount: payload.mount.clone(),
            actions: payload.actions.clone(),
        })?,
    )?;

    let mut registry = load_skill_registry(paths)?;
    registry.skills.retain(|item| item.id != payload.id);
    registry.skills.push(SkillRegistryEntry {
        id: payload.id.clone(),
        enabled: payload.enabled,
    });
    registry
        .blocked_builtin_sync
        .retain(|item| item != &payload.id);
    if payload.builtin_sync_blocked {
        registry.blocked_builtin_sync.push(payload.id.clone());
    }
    registry.blocked_builtin_sync.sort();
    registry.blocked_builtin_sync.dedup();
    sort_skill_registry_entries(&mut registry.skills);
    write_skill_registry(paths, &registry)
}

pub fn delete_skill_package(paths: &RuntimePaths, skill_id: &str) -> Result<bool, AppError> {
    let mut registry = load_skill_registry(paths)?;
    let skill_root = paths.skill_dir(skill_id);
    let existed = skill_root.exists();
    if skill_root.exists() {
        fs::remove_dir_all(&skill_root)?;
    }
    registry.skills.retain(|item| item.id != skill_id);
    if builtin_skill_ids().iter().any(|id| id == skill_id) {
        registry
            .blocked_builtin_sync
            .retain(|item| item != skill_id);
        registry.blocked_builtin_sync.push(skill_id.to_string());
        registry.blocked_builtin_sync.sort();
        registry.blocked_builtin_sync.dedup();
    } else {
        registry
            .blocked_builtin_sync
            .retain(|item| item != skill_id);
    }
    sort_skill_registry_entries(&mut registry.skills);
    write_skill_registry(paths, &registry)?;
    Ok(existed)
}

fn sort_skill_registry_entries(entries: &mut [SkillRegistryEntry]) {
    entries.sort_by(|left, right| left.id.cmp(&right.id));
}

fn builtin_skill_ids() -> Vec<String> {
    let mut ids = builtins::skills()
        .into_iter()
        .filter(|asset| asset.logical_path.ends_with("/skill.toml"))
        .filter_map(|asset| asset.logical_path.split('/').nth(1).map(str::to_string))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

pub(crate) fn record_trace_span(state: &AppState, entry: LogTraceWrite) {
    let _ = state.logs.append_span(entry);
}
