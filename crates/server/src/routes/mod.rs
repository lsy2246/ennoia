use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path as StdPath, PathBuf};
use std::time::Duration;

use axum::{
    extract::{Path, Query, State},
    http::header,
    http::StatusCode,
    middleware as axum_middleware,
    response::sse::{Event, KeepAlive, Sse},
    response::IntoResponse,
    routing::{delete, get, post, put},
    Extension, Json, Router,
};
use chrono::Utc;
use ennoia_contract::ApiError;
use ennoia_extension_host::{
    read_registry_file, ExtensionRuntimeSnapshot, RegisteredCommandContribution,
    RegisteredHookContribution, RegisteredLocaleContribution, RegisteredPageContribution,
    RegisteredPanelContribution, RegisteredProviderContribution, RegisteredThemeContribution,
    ResolvedExtensionSnapshot,
};
use ennoia_kernel::{
    AgentConfig, AppConfig, ArtifactKind, ArtifactSpec, BootstrapState, ConfigChangeRecord,
    ConfigEntry, ConfigStore, ConversationSpec, ConversationTopology, ExtensionDiagnostic,
    ExtensionRuntimeEvent, HandoffSpec, LaneSpec, LocalizedText, MessageRole, MessageSpec,
    OwnerKind, OwnerRef, ProviderConfig, RunSpec, RuntimeProfile, SkillConfig, SystemConfig,
    TaskSpec, UiConfig, UiPreference, UiPreferenceRecord, ALL_CONFIG_KEYS, CONFIG_KEY_BOOTSTRAP,
};
use ennoia_memory::{
    AssembleRequest, ContextView, EpisodeKind, EpisodeRequest, MemoryKind, MemoryRecord,
    MemorySource, RecallMode, RecallQuery, RecallResult, RememberReceipt, RememberRequest,
    ReviewAction, ReviewActionKind, ReviewReceipt, Stability,
};
use ennoia_observability::RequestContext;
use ennoia_orchestrator::{RunRequest, RunTrigger};
use ennoia_scheduler::{JobKind, JobRecord, ScheduleKind};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::app::{
    delete_config_from_dir, delete_skill_package, load_agent_configs, load_provider_configs,
    load_skill_configs, normalize_app_config, upsert_skill_package, write_config_to_dir, AppState,
};
use crate::db::{self, JobDetailRow, JobRow, LogRecordRow};
use crate::middleware::{
    body_limit_middleware, cors_middleware, logging_middleware, rate_limit_middleware,
    request_context_middleware, timeout_middleware,
};

mod config;
mod conversations;
mod execution;
mod extensions;
mod jobs;
mod logs;
mod memories;
mod resources;
mod runtime;

use config::*;
use conversations::*;
use execution::*;
use extensions::*;
use jobs::*;
use logs::*;
use memories::*;
use resources::*;
use runtime::*;

type ApiResult<T> = Result<Json<T>, ApiError>;

fn scoped(error: ApiError, request: &RequestContext) -> ApiError {
    error.with_request_id(&request.request_id)
}

pub fn build_router(state: AppState) -> Router {
    let bootstrap = Router::new()
        .route("/api/v1/bootstrap/status", get(bootstrap_status))
        .route("/api/v1/bootstrap/setup", post(bootstrap_setup));

    let runtime = Router::new()
        .route(
            "/api/v1/runtime/profile",
            get(runtime_profile).put(runtime_profile_put),
        )
        .route(
            "/api/v1/runtime/preferences",
            get(runtime_preferences).put(runtime_preferences_put),
        )
        .route("/api/v1/runtime/config", get(config_list))
        .route(
            "/api/v1/runtime/app-config",
            get(runtime_app_config).put(runtime_app_config_put),
        )
        .route("/api/v1/runtime/config/snapshot", get(config_snapshot))
        .route(
            "/api/v1/runtime/config/{key}",
            get(config_get).put(config_put),
        )
        .route("/api/v1/runtime/config/{key}/history", get(config_history));

    let conversations = Router::new()
        .route(
            "/api/v1/conversations",
            get(conversations_list).post(conversations_create),
        )
        .route(
            "/api/v1/conversations/{conversation_id}",
            get(conversation_detail).delete(conversation_delete),
        )
        .route(
            "/api/v1/conversations/{conversation_id}/messages",
            get(conversation_messages).post(conversation_messages_create),
        )
        .route(
            "/api/v1/conversations/{conversation_id}/runs",
            get(conversation_runs),
        )
        .route(
            "/api/v1/conversations/{conversation_id}/lanes",
            get(conversation_lanes),
        )
        .route(
            "/api/v1/lanes/{lane_id}/handoffs",
            get(lane_handoffs).post(lane_handoffs_create),
        );

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/ui/runtime", get(ui_runtime))
        .route("/api/v1/ui/messages", get(ui_messages))
        .route(
            "/api/v1/spaces/{space_id}/ui-preferences",
            get(space_ui_preferences).put(space_ui_preferences_put),
        )
        .route("/api/v1/extensions", get(extensions))
        .route(
            "/api/v1/extensions/{extension_id}/enabled",
            put(extension_enabled_put),
        )
        .route("/api/v1/extensions/runtime", get(extensions_runtime))
        .route("/api/v1/extensions/events", get(extension_events))
        .route(
            "/api/v1/extensions/events/stream",
            get(extension_events_stream),
        )
        .route("/api/v1/extensions/registry", get(extensions_runtime))
        .route("/api/v1/extensions/pages", get(extension_pages))
        .route("/api/v1/extensions/panels", get(extension_panels))
        .route("/api/v1/extensions/commands", get(extension_commands))
        .route("/api/v1/extensions/providers", get(extension_providers))
        .route("/api/v1/extensions/hooks", get(extension_hooks))
        .route("/api/v1/extensions/attach", post(extension_attach))
        .route("/api/v1/extensions/{extension_id}", get(extension_detail))
        .route(
            "/api/v1/extensions/{extension_id}/diagnostics",
            get(extension_diagnostics),
        )
        .route(
            "/api/v1/extensions/{extension_id}/frontend/module",
            get(extension_frontend_module),
        )
        .route(
            "/api/v1/extensions/{extension_id}/themes/{theme_id}/stylesheet",
            get(extension_theme_stylesheet),
        )
        .route(
            "/api/v1/extensions/{extension_id}/logs",
            get(extension_logs),
        )
        .route(
            "/api/v1/extensions/{extension_id}/reload",
            post(extension_reload),
        )
        .route(
            "/api/v1/extensions/{extension_id}/restart",
            post(extension_restart),
        )
        .route(
            "/api/v1/extensions/attach/{extension_id}",
            delete(extension_detach),
        )
        .route("/api/v1/agents", get(agents).post(agent_create))
        .route(
            "/api/v1/agents/{agent_id}",
            get(agent_detail).put(agent_update).delete(agent_delete),
        )
        .route("/api/v1/skills", get(skills).post(skill_create))
        .route(
            "/api/v1/skills/{skill_id}",
            get(skill_detail).put(skill_update).delete(skill_delete),
        )
        .route("/api/v1/providers", get(providers).post(provider_create))
        .route(
            "/api/v1/providers/{provider_id}",
            get(provider_detail)
                .put(provider_update)
                .delete(provider_delete),
        )
        .route(
            "/api/v1/providers/{provider_id}/models",
            get(provider_models),
        )
        .route("/api/v1/spaces", get(spaces))
        .route("/api/v1/runs", get(runs))
        .route("/api/v1/runs/{run_id}/tasks", get(run_tasks))
        .route("/api/v1/runs/{run_id}/artifacts", get(run_artifacts))
        .route("/api/v1/runs/{run_id}/stages", get(run_stages))
        .route("/api/v1/runs/{run_id}/decisions", get(run_decisions))
        .route("/api/v1/runs/{run_id}/gates", get(run_gates))
        .route("/api/v1/tasks", get(tasks))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/logs", get(logs_list))
        .route("/api/v1/logs/frontend", post(frontend_log_create))
        .route("/api/v1/memories", get(memories_list).post(memories_create))
        .route("/api/v1/memories/recall", post(memories_recall))
        .route("/api/v1/memories/review", post(memories_review))
        .route("/api/v1/jobs", get(jobs_list).post(jobs_create))
        .route(
            "/api/v1/jobs/{job_id}",
            get(job_detail).put(job_update).delete(job_delete),
        )
        .route("/api/v1/jobs/{job_id}/run", post(job_run_now))
        .route("/api/v1/jobs/{job_id}/enable", post(job_enable))
        .route("/api/v1/jobs/{job_id}/disable", post(job_disable))
        .merge(bootstrap)
        .merge(runtime)
        .merge(conversations)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            body_limit_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            timeout_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            cors_middleware,
        ))
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            logging_middleware,
        ))
        .layer(axum_middleware::from_fn(request_context_middleware))
        .with_state(state)
}

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
    app: &'static str,
}

#[derive(Debug, Serialize)]
struct OverviewResponse {
    app_name: String,
    web_title: LocalizedText,
    default_theme: String,
    modules: Vec<String>,
    counts: JsonValue,
}

#[derive(Debug, Serialize)]
struct UiRuntimeRegistryResponse {
    pages: Vec<RegisteredPageContribution>,
    panels: Vec<RegisteredPanelContribution>,
    themes: Vec<RegisteredThemeContribution>,
    locales: Vec<RegisteredLocaleContribution>,
    providers: Vec<RegisteredProviderContribution>,
}

#[derive(Debug, Serialize)]
struct UiRuntimeVersionsResponse {
    registry: u64,
    preferences: u64,
}

#[derive(Debug, Serialize)]
struct UiRuntimeResponse {
    ui_config: UiConfig,
    registry: UiRuntimeRegistryResponse,
    instance_preference: Option<UiPreferenceRecord>,
    space_preferences: Vec<UiPreferenceRecord>,
    versions: UiRuntimeVersionsResponse,
}

#[derive(Debug, Serialize)]
struct UiMessageBundleResponse {
    locale: String,
    resolved_locale: String,
    namespace: String,
    messages: HashMap<String, String>,
    source: String,
    version: String,
}

#[derive(Debug, Serialize)]
struct UiMessagesResponse {
    locale: String,
    fallback_locale: String,
    bundles: Vec<UiMessageBundleResponse>,
}

#[derive(Debug, Deserialize)]
struct UiMessagesQuery {
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    namespaces: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UiPreferencePayload {
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    theme_id: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    date_style: Option<String>,
    #[serde(default)]
    density: Option<String>,
    #[serde(default)]
    motion: Option<String>,
}

#[derive(Debug, Deserialize)]
struct BootstrapSetupPayload {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    default_space_id: Option<String>,
    #[serde(default)]
    theme_id: Option<String>,
    #[serde(default)]
    date_style: Option<String>,
    #[serde(default)]
    density: Option<String>,
    #[serde(default)]
    motion: Option<String>,
}

#[derive(Debug, Serialize)]
struct BootstrapSetupResponse {
    bootstrap: BootstrapState,
    profile: RuntimeProfile,
    preference: UiPreferenceRecord,
}

#[derive(Debug, Deserialize)]
struct RuntimeProfilePayload {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    locale: Option<String>,
    #[serde(default)]
    time_zone: Option<String>,
    #[serde(default)]
    default_space_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateConversationPayload {
    topology: String,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    space_id: Option<String>,
    #[serde(default)]
    agent_ids: Vec<String>,
    #[serde(default)]
    lane_name: Option<String>,
    #[serde(default)]
    lane_type: Option<String>,
    #[serde(default)]
    lane_goal: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConversationCreateResponse {
    conversation: ConversationSpec,
    default_lane: LaneSpec,
}

#[derive(Debug, Serialize)]
struct ConversationDetailResponse {
    conversation: ConversationSpec,
    lanes: Vec<LaneSpec>,
}

#[derive(Debug, Deserialize)]
struct ConversationMessagePayload {
    body: String,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    lane_id: Option<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct HandoffPayload {
    to_lane_id: String,
    #[serde(default)]
    from_agent_id: Option<String>,
    #[serde(default)]
    to_agent_id: Option<String>,
    summary: String,
    instructions: String,
    #[serde(default)]
    status: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateJobRequest {
    owner_kind: String,
    owner_id: String,
    #[serde(default)]
    job_kind: Option<String>,
    schedule_kind: String,
    schedule_value: String,
    #[serde(default)]
    payload: Option<JsonValue>,
    #[serde(default)]
    max_retries: Option<u32>,
    #[serde(default)]
    run_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdateJobRequest {
    #[serde(default)]
    job_kind: Option<String>,
    #[serde(default)]
    schedule_kind: Option<String>,
    #[serde(default)]
    schedule_value: Option<String>,
    #[serde(default)]
    payload: Option<JsonValue>,
    #[serde(default)]
    max_retries: Option<u32>,
    #[serde(default)]
    run_at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionEnabledPayload {
    enabled: bool,
}

#[derive(Debug, Serialize)]
struct ExtensionWorkbenchRecord {
    id: String,
    name: String,
    enabled: bool,
    status: String,
    version: String,
    kind: String,
    source_mode: String,
    install_dir: String,
    source_root: String,
    diagnostics: Vec<ExtensionDiagnostic>,
}

#[derive(Debug, Deserialize)]
struct RememberPayload {
    owner_kind: String,
    owner_id: String,
    namespace: String,
    memory_kind: String,
    stability: String,
    #[serde(default)]
    title: Option<String>,
    content: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    confidence: Option<f32>,
    #[serde(default)]
    importance: Option<f32>,
    #[serde(default)]
    sources: Vec<MemorySource>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    entities: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct RecallPayload {
    owner_kind: String,
    owner_id: String,
    #[serde(default)]
    query_text: Option<String>,
    #[serde(default)]
    namespace_prefix: Option<String>,
    #[serde(default)]
    memory_kind: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ReviewPayload {
    target_memory_id: String,
    reviewer: String,
    action: String,
    #[serde(default)]
    notes: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConversationEnvelope {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
    run: RunSpec,
    tasks: Vec<TaskSpec>,
    artifacts: Vec<ArtifactSpec>,
    context: ContextView,
    gate_verdicts: Vec<ennoia_kernel::GateVerdict>,
    stage_event: ennoia_kernel::RunStageEvent,
    decision: ennoia_kernel::Decision,
}

#[derive(Debug, Deserialize)]
struct ConfigPutPayload {
    payload: JsonValue,
    updated_by: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogsQuery {
    #[serde(default)]
    limit: Option<u32>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    source: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct FrontendLogPayload {
    level: String,
    title: String,
    summary: String,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    details: Option<String>,
    #[serde(default)]
    at: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ExtensionEventsQuery {
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct ExtensionAttachPayload {
    path: String,
}

#[derive(Debug, Serialize)]
struct ProviderModelsResponse {
    provider_id: String,
    source: String,
    models: Vec<String>,
    recommended_model: Option<String>,
    manual_allowed: bool,
    generation_options: Vec<ennoia_kernel::ProviderGenerationOption>,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "Ennoia",
    })
}

async fn overview(State(state): State<AppState>) -> Json<OverviewResponse> {
    let extension_snapshot = state.extensions.snapshot();
    let agent_count = load_agent_configs(&state.runtime_paths)
        .map(|items| items.len())
        .unwrap_or(state.agents.len());
    let conversation_count = db::count_rows(&state.pool, db::CountTable::Conversations)
        .await
        .unwrap_or(0);
    let message_count = db::count_rows(&state.pool, db::CountTable::Messages)
        .await
        .unwrap_or(0);
    let run_count = db::count_rows(&state.pool, db::CountTable::Runs)
        .await
        .unwrap_or(0);
    let task_count = db::count_rows(&state.pool, db::CountTable::Tasks)
        .await
        .unwrap_or(0);
    let artifact_count = db::count_rows(&state.pool, db::CountTable::Artifacts)
        .await
        .unwrap_or(0);
    let memory_count = db::count_rows(&state.pool, db::CountTable::Memories)
        .await
        .unwrap_or(0);
    let job_count = db::count_rows(&state.pool, db::CountTable::Jobs)
        .await
        .unwrap_or(0);
    let decision_count = db::count_rows(&state.pool, db::CountTable::Decisions)
        .await
        .unwrap_or(0);

    Json(OverviewResponse {
        app_name: state.overview.app_name,
        web_title: state.ui_config.web_title.clone(),
        default_theme: state.ui_config.default_theme.clone(),
        modules: state.overview.modules,
        counts: serde_json::json!({
            "agents": agent_count,
            "spaces": state.spaces.len(),
            "extensions": extension_snapshot.extensions.len(),
            "conversations": conversation_count,
            "messages": message_count,
            "runs": run_count,
            "tasks": task_count,
            "artifacts": artifact_count,
            "memories": memory_count,
            "jobs": job_count,
            "decisions": decision_count
        }),
    })
}

async fn ui_runtime(State(state): State<AppState>) -> Json<UiRuntimeResponse> {
    let snapshot = state.extensions.snapshot();
    let instance_preference = db::get_instance_ui_preference(&state.pool)
        .await
        .ok()
        .flatten()
        .map(to_preference_record);
    let space_preferences = db::list_space_ui_preferences(&state.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(to_preference_record)
        .collect::<Vec<_>>();
    let registry_version = (snapshot.pages.len()
        + snapshot.panels.len()
        + snapshot.themes.len()
        + snapshot.locales.len()
        + snapshot.providers.len()) as u64;
    let preference_version = db::max_ui_preference_version(&state.pool)
        .await
        .unwrap_or(0);

    Json(UiRuntimeResponse {
        ui_config: state.ui_config.clone(),
        registry: UiRuntimeRegistryResponse {
            pages: snapshot.pages,
            panels: snapshot.panels,
            themes: snapshot.themes,
            locales: snapshot.locales,
            providers: snapshot.providers,
        },
        instance_preference,
        space_preferences,
        versions: UiRuntimeVersionsResponse {
            registry: registry_version,
            preferences: preference_version,
        },
    })
}

async fn ui_messages(
    State(state): State<AppState>,
    Query(query): Query<UiMessagesQuery>,
) -> Json<UiMessagesResponse> {
    let locale = query
        .locale
        .unwrap_or_else(|| state.ui_config.default_locale.clone());
    let namespaces = query
        .namespaces
        .as_deref()
        .map(parse_namespaces)
        .filter(|items| !items.is_empty())
        .unwrap_or_else(builtin_message_namespaces);

    let snapshot = state.extensions.snapshot();
    let bundles = namespaces
        .iter()
        .filter_map(|namespace| {
            extension_message_bundle(
                &snapshot.locales,
                &locale,
                &state.ui_config.fallback_locale,
                namespace,
            )
            .or_else(|| {
                builtin_message_bundle(&locale, &state.ui_config.fallback_locale, namespace)
            })
        })
        .collect::<Vec<_>>();

    Json(UiMessagesResponse {
        locale,
        fallback_locale: state.ui_config.fallback_locale.clone(),
        bundles,
    })
}

async fn drive_run(
    state: &AppState,
    conversation: ConversationSpec,
    lane: LaneSpec,
    body: &str,
    goal: &str,
    addressed_agents: Vec<String>,
) -> Result<ConversationEnvelope, String> {
    let now = now_iso();
    let target_agents = resolve_addressed_agents(&conversation, &lane, addressed_agents);
    if target_agents.is_empty() {
        return Err("no addressed agents resolved for this message".to_string());
    }

    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        conversation_id: conversation.id.clone(),
        lane_id: Some(lane.id.clone()),
        sender: "operator".to_string(),
        role: MessageRole::Operator,
        body: body.to_string(),
        mentions: target_agents.clone(),
        created_at: now.clone(),
    };

    db::insert_message(&state.pool, &message)
        .await
        .map_err(|error| error.to_string())?;
    db::upsert_conversation(
        &state.pool,
        &ConversationSpec {
            updated_at: now.clone(),
            ..conversation.clone()
        },
    )
    .await
    .map_err(|error| error.to_string())?;
    db::insert_lane(
        &state.pool,
        &LaneSpec {
            updated_at: now.clone(),
            goal: if lane.goal.is_empty() {
                goal.to_string()
            } else {
                lane.goal.clone()
            },
            ..lane.clone()
        },
    )
    .await
    .map_err(|error| error.to_string())?;

    let recent_messages = db::list_messages_for_conversation(&state.pool, &conversation.id)
        .await
        .map_err(|error| error.to_string())?
        .into_iter()
        .rev()
        .take(12)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .map(|item| format!("{}: {}", item.sender, item.body))
        .collect::<Vec<_>>();

    let context = state
        .memory_store
        .assemble_context(AssembleRequest {
            owner: conversation.owner.clone(),
            conversation_id: Some(conversation.id.clone()),
            run_id: None,
            recent_messages,
            active_tasks: Vec::new(),
            budget_chars: None,
        })
        .await
        .map_err(|error| error.to_string())?;

    let available_agents: Vec<String> = load_agent_configs(&state.runtime_paths)
        .unwrap_or_else(|_| state.agents.clone())
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    let request = RunRequest {
        owner: conversation.owner.clone(),
        conversation: conversation.clone(),
        message: message.clone(),
        trigger: match conversation.topology {
            ConversationTopology::Direct => RunTrigger::DirectConversation,
            ConversationTopology::Group => RunTrigger::GroupConversation,
        },
        goal: goal.to_string(),
        addressed_agents: target_agents,
    };
    let plan = state
        .orchestrator
        .plan_run(request, context.clone(), available_agents)
        .await;

    db::upsert_run(&state.pool, &plan.run)
        .await
        .map_err(|error| error.to_string())?;
    for task in &plan.tasks {
        db::upsert_task(&state.pool, task)
            .await
            .map_err(|error| error.to_string())?;
    }

    state
        .runtime_store
        .log_stage_event(&plan.stage_event)
        .await
        .map_err(|error| error.to_string())?;
    state
        .runtime_store
        .log_decision(&plan.decision_snapshot)
        .await
        .map_err(|error| error.to_string())?;
    for record in &plan.gate_records {
        state
            .runtime_store
            .log_gate_verdict(record)
            .await
            .map_err(|error| error.to_string())?;
    }

    let _ = state
        .memory_store
        .record_episode(EpisodeRequest {
            owner: conversation.owner.clone(),
            namespace: format!("conversations/{}", conversation.id),
            conversation_id: Some(conversation.id.clone()),
            run_id: Some(plan.run.id.clone()),
            episode_kind: EpisodeKind::Message,
            role: Some("operator".to_string()),
            content: message.body.clone(),
            content_type: None,
            source_uri: None,
            entities: Vec::new(),
            tags: lane.participants.clone(),
            importance: Some(0.4),
            occurred_at: Some(message.created_at.clone()),
        })
        .await;

    let _ = state
        .memory_store
        .remember(RememberRequest {
            owner: conversation.owner.clone(),
            namespace: format!("conversations/{}/ledger", conversation.id),
            memory_kind: MemoryKind::Context,
            stability: Stability::Working,
            title: Some(goal.to_string()),
            content: format!("lane={} operator_request={body}", lane.name),
            summary: Some(goal.to_string()),
            confidence: Some(0.6),
            importance: Some(0.4),
            valid_from: None,
            valid_to: None,
            sources: Vec::new(),
            tags: lane.participants.clone(),
            entities: Vec::new(),
        })
        .await;

    let artifact = persist_run_artifact(state, &plan.run, &conversation.owner, goal);
    db::insert_artifact(&state.pool, &artifact)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ConversationEnvelope {
        conversation: plan.conversation,
        lane,
        message: plan.message,
        run: plan.run,
        tasks: plan.tasks,
        artifacts: vec![artifact],
        context: plan.context,
        gate_verdicts: plan.gate_verdicts,
        stage_event: plan.stage_event,
        decision: plan.decision,
    })
}

fn persist_run_artifact(
    state: &AppState,
    run: &RunSpec,
    owner: &OwnerRef,
    goal: &str,
) -> ArtifactSpec {
    let owner_root = state.runtime_paths.owner_run_artifact_dir(owner, &run.id);
    let _ = fs::create_dir_all(&owner_root);
    let relative_path = state
        .runtime_paths
        .owner_run_artifact_relative_path(owner, &run.id);

    let _ = fs::write(
        owner_root.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": run.id,
            "conversation_id": run.conversation_id,
            "lane_id": run.lane_id,
            "owner": owner,
            "goal": goal
        }))
        .unwrap_or_default(),
    );

    ArtifactSpec {
        id: format!("art-{}", Uuid::new_v4()),
        owner: owner.clone(),
        run_id: run.id.clone(),
        conversation_id: Some(run.conversation_id.clone()),
        lane_id: run.lane_id.clone(),
        kind: ArtifactKind::Summary,
        relative_path,
        created_at: now_iso(),
    }
}

fn resolve_owner(
    topology: &ConversationTopology,
    space_id: Option<&str>,
    agent_ids: &[String],
) -> OwnerRef {
    match topology {
        ConversationTopology::Direct => OwnerRef::agent(agent_ids[0].clone()),
        ConversationTopology::Group => {
            if let Some(space_id) = space_id {
                OwnerRef::space(space_id.to_string())
            } else {
                OwnerRef::global("global")
            }
        }
    }
}

type StaticMessages = &'static [(&'static str, &'static str)];
type StaticCatalog = &'static [(&'static str, StaticMessages)];

fn parse_namespaces(value: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .filter_map(|item| {
            let namespace = item.to_string();
            if seen.insert(namespace.clone()) {
                Some(namespace)
            } else {
                None
            }
        })
        .collect()
}

fn builtin_message_namespaces() -> Vec<String> {
    vec![
        "web".to_string(),
        "settings".to_string(),
        "ext.observatory".to_string(),
    ]
}

fn builtin_message_bundle(
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<UiMessageBundleResponse> {
    const WEB_ZH_CN: StaticMessages = &[("web.title", "Ennoia")];
    const WEB_EN_US: StaticMessages = &[("web.title", "Ennoia")];
    const SETTINGS_ZH_CN: StaticMessages = &[("settings.personal.saved", "偏好已保存。")];
    const SETTINGS_EN_US: StaticMessages = &[("settings.personal.saved", "Preferences saved.")];
    const OBSERVATORY_ZH_CN: StaticMessages = &[
        ("ext.observatory.page.events", "观测台"),
        ("ext.observatory.panel.timeline", "事件时间线"),
        ("ext.observatory.theme.daybreak", "Daybreak"),
        ("ext.observatory.command.open", "打开观测台"),
    ];
    const OBSERVATORY_EN_US: StaticMessages = &[
        ("ext.observatory.page.events", "Observatory"),
        ("ext.observatory.panel.timeline", "Event Timeline"),
        ("ext.observatory.theme.daybreak", "Daybreak"),
        ("ext.observatory.command.open", "Open Observatory"),
    ];

    let (source, version, catalogs): (&str, &str, StaticCatalog) = match namespace {
        "web" => (
            "builtin:web",
            "1",
            &[("zh-CN", WEB_ZH_CN), ("en-US", WEB_EN_US)],
        ),
        "settings" => (
            "builtin:settings",
            "1",
            &[("zh-CN", SETTINGS_ZH_CN), ("en-US", SETTINGS_EN_US)],
        ),
        "ext.observatory" => (
            "builtin:ext.observatory",
            "1",
            &[("zh-CN", OBSERVATORY_ZH_CN), ("en-US", OBSERVATORY_EN_US)],
        ),
        _ => return None,
    };

    let (resolved_locale, messages) = select_messages_for_locale(locale, fallback_locale, catalogs);

    Some(UiMessageBundleResponse {
        locale: locale.to_string(),
        resolved_locale: resolved_locale.to_string(),
        namespace: namespace.to_string(),
        messages: messages
            .iter()
            .map(|(key, value)| (key.to_string(), value.to_string()))
            .collect(),
        source: source.to_string(),
        version: version.to_string(),
    })
}

fn extension_message_bundle(
    locales: &[RegisteredLocaleContribution],
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<UiMessageBundleResponse> {
    let contribution =
        select_registered_locale_contribution(locales, locale, fallback_locale, namespace)?;
    let source_root = PathBuf::from(&contribution.install_dir);
    let message_path =
        resolve_safe_extension_asset(&source_root, &contribution.locale.entry).ok()?;
    let messages = fs::read_to_string(message_path).ok()?;
    let parsed = serde_json::from_str::<HashMap<String, String>>(&messages).ok()?;

    Some(UiMessageBundleResponse {
        locale: locale.to_string(),
        resolved_locale: contribution.locale.locale.clone(),
        namespace: namespace.to_string(),
        messages: parsed,
        source: format!("extension:{}", contribution.extension_id),
        version: contribution.locale.version.clone(),
    })
}

fn select_registered_locale_contribution<'a>(
    locales: &'a [RegisteredLocaleContribution],
    locale: &str,
    fallback_locale: &str,
    namespace: &str,
) -> Option<&'a RegisteredLocaleContribution> {
    let candidates = locales
        .iter()
        .filter(|item| item.locale.namespace == namespace)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }

    find_registered_locale_match(&candidates, locale)
        .or_else(|| find_registered_locale_match(&candidates, fallback_locale))
}

fn find_registered_locale_match<'a>(
    candidates: &[&'a RegisteredLocaleContribution],
    locale: &str,
) -> Option<&'a RegisteredLocaleContribution> {
    let normalized = locale.to_lowercase();
    let language = normalized.split('-').next().unwrap_or_default();

    candidates
        .iter()
        .copied()
        .find(|item| item.locale.locale.to_lowercase() == normalized)
        .or_else(|| {
            candidates.iter().copied().find(|item| {
                item.locale
                    .locale
                    .to_lowercase()
                    .split('-')
                    .next()
                    .unwrap_or_default()
                    == language
            })
        })
}

fn select_messages_for_locale(
    locale: &str,
    fallback_locale: &str,
    catalogs: StaticCatalog,
) -> (&'static str, StaticMessages) {
    let normalized = locale.to_lowercase();
    let language = normalized.split('-').next().unwrap_or_default();
    let fallback_normalized = fallback_locale.to_lowercase();
    let fallback_language = fallback_normalized.split('-').next().unwrap_or_default();

    catalogs
        .iter()
        .find(|(candidate, _)| candidate.to_lowercase() == normalized)
        .copied()
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| {
                    candidate
                        .to_lowercase()
                        .split('-')
                        .next()
                        .unwrap_or_default()
                        == language
                })
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| candidate.to_lowercase() == fallback_normalized)
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| {
                    candidate
                        .to_lowercase()
                        .split('-')
                        .next()
                        .unwrap_or_default()
                        == fallback_language
                })
                .copied()
        })
        .or_else(|| {
            catalogs
                .iter()
                .find(|(candidate, _)| candidate.eq_ignore_ascii_case("en-US"))
                .copied()
        })
        .or_else(|| catalogs.first().copied())
        .unwrap_or(("en-US", &[]))
}

fn resolve_addressed_agents(
    conversation: &ConversationSpec,
    lane: &LaneSpec,
    addressed_agents: Vec<String>,
) -> Vec<String> {
    if !addressed_agents.is_empty() {
        return addressed_agents;
    }

    let source = if !lane.participants.is_empty() {
        &lane.participants
    } else {
        &conversation.participants
    };
    source
        .iter()
        .filter(|participant| participant.as_str() != "operator")
        .cloned()
        .collect()
}

fn build_participants(agent_ids: &[String]) -> Vec<String> {
    let mut participants = vec!["operator".to_string()];
    participants.extend(agent_ids.iter().cloned());
    participants
}

fn default_conversation_title(paths: &ennoia_paths::RuntimePaths, agent_ids: &[String]) -> String {
    let known_agents = load_agent_configs(paths).unwrap_or_default();
    if agent_ids.len() == 1 {
        let agent_id = &agent_ids[0];
        let label = known_agents
            .into_iter()
            .find(|agent| agent.id == *agent_id)
            .map(|agent| agent.display_name)
            .unwrap_or_else(|| agent_id.clone());
        return format!("与 {label} 的会话");
    }

    let labels = agent_ids
        .iter()
        .take(3)
        .map(|agent_id| {
            known_agents
                .iter()
                .find(|agent| agent.id == *agent_id)
                .map(|agent| agent.display_name.clone())
                .unwrap_or_else(|| agent_id.clone())
        })
        .collect::<Vec<_>>();
    if labels.is_empty() {
        "多 Agent 协作会话".to_string()
    } else if agent_ids.len() > 3 {
        format!("{} 等 {} 个 Agent", labels.join("、"), agent_ids.len())
    } else {
        format!("{} 协作会话", labels.join("、"))
    }
}

fn normalize_agent_ids(paths: &ennoia_paths::RuntimePaths, requested: &[String]) -> Vec<String> {
    let known: HashSet<String> = load_agent_configs(paths)
        .unwrap_or_default()
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    requested
        .iter()
        .filter(|agent_id| known.contains(agent_id.as_str()))
        .cloned()
        .collect()
}

fn infer_topology_from_agent_count(
    topology: ConversationTopology,
    agent_ids: &[String],
) -> ConversationTopology {
    if agent_ids.len() <= 1 {
        ConversationTopology::Direct
    } else {
        match topology {
            ConversationTopology::Direct | ConversationTopology::Group => {
                ConversationTopology::Group
            }
        }
    }
}

fn build_default_lane_name(agent_ids: &[String]) -> String {
    if agent_ids.len() <= 1 {
        "私聊".to_string()
    } else {
        "群聊".to_string()
    }
}

fn build_default_lane_goal(agent_ids: &[String]) -> String {
    if agent_ids.len() <= 1 {
        "与目标 Agent 持续推进当前问题".to_string()
    } else {
        "在多 Agent 协作中持续推进当前问题".to_string()
    }
}

fn select_lane<'a>(lanes: &'a [LaneSpec], lane_id: Option<&str>) -> Option<LaneSpec> {
    if let Some(lane_id) = lane_id {
        return lanes.iter().find(|lane| lane.id == lane_id).cloned();
    }
    lanes.first().cloned()
}

fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn conversation_topology_from_value(value: &str) -> Option<ConversationTopology> {
    match value {
        "direct" => Some(ConversationTopology::Direct),
        "group" => Some(ConversationTopology::Group),
        _ => None,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

const BUILTIN_THEME_IDS: &[&str] = &[
    "system",
    "ennoia.midnight",
    "ennoia.paper",
    "observatory.daybreak",
];

fn ensure_supported_locale(
    state: &AppState,
    request: &RequestContext,
    locale: String,
) -> Result<String, ApiError> {
    if state
        .ui_config
        .available_locales
        .iter()
        .any(|item| item == &locale)
    {
        return Ok(locale);
    }
    Err(scoped(
        ApiError::bad_request(format!("unsupported locale '{locale}'")),
        request,
    ))
}

fn ensure_supported_theme_id(
    state: &AppState,
    request: &RequestContext,
    theme_id: String,
) -> Result<String, ApiError> {
    if supported_theme_ids(state).contains(&theme_id) {
        return Ok(theme_id);
    }
    Err(scoped(
        ApiError::bad_request(format!("unsupported theme '{theme_id}'")),
        request,
    ))
}

fn validate_ui_preference_payload(
    state: &AppState,
    request: &RequestContext,
    payload: &UiPreferencePayload,
) -> Result<(), ApiError> {
    if let Some(locale) = &payload.locale {
        ensure_supported_locale(state, request, locale.clone())?;
    }
    if let Some(theme_id) = &payload.theme_id {
        ensure_supported_theme_id(state, request, theme_id.clone())?;
    }
    Ok(())
}

fn supported_theme_ids(state: &AppState) -> HashSet<String> {
    let mut ids = BUILTIN_THEME_IDS
        .iter()
        .map(|item| item.to_string())
        .collect::<HashSet<_>>();
    for theme in state.extensions.snapshot().themes {
        ids.insert(theme.theme.id);
    }
    ids
}

fn resolve_safe_extension_asset(root: &StdPath, entry: &str) -> std::io::Result<PathBuf> {
    let canonical_root = fs::canonicalize(root)?;
    let canonical_asset = fs::canonicalize(root.join(entry))?;
    if !canonical_asset.starts_with(&canonical_root) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "extension asset must stay inside the extension root",
        ));
    }
    Ok(canonical_asset)
}

fn to_preference_record(row: db::UiPreferenceRow) -> UiPreferenceRecord {
    UiPreferenceRecord {
        subject_id: row.subject_id,
        preference: row.preference,
    }
}

fn merge_ui_preference(
    current: Option<&UiPreference>,
    payload: UiPreferencePayload,
) -> UiPreference {
    UiPreference {
        locale: payload
            .locale
            .or_else(|| current.and_then(|item| item.locale.clone())),
        theme_id: payload
            .theme_id
            .or_else(|| current.and_then(|item| item.theme_id.clone())),
        time_zone: payload
            .time_zone
            .or_else(|| current.and_then(|item| item.time_zone.clone())),
        date_style: payload
            .date_style
            .or_else(|| current.and_then(|item| item.date_style.clone())),
        density: payload
            .density
            .or_else(|| current.and_then(|item| item.density.clone())),
        motion: payload
            .motion
            .or_else(|| current.and_then(|item| item.motion.clone())),
        version: current.map(|item| item.version + 1).unwrap_or(1),
        updated_at: now_iso(),
    }
}

fn ensure_full_config_set(mut rows: Vec<ConfigEntry>) -> Vec<ConfigEntry> {
    let have: HashSet<String> = rows.iter().map(|row| row.key.clone()).collect();
    for key in ALL_CONFIG_KEYS {
        if !have.contains(*key) {
            rows.push(ConfigEntry {
                key: key.to_string(),
                payload_json: "{}".to_string(),
                enabled: true,
                version: 0,
                updated_by: None,
                updated_at: String::new(),
            });
        }
    }
    rows.sort_by(|left, right| left.key.cmp(&right.key));
    rows
}

fn list_extension_workbench_records(state: &AppState) -> Vec<ExtensionWorkbenchRecord> {
    let mut by_id = state
        .extensions
        .snapshot()
        .extensions
        .into_iter()
        .map(|item| (item.id.clone(), map_extension_workbench_record(&item)))
        .collect::<HashMap<_, _>>();

    if let Ok(registry) = read_registry_file(&state.runtime_paths.extensions_registry_file()) {
        for record in registry.extensions.into_iter().filter(|item| !item.removed) {
            if by_id.contains_key(&record.id) {
                continue;
            }
            by_id.insert(
                record.id.clone(),
                ExtensionWorkbenchRecord {
                    id: record.id.clone(),
                    name: record.id.clone(),
                    enabled: record.enabled,
                    status: if record.enabled {
                        "ready".to_string()
                    } else {
                        "disabled".to_string()
                    },
                    version: "unknown".to_string(),
                    kind: "extension".to_string(),
                    source_mode: record.source,
                    install_dir: record.path.clone(),
                    source_root: record.path,
                    diagnostics: Vec::new(),
                },
            );
        }
    }

    let mut items = by_id.into_values().collect::<Vec<_>>();
    items.sort_by(|left, right| left.id.cmp(&right.id));
    items
}

fn map_extension_workbench_record(
    extension: &ResolvedExtensionSnapshot,
) -> ExtensionWorkbenchRecord {
    ExtensionWorkbenchRecord {
        id: extension.id.clone(),
        name: extension.name.clone(),
        enabled: !matches!(extension.health, ennoia_kernel::ExtensionHealth::Stopped),
        status: match extension.health {
            ennoia_kernel::ExtensionHealth::Ready => "ready".to_string(),
            ennoia_kernel::ExtensionHealth::Failed => "failed".to_string(),
            ennoia_kernel::ExtensionHealth::Degraded => "degraded".to_string(),
            ennoia_kernel::ExtensionHealth::Stopped => "disabled".to_string(),
            ennoia_kernel::ExtensionHealth::Discovering => "discovering".to_string(),
            ennoia_kernel::ExtensionHealth::Resolving => "resolving".to_string(),
        },
        version: extension.version.clone(),
        kind: format!("{:?}", extension.kind),
        source_mode: format!("{:?}", extension.source_mode),
        install_dir: extension.install_dir.clone(),
        source_root: extension.source_root.clone(),
        diagnostics: extension.diagnostics.clone(),
    }
}
