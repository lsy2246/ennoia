use std::fs;

use axum::{
    extract::{Extension, Path, State},
    http::{HeaderMap, StatusCode},
    middleware as axum_middleware,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use ennoia_auth::tokens;
use ennoia_contract::ApiError;
use ennoia_extension_host::{
    ExtensionRegistrySnapshot, RegisteredExtensionSnapshot, RegisteredLocaleContribution,
    RegisteredPageContribution, RegisteredPanelContribution, RegisteredThemeContribution,
};
use ennoia_kernel::{
    ApiKey, ArtifactKind, ArtifactSpec, AssembleRequest, AuthMode, BootstrapState,
    ConfigChangeRecord, ConfigEntry, ConfigStore, ContextView, EnqueueRequest, EpisodeKind,
    EpisodeRequest, JobKind, JobRecord, LocalizedText, MemoryKind, MemoryRecord, MemorySource,
    MessageRole, MessageSpec, OwnerKind, OwnerRef, RecallMode, RecallQuery, RecallResult,
    RememberReceipt, RememberRequest, ReviewAction, ReviewActionKind, ReviewReceipt, ScheduleKind,
    Session, Stability, SystemConfig, ThreadKind, ThreadSpec, UiConfig, UiPreference,
    UiPreferenceRecord, UpdateUserRequest, User, UserRole, ALL_CONFIG_KEYS, CONFIG_KEY_AUTH,
    CONFIG_KEY_BOOTSTRAP,
};
use ennoia_observability::RequestContext;
use ennoia_orchestrator::{RunRequest, RunTrigger};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::app::AppState;
use crate::db::{self, JobRow};
use crate::middleware::{
    auth_middleware, body_limit_middleware, cors_middleware, logging_middleware,
    rate_limit_middleware, request_context_middleware, require_admin, timeout_middleware,
    AuthedUser,
};

type ApiResult<T> = Result<Json<T>, ApiError>;
type StatusResult = Result<StatusCode, ApiError>;

fn scoped(error: ApiError, request: &RequestContext) -> ApiError {
    error.with_request_id(&request.request_id)
}

pub fn build_router(state: AppState) -> Router {
    let admin = Router::new()
        .route("/api/v1/admin/config", get(config_list))
        .route(
            "/api/v1/admin/config/{key}",
            get(config_get).put(config_put),
        )
        .route("/api/v1/admin/config/{key}/history", get(config_history))
        .route("/api/v1/admin/config/snapshot", get(config_snapshot))
        .route("/api/v1/admin/users", get(users_list).post(users_create))
        .route(
            "/api/v1/admin/users/{user_id}",
            get(users_get).put(users_update).delete(users_delete),
        )
        .route(
            "/api/v1/admin/users/{user_id}/reset-password",
            post(users_reset_password),
        )
        .route("/api/v1/admin/sessions", get(sessions_list))
        .route(
            "/api/v1/admin/sessions/{session_id}",
            axum::routing::delete(sessions_delete),
        )
        .route(
            "/api/v1/admin/api-keys",
            get(api_keys_list).post(api_keys_create),
        )
        .route(
            "/api/v1/admin/api-keys/{key_id}",
            axum::routing::delete(api_keys_delete),
        );

    let auth_public = Router::new()
        .route("/api/v1/auth/register", post(auth_register))
        .route("/api/v1/auth/login", post(auth_login))
        .route("/api/v1/auth/logout", post(auth_logout))
        .route("/api/v1/auth/me", get(auth_me))
        .route("/api/v1/auth/refresh", post(auth_refresh));

    let bootstrap = Router::new()
        .route("/api/v1/bootstrap/state", get(bootstrap_state_handler))
        .route("/api/v1/bootstrap", post(bootstrap_complete));

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/ui/runtime", get(ui_runtime))
        .route(
            "/api/v1/me/ui-preferences",
            get(me_ui_preferences).put(me_ui_preferences_put),
        )
        .route(
            "/api/v1/spaces/{space_id}/ui-preferences",
            get(space_ui_preferences).put(space_ui_preferences_put),
        )
        .route("/api/v1/extensions", get(extensions))
        .route("/api/v1/extensions/registry", get(extension_registry))
        .route("/api/v1/extensions/pages", get(extension_pages))
        .route("/api/v1/extensions/panels", get(extension_panels))
        .route("/api/v1/agents", get(agents))
        .route("/api/v1/spaces", get(spaces))
        .route("/api/v1/threads", get(threads))
        .route("/api/v1/threads/{thread_id}/messages", get(thread_messages))
        .route("/api/v1/threads/{thread_id}/runs", get(thread_runs))
        .route(
            "/api/v1/threads/private/messages",
            post(create_private_message),
        )
        .route("/api/v1/threads/space/messages", post(create_space_message))
        .route("/api/v1/runs", get(runs))
        .route("/api/v1/runs/{run_id}/tasks", get(run_tasks))
        .route("/api/v1/runs/{run_id}/artifacts", get(run_artifacts))
        .route("/api/v1/runs/{run_id}/stages", get(run_stages))
        .route("/api/v1/runs/{run_id}/decisions", get(run_decisions))
        .route("/api/v1/runs/{run_id}/gates", get(run_gates))
        .route("/api/v1/tasks", get(tasks))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/memories", get(memories_list).post(memories_create))
        .route("/api/v1/memories/recall", post(memories_recall))
        .route("/api/v1/memories/review", post(memories_review))
        .route("/api/v1/jobs", get(jobs_list).post(jobs_create))
        .merge(admin)
        .merge(auth_public)
        .merge(bootstrap)
        .layer(axum_middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
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
    shell_title: LocalizedText,
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
    user_preference: Option<UiPreferenceRecord>,
    space_preferences: Vec<UiPreferenceRecord>,
    versions: UiRuntimeVersionsResponse,
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
struct PrivateMessageRequest {
    agent_id: String,
    body: String,
    goal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct SpaceMessageRequest {
    space_id: String,
    addressed_agents: Vec<String>,
    body: String,
    goal: Option<String>,
}

#[derive(Debug, Deserialize)]
struct CreateJobRequest {
    owner_kind: String,
    owner_id: String,
    job_kind: String,
    schedule_kind: String,
    schedule_value: String,
    payload: Option<JsonValue>,
    max_retries: Option<u32>,
    run_at: Option<String>,
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
    thread_id: Option<String>,
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
    thread: ThreadSpec,
    message: MessageSpec,
    run: ennoia_kernel::RunSpec,
    tasks: Vec<ennoia_kernel::TaskSpec>,
    artifacts: Vec<ArtifactSpec>,
    context: ContextView,
    gate_verdicts: Vec<ennoia_kernel::GateVerdict>,
    stage_event: ennoia_kernel::RunStageEvent,
    decision: ennoia_kernel::Decision,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "Ennoia",
    })
}

async fn overview(State(state): State<AppState>) -> Json<OverviewResponse> {
    let thread_count = db::count_rows(&state.pool, db::CountTable::Threads)
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
        shell_title: state.ui_config.shell_title.clone(),
        default_theme: state.ui_config.default_theme.clone(),
        modules: state.overview.modules,
        counts: serde_json::json!({
            "agents": state.agents.len(),
            "spaces": state.spaces.len(),
            "extensions": state.extensions.items().len(),
            "threads": thread_count,
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

async fn ui_runtime(
    State(state): State<AppState>,
    Extension(user): Extension<AuthedUser>,
) -> Json<UiRuntimeResponse> {
    let snapshot = state.extensions.snapshot();
    let user_preference = if user.id == "anonymous" {
        None
    } else {
        db::get_user_ui_preference(&state.pool, &user.id)
            .await
            .ok()
            .flatten()
            .map(to_preference_record)
    };
    let space_preferences = db::list_space_ui_preferences(&state.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(to_preference_record)
        .collect::<Vec<_>>();
    let registry_version = (snapshot.pages.len()
        + snapshot.panels.len()
        + snapshot.themes.len()
        + snapshot.locales.len()) as u64;
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
        },
        user_preference,
        space_preferences,
        versions: UiRuntimeVersionsResponse {
            registry: registry_version,
            preferences: preference_version,
        },
    })
}

async fn me_ui_preferences(
    State(state): State<AppState>,
    Extension(user): Extension<AuthedUser>,
) -> Json<Option<UiPreferenceRecord>> {
    if user.id == "anonymous" {
        return Json(None);
    }
    Json(
        db::get_user_ui_preference(&state.pool, &user.id)
            .await
            .ok()
            .flatten()
            .map(to_preference_record),
    )
}

async fn me_ui_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(user): Extension<AuthedUser>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    let current = if user.id == "anonymous" {
        None
    } else {
        db::get_user_ui_preference(&state.pool, &user.id)
            .await
            .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?
    };
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = db::upsert_user_ui_preference(&state.pool, &user.id, &preference)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    Ok(Json(to_preference_record(saved)))
}

async fn space_ui_preferences(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(user): Extension<AuthedUser>,
    Path(space_id): Path<String>,
) -> ApiResult<Option<UiPreferenceRecord>> {
    require_admin(&user, &state).map_err(|error| scoped(error, &request))?;
    let row = db::get_space_ui_preference(&state.pool, &space_id)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    Ok(Json(row.map(to_preference_record)))
}

async fn space_ui_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(user): Extension<AuthedUser>,
    Path(space_id): Path<String>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    require_admin(&user, &state).map_err(|error| scoped(error, &request))?;
    let current = db::get_space_ui_preference(&state.pool, &space_id)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = db::upsert_space_ui_preference(&state.pool, &space_id, &preference)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    Ok(Json(to_preference_record(saved)))
}

async fn extensions(State(state): State<AppState>) -> Json<Vec<RegisteredExtensionSnapshot>> {
    Json(state.extensions.snapshot().extensions)
}

async fn extension_registry(State(state): State<AppState>) -> Json<ExtensionRegistrySnapshot> {
    Json(state.extensions.snapshot())
}

async fn extension_pages(State(state): State<AppState>) -> Json<Vec<RegisteredPageContribution>> {
    Json(state.extensions.pages())
}

async fn extension_panels(State(state): State<AppState>) -> Json<Vec<RegisteredPanelContribution>> {
    Json(state.extensions.panels())
}

async fn agents(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::AgentConfig>> {
    Json(state.agents)
}

async fn spaces(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::SpaceSpec>> {
    Json(state.spaces)
}

async fn threads(State(state): State<AppState>) -> Json<Vec<ThreadSpec>> {
    Json(db::list_threads(&state.pool).await.unwrap_or_default())
}

async fn thread_messages(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> Json<Vec<MessageSpec>> {
    Json(
        db::list_messages_for_thread(&state.pool, &thread_id)
            .await
            .unwrap_or_default(),
    )
}

async fn thread_runs(
    State(state): State<AppState>,
    Path(thread_id): Path<String>,
) -> Json<Vec<ennoia_kernel::RunSpec>> {
    Json(
        db::list_runs_for_thread(&state.pool, &thread_id)
            .await
            .unwrap_or_default(),
    )
}

async fn runs(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::RunSpec>> {
    Json(db::list_runs(&state.pool).await.unwrap_or_default())
}

async fn run_tasks(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::TaskSpec>> {
    Json(
        db::list_tasks_for_run(&state.pool, &run_id)
            .await
            .unwrap_or_default(),
    )
}

async fn run_artifacts(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ArtifactSpec>> {
    Json(
        db::list_artifacts_for_run(&state.pool, &run_id)
            .await
            .unwrap_or_default(),
    )
}

async fn run_stages(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::RunStageEvent>> {
    Json(
        state
            .runtime_store
            .list_stage_events_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

async fn run_decisions(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::DecisionSnapshot>> {
    Json(
        state
            .runtime_store
            .list_decisions_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

async fn run_gates(
    State(state): State<AppState>,
    Path(run_id): Path<String>,
) -> Json<Vec<ennoia_kernel::GateRecord>> {
    Json(
        state
            .runtime_store
            .list_gate_verdicts_for_run(&run_id)
            .await
            .unwrap_or_default(),
    )
}

async fn tasks(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::TaskSpec>> {
    Json(db::list_tasks(&state.pool).await.unwrap_or_default())
}

async fn artifacts(State(state): State<AppState>) -> Json<Vec<ArtifactSpec>> {
    Json(db::list_artifacts(&state.pool).await.unwrap_or_default())
}

async fn memories_list(State(state): State<AppState>) -> Json<Vec<MemoryRecord>> {
    Json(
        state
            .memory_store
            .list_memories(100)
            .await
            .unwrap_or_default(),
    )
}

async fn memories_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RememberPayload>,
) -> ApiResult<RememberReceipt> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let remember = RememberRequest {
        owner,
        namespace: payload.namespace,
        memory_kind: MemoryKind::from_str(&payload.memory_kind),
        stability: Stability::from_str(&payload.stability),
        title: payload.title,
        content: payload.content,
        summary: payload.summary,
        confidence: payload.confidence,
        importance: payload.importance,
        valid_from: None,
        valid_to: None,
        sources: payload.sources,
        tags: payload.tags,
        entities: payload.entities,
    };
    state
        .memory_store
        .remember(remember)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

async fn memories_recall(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RecallPayload>,
) -> ApiResult<RecallResult> {
    let mode = payload.mode.as_deref().unwrap_or("namespace");
    let mode = match mode {
        "fts" => RecallMode::Fts,
        "hybrid" => RecallMode::Hybrid,
        _ => RecallMode::Namespace,
    };
    let query = RecallQuery {
        owner: OwnerRef {
            kind: owner_kind_from(&payload.owner_kind),
            id: payload.owner_id,
        },
        thread_id: payload.thread_id,
        run_id: payload.run_id,
        query_text: payload.query_text,
        namespace_prefix: payload.namespace_prefix,
        memory_kind: payload.memory_kind.as_deref().map(MemoryKind::from_str),
        mode,
        limit: payload.limit.unwrap_or(20),
    };
    state
        .memory_store
        .recall(query)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

async fn memories_review(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ReviewPayload>,
) -> ApiResult<ReviewReceipt> {
    let action_kind = match payload.action.as_str() {
        "approve" => ReviewActionKind::Approve,
        "reject" => ReviewActionKind::Reject,
        "supersede" => ReviewActionKind::Supersede,
        "retire" => ReviewActionKind::Retire,
        _ => return Err(scoped(ApiError::bad_request("unknown action"), &request)),
    };
    let action = ReviewAction {
        target_memory_id: payload.target_memory_id,
        reviewer: payload.reviewer,
        action: action_kind,
        notes: payload.notes,
    };
    state
        .memory_store
        .review(action)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

async fn jobs_list(State(state): State<AppState>) -> Json<Vec<JobRow>> {
    Json(db::list_jobs(&state.pool).await.unwrap_or_default())
}

async fn jobs_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<CreateJobRequest>,
) -> ApiResult<JobRecord> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let enqueue = EnqueueRequest {
        owner,
        job_kind: JobKind::from_str(&payload.job_kind),
        schedule_kind: ScheduleKind::from_str(&payload.schedule_kind),
        schedule_value: payload.schedule_value,
        payload: payload.payload.unwrap_or(JsonValue::Null),
        max_retries: payload.max_retries,
        run_at: payload.run_at,
    };
    state
        .scheduler_store
        .enqueue(enqueue)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

async fn create_private_message(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<PrivateMessageRequest>,
) -> ApiResult<ConversationEnvelope> {
    let goal = payload.goal.unwrap_or_else(|| payload.body.clone());
    process_private_message(&state, &payload.agent_id, &payload.body, &goal)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error), &request))
}

async fn create_space_message(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<SpaceMessageRequest>,
) -> ApiResult<ConversationEnvelope> {
    let goal = payload.goal.unwrap_or_else(|| payload.body.clone());
    process_space_message(
        &state,
        &payload.space_id,
        &payload.addressed_agents,
        &payload.body,
        &goal,
    )
    .await
    .map(Json)
    .map_err(|error| scoped(ApiError::bad_request(error), &request))
}

async fn process_private_message(
    state: &AppState,
    agent_id: &str,
    body: &str,
    goal: &str,
) -> Result<ConversationEnvelope, String> {
    let now = now_iso();
    let owner = OwnerRef {
        kind: OwnerKind::Agent,
        id: agent_id.to_string(),
    };
    let thread = ThreadSpec {
        id: format!("thread-private-{agent_id}"),
        kind: ThreadKind::Private,
        owner: owner.clone(),
        space_id: None,
        title: format!("与 {agent_id} 的私聊"),
        participants: vec!["user".to_string(), agent_id.to_string()],
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        thread_id: thread.id.clone(),
        sender: "user".to_string(),
        role: MessageRole::User,
        body: body.to_string(),
        mentions: vec![agent_id.to_string()],
        created_at: now.clone(),
    };

    drive_run(
        state,
        thread,
        message,
        owner,
        RunTrigger::DirectMessage,
        goal,
        vec![agent_id.to_string()],
    )
    .await
}

async fn process_space_message(
    state: &AppState,
    space_id: &str,
    addressed_agents: &[String],
    body: &str,
    goal: &str,
) -> Result<ConversationEnvelope, String> {
    let now = now_iso();
    let owner = OwnerRef {
        kind: OwnerKind::Space,
        id: space_id.to_string(),
    };
    let resolved_agents = resolve_space_agents(state, space_id, addressed_agents);
    let mut participants = vec!["user".to_string()];
    participants.extend(resolved_agents.iter().cloned());
    let thread = ThreadSpec {
        id: format!("thread-space-{space_id}"),
        kind: ThreadKind::Space,
        owner: owner.clone(),
        space_id: Some(space_id.to_string()),
        title: format!("{space_id} 协作线程"),
        participants,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        thread_id: thread.id.clone(),
        sender: "user".to_string(),
        role: MessageRole::User,
        body: body.to_string(),
        mentions: resolved_agents.clone(),
        created_at: now.clone(),
    };

    drive_run(
        state,
        thread,
        message,
        owner,
        RunTrigger::SpaceMessage,
        goal,
        resolved_agents,
    )
    .await
}

async fn drive_run(
    state: &AppState,
    thread: ThreadSpec,
    message: MessageSpec,
    owner: OwnerRef,
    trigger: RunTrigger,
    goal: &str,
    addressed_agents: Vec<String>,
) -> Result<ConversationEnvelope, String> {
    db::upsert_thread(&state.pool, &thread)
        .await
        .map_err(|e| e.to_string())?;
    db::insert_message(&state.pool, &message)
        .await
        .map_err(|e| e.to_string())?;

    let assemble = AssembleRequest {
        owner: owner.clone(),
        thread_id: Some(thread.id.clone()),
        run_id: None,
        recent_messages: vec![format!("{}: {}", message.sender, message.body)],
        active_tasks: Vec::new(),
        budget_chars: None,
    };
    let context = state
        .memory_store
        .assemble_context(assemble)
        .await
        .map_err(|e| e.to_string())?;

    let available_agents: Vec<String> = state.agents.iter().map(|a| a.id.clone()).collect();

    let request = RunRequest {
        owner: owner.clone(),
        thread: thread.clone(),
        message: message.clone(),
        trigger,
        goal: goal.to_string(),
        addressed_agents,
    };

    let plan = state
        .orchestrator
        .plan_run(request, context.clone(), available_agents)
        .await;

    db::upsert_run(&state.pool, &plan.run)
        .await
        .map_err(|e| e.to_string())?;
    for task in &plan.tasks {
        db::upsert_task(&state.pool, task)
            .await
            .map_err(|e| e.to_string())?;
    }

    state
        .runtime_store
        .log_stage_event(&plan.stage_event)
        .await
        .map_err(|e| e.to_string())?;
    state
        .runtime_store
        .log_decision(&plan.decision_snapshot)
        .await
        .map_err(|e| e.to_string())?;
    for record in &plan.gate_records {
        state
            .runtime_store
            .log_gate_verdict(record)
            .await
            .map_err(|e| e.to_string())?;
    }

    let _ = state
        .memory_store
        .record_episode(EpisodeRequest {
            owner: owner.clone(),
            namespace: format!("threads/{}", thread.id),
            thread_id: Some(thread.id.clone()),
            run_id: Some(plan.run.id.clone()),
            episode_kind: EpisodeKind::Message,
            role: Some("user".to_string()),
            content: message.body.clone(),
            content_type: None,
            source_uri: None,
            entities: Vec::new(),
            tags: Vec::new(),
            importance: None,
            occurred_at: Some(message.created_at.clone()),
        })
        .await;

    let artifact = persist_run_artifact(state, &owner, &plan.run.id, goal);
    db::insert_artifact(&state.pool, &artifact)
        .await
        .map_err(|e| e.to_string())?;

    Ok(ConversationEnvelope {
        thread: plan.thread,
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
    owner: &OwnerRef,
    run_id: &str,
    goal: &str,
) -> ArtifactSpec {
    let owner_root = state.runtime_paths.owner_run_artifact_dir(owner, run_id);

    let _ = fs::create_dir_all(&owner_root);
    let relative_path = state
        .runtime_paths
        .owner_run_artifact_relative_path(owner, run_id);

    let _ = fs::write(
        owner_root.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": run_id,
            "owner": owner,
            "goal": goal
        }))
        .unwrap_or_default(),
    );

    ArtifactSpec {
        id: format!("art-{}", Uuid::new_v4()),
        owner: owner.clone(),
        run_id: run_id.to_string(),
        kind: ArtifactKind::Report,
        relative_path,
        created_at: now_iso(),
    }
}

fn resolve_space_agents(
    state: &AppState,
    space_id: &str,
    addressed_agents: &[String],
) -> Vec<String> {
    if !addressed_agents.is_empty() {
        return addressed_agents.to_vec();
    }

    state
        .spaces
        .iter()
        .find(|space| space.id == space_id)
        .map(|space| space.default_agents.clone())
        .filter(|agents| !agents.is_empty())
        .unwrap_or_else(|| vec!["coder".to_string(), "planner".to_string()])
}

fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
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

// ========== Admin config API ==========

#[derive(Debug, Deserialize)]
struct ConfigPutPayload {
    payload: JsonValue,
    updated_by: Option<String>,
}

async fn config_list(State(state): State<AppState>) -> Json<Vec<ConfigEntry>> {
    let raw = state.system_config.store.list().await.unwrap_or_default();
    Json(ensure_full_config_set(raw))
}

async fn config_get(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(key): Path<String>,
) -> ApiResult<ConfigEntry> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err(scoped(
            ApiError::not_found(format!("unknown config key '{key}'")),
            &request,
        ));
    }
    state
        .system_config
        .store
        .get(&key)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("config '{key}' not initialized")),
                &request,
            )
        })
}

async fn config_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(key): Path<String>,
    Json(payload): Json<ConfigPutPayload>,
) -> ApiResult<ConfigEntry> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err(scoped(
            ApiError::not_found(format!("unknown config key '{key}'")),
            &request,
        ));
    }
    let entry = state
        .system_config
        .store
        .put(&key, &payload.payload, payload.updated_by.as_deref())
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;

    let applied = state
        .system_config
        .apply(&key, &payload.payload)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))?;
    if !applied {
        return Err(scoped(
            ApiError::bad_request(format!("unsupported key '{key}'")),
            &request,
        ));
    }

    Ok(Json(entry))
}

async fn config_history(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Json<Vec<ConfigChangeRecord>> {
    Json(
        state
            .system_config
            .store
            .history(&key, 50)
            .await
            .unwrap_or_default(),
    )
}

async fn config_snapshot(State(state): State<AppState>) -> Json<SystemConfig> {
    Json(state.system_config.snapshot())
}

fn ensure_full_config_set(mut rows: Vec<ConfigEntry>) -> Vec<ConfigEntry> {
    let have: std::collections::HashSet<String> = rows.iter().map(|r| r.key.clone()).collect();
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
    rows.sort_by(|a, b| a.key.cmp(&b.key));
    rows
}

// ========== Auth public API ==========

#[derive(Debug, Deserialize)]
struct RegisterPayload {
    username: String,
    password: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LoginPayload {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
struct LoginResponse {
    user: User,
    token: String,
    token_kind: &'static str,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct RegisterResponse {
    user: User,
}

async fn auth_register(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RegisterPayload>,
) -> ApiResult<RegisterResponse> {
    let cfg = state.system_config.auth.load();
    if !cfg.allow_registration {
        return Err(scoped(
            ApiError::forbidden("registration is disabled"),
            &request,
        ));
    }
    let user = state
        .auth_service
        .register(
            &payload.username,
            &payload.password,
            payload.display_name,
            payload.email,
            UserRole::User,
        )
        .await
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))?;
    Ok(Json(RegisterResponse { user }))
}

async fn auth_login(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    headers: HeaderMap,
    Json(payload): Json<LoginPayload>,
) -> ApiResult<LoginResponse> {
    let cfg = state.system_config.auth.load();
    let ttl = cfg.session_ttl_seconds;
    let user_agent = headers
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string());
    let ip = headers
        .get("x-forwarded-for")
        .or_else(|| headers.get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or("").trim().to_string());

    match cfg.mode {
        AuthMode::Jwt => {
            // JWT still requires username+password to authenticate against the user store.
            let (user, password_hash) = state
                .user_store
                .get_by_username(&payload.username)
                .await
                .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?
                .ok_or_else(|| scoped(ApiError::unauthorized("invalid credentials"), &request))?;
            let ok = ennoia_auth::verify_password(&payload.password, &password_hash)
                .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
            if !ok {
                return Err(scoped(
                    ApiError::unauthorized("invalid credentials"),
                    &request,
                ));
            }
            let secret = cfg
                .jwt_secret
                .as_deref()
                .ok_or_else(|| scoped(ApiError::internal("jwt secret not configured"), &request))?;
            let token = state
                .auth_service
                .mint_jwt(&user, secret, ttl)
                .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
            let expires_at = (Utc::now() + chrono::Duration::seconds(ttl as i64)).to_rfc3339();
            let _ = state.user_store.touch_login(&user.id, &now_iso()).await;
            Ok(Json(LoginResponse {
                user,
                token,
                token_kind: "jwt",
                expires_at,
            }))
        }
        _ => {
            let outcome = state
                .auth_service
                .login(&payload.username, &payload.password, ttl, user_agent, ip)
                .await
                .map_err(|e| scoped(ApiError::unauthorized(e.to_string()), &request))?;
            Ok(Json(LoginResponse {
                user: outcome.user,
                token: outcome.raw_token,
                token_kind: "session",
                expires_at: outcome.session.expires_at,
            }))
        }
    }
}

async fn auth_logout(State(state): State<AppState>, headers: HeaderMap) -> StatusResult {
    let token = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .map(|s| s.trim().to_string());
    if let Some(t) = token {
        let _ = state.auth_service.logout(&t).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

async fn auth_me(Extension(user): Extension<AuthedUser>) -> Json<AuthedUser> {
    Json(user)
}

#[derive(Debug, Deserialize)]
struct RefreshPayload {
    token: String,
}

async fn auth_refresh(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RefreshPayload>,
) -> ApiResult<LoginResponse> {
    let cfg = state.system_config.auth.load();
    let secret = cfg
        .jwt_secret
        .as_deref()
        .ok_or_else(|| scoped(ApiError::bad_request("jwt secret not configured"), &request))?;
    let (user, _claims) = state
        .auth_service
        .authenticate_jwt(&payload.token, secret)
        .await
        .map_err(|e| scoped(ApiError::unauthorized(e.to_string()), &request))?;
    let ttl = cfg.session_ttl_seconds;
    let token = state
        .auth_service
        .mint_jwt(&user, secret, ttl)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    let expires_at = (Utc::now() + chrono::Duration::seconds(ttl as i64)).to_rfc3339();
    Ok(Json(LoginResponse {
        user,
        token,
        token_kind: "jwt",
        expires_at,
    }))
}

// ========== Admin users API ==========

#[derive(Debug, Deserialize)]
struct AdminCreateUserPayload {
    username: String,
    password: String,
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminUpdateUserPayload {
    #[serde(default)]
    display_name: Option<String>,
    #[serde(default)]
    email: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AdminResetPasswordPayload {
    new_password: String,
}

async fn users_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(user): Extension<AuthedUser>,
) -> ApiResult<Vec<User>> {
    require_admin(&user, &state).map_err(|error| scoped(error, &request))?;
    state
        .user_store
        .list()
        .await
        .map(Json)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))
}

async fn users_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(user): Extension<AuthedUser>,
    Json(payload): Json<AdminCreateUserPayload>,
) -> ApiResult<User> {
    require_admin(&user, &state).map_err(|error| scoped(error, &request))?;
    let role = payload
        .role
        .as_deref()
        .map(UserRole::from_str)
        .unwrap_or(UserRole::User);
    state
        .auth_service
        .register(
            &payload.username,
            &payload.password,
            payload.display_name,
            payload.email,
            role,
        )
        .await
        .map(Json)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

async fn users_get(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(user_id): Path<String>,
) -> ApiResult<User> {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .user_store
        .get(&user_id)
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found(format!("user {user_id}")), &request))
}

async fn users_update(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(user_id): Path<String>,
    Json(payload): Json<AdminUpdateUserPayload>,
) -> ApiResult<User> {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    let update = UpdateUserRequest {
        display_name: payload.display_name,
        email: payload.email,
        role: payload.role.as_deref().map(UserRole::from_str),
    };
    state
        .user_store
        .update(&user_id, update)
        .await
        .map(Json)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

async fn users_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(user_id): Path<String>,
) -> StatusResult {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .user_store
        .delete(&user_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

async fn users_reset_password(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(user_id): Path<String>,
    Json(payload): Json<AdminResetPasswordPayload>,
) -> StatusResult {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    let hash = ennoia_auth::hash_password(&payload.new_password)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))?;
    state
        .user_store
        .set_password(&user_id, &hash)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

// ========== Admin sessions API ==========

async fn sessions_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
) -> ApiResult<Vec<Session>> {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .session_store
        .list_all()
        .await
        .map(Json)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))
}

async fn sessions_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(session_id): Path<String>,
) -> StatusResult {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .session_store
        .delete(&session_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

// ========== Admin API keys API ==========

#[derive(Debug, Deserialize)]
struct AdminCreateApiKeyPayload {
    user_id: String,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    expires_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct AdminCreateApiKeyResponse {
    key: ApiKey,
    raw_key: String,
}

async fn api_keys_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
) -> ApiResult<Vec<ApiKey>> {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .api_key_store
        .list()
        .await
        .map(Json)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))
}

async fn api_keys_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Json(payload): Json<AdminCreateApiKeyPayload>,
) -> ApiResult<AdminCreateApiKeyResponse> {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    let (key, raw) = state
        .auth_service
        .create_api_key(
            &payload.user_id,
            payload.label,
            payload.scopes,
            payload.expires_at,
        )
        .await
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))?;
    Ok(Json(AdminCreateApiKeyResponse { key, raw_key: raw }))
}

async fn api_keys_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Extension(caller): Extension<AuthedUser>,
    Path(key_id): Path<String>,
) -> StatusResult {
    require_admin(&caller, &state).map_err(|error| scoped(error, &request))?;
    state
        .api_key_store
        .delete(&key_id)
        .await
        .map(|_| StatusCode::NO_CONTENT)
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))
}

// ========== Bootstrap API ==========

#[derive(Debug, Deserialize)]
struct BootstrapPayload {
    admin_username: String,
    admin_password: String,
    #[serde(default)]
    admin_display_name: Option<String>,
    #[serde(default)]
    auth_mode: Option<String>,
    #[serde(default)]
    allow_registration: Option<bool>,
}

#[derive(Debug, Serialize)]
struct BootstrapResponse {
    user: User,
    bootstrap: BootstrapState,
    jwt_secret_generated: bool,
}

async fn bootstrap_state_handler(State(state): State<AppState>) -> Json<BootstrapState> {
    Json((**state.system_config.bootstrap.load()).clone())
}

async fn bootstrap_complete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<BootstrapPayload>,
) -> ApiResult<BootstrapResponse> {
    let current = (**state.system_config.bootstrap.load()).clone();
    if current.completed {
        return Err(scoped(
            ApiError::conflict("bootstrap already completed"),
            &request,
        ));
    }

    let count = state
        .user_store
        .count()
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    if count > 0 {
        return Err(scoped(
            ApiError::conflict("users already exist; bootstrap disabled"),
            &request,
        ));
    }

    let admin = state
        .auth_service
        .register(
            &payload.admin_username,
            &payload.admin_password,
            payload.admin_display_name,
            None,
            UserRole::Admin,
        )
        .await
        .map_err(|e| scoped(ApiError::bad_request(e.to_string()), &request))?;

    let mut auth_cfg = (**state.system_config.auth.load()).clone();
    auth_cfg.enabled = true;
    auth_cfg.mode = payload
        .auth_mode
        .as_deref()
        .map(|m| match m {
            "api_key" => AuthMode::ApiKey,
            "jwt" => AuthMode::Jwt,
            "none" => AuthMode::None,
            _ => AuthMode::Session,
        })
        .unwrap_or(AuthMode::Session);
    if let Some(allow) = payload.allow_registration {
        auth_cfg.allow_registration = allow;
    }

    let mut generated_secret = false;
    if auth_cfg.jwt_secret.is_none() {
        let secret = tokens::generate_jwt_secret()
            .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
        auth_cfg.jwt_secret = Some(secret);
        generated_secret = true;
    }

    let auth_value = serde_json::to_value(&auth_cfg)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    state
        .system_config
        .store
        .put(CONFIG_KEY_AUTH, &auth_value, Some("bootstrap"))
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    let _ = state.system_config.apply(CONFIG_KEY_AUTH, &auth_value);

    let bootstrap = BootstrapState {
        completed: true,
        admin_created_at: Some(now_iso()),
    };
    let boot_value = serde_json::to_value(&bootstrap)
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    state
        .system_config
        .store
        .put(CONFIG_KEY_BOOTSTRAP, &boot_value, Some("bootstrap"))
        .await
        .map_err(|e| scoped(ApiError::internal(e.to_string()), &request))?;
    let _ = state.system_config.apply(CONFIG_KEY_BOOTSTRAP, &boot_value);

    Ok(Json(BootstrapResponse {
        user: admin,
        bootstrap,
        jwt_secret_generated: generated_secret,
    }))
}
