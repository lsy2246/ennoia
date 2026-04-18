use std::fs;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware as axum_middleware,
    routing::{get, post},
    Json, Router,
};
use chrono::Utc;
use ennoia_extension_host::{
    ExtensionRegistrySnapshot, RegisteredExtensionSnapshot, RegisteredPageContribution,
    RegisteredPanelContribution,
};
use ennoia_kernel::{
    ArtifactKind, ArtifactSpec, AssembleRequest, ConfigChangeRecord, ConfigEntry, ConfigStore,
    ContextView, EnqueueRequest, EpisodeKind, EpisodeRequest, JobKind, JobRecord, MemoryKind,
    MemoryRecord, MemorySource, MessageRole, MessageSpec, OwnerKind, OwnerRef, RecallMode,
    RecallQuery, RecallResult, RememberReceipt, RememberRequest, ReviewAction, ReviewActionKind,
    ReviewReceipt, ScheduleKind, Stability, SystemConfig, ThreadKind, ThreadSpec,
    ALL_CONFIG_KEYS,
};
use ennoia_orchestrator::{RunRequest, RunTrigger};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::app::AppState;
use crate::db::{self, JobRow};
use crate::middleware::{
    auth_middleware, body_limit_middleware, cors_middleware, logging_middleware,
    rate_limit_middleware, timeout_middleware,
};

pub fn build_router(state: AppState) -> Router {
    let admin = Router::new()
        .route("/api/v1/admin/config", get(config_list))
        .route(
            "/api/v1/admin/config/{key}",
            get(config_get).put(config_put),
        )
        .route("/api/v1/admin/config/{key}/history", get(config_history))
        .route("/api/v1/admin/config/snapshot", get(config_snapshot));

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
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
        .route(
            "/api/v1/memories",
            get(memories_list).post(memories_create),
        )
        .route("/api/v1/memories/recall", post(memories_recall))
        .route("/api/v1/memories/review", post(memories_review))
        .route("/api/v1/jobs", get(jobs_list).post(jobs_create))
        .merge(admin)
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
    shell_title: String,
    default_theme: String,
    modules: Vec<String>,
    counts: JsonValue,
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
    let thread_count = db::count_rows(&state.pool, "threads").await.unwrap_or(0);
    let message_count = db::count_rows(&state.pool, "messages").await.unwrap_or(0);
    let run_count = db::count_rows(&state.pool, "runs").await.unwrap_or(0);
    let task_count = db::count_rows(&state.pool, "tasks").await.unwrap_or(0);
    let artifact_count = db::count_rows(&state.pool, "artifacts").await.unwrap_or(0);
    let memory_count = db::count_rows(&state.pool, "memories").await.unwrap_or(0);
    let job_count = db::count_rows(&state.pool, "jobs").await.unwrap_or(0);
    let decision_count = db::count_rows(&state.pool, "decisions").await.unwrap_or(0);

    Json(OverviewResponse {
        app_name: state.overview.app_name,
        shell_title: state.ui_config.shell_title,
        default_theme: state.ui_config.default_theme,
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
    Json(state.memory_store.list_memories(100).await.unwrap_or_default())
}

async fn memories_create(
    State(state): State<AppState>,
    Json(payload): Json<RememberPayload>,
) -> Result<Json<RememberReceipt>, (StatusCode, String)> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let request = RememberRequest {
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
        .remember(request)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
}

async fn memories_recall(
    State(state): State<AppState>,
    Json(payload): Json<RecallPayload>,
) -> Result<Json<RecallResult>, (StatusCode, String)> {
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
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
}

async fn memories_review(
    State(state): State<AppState>,
    Json(payload): Json<ReviewPayload>,
) -> Result<Json<ReviewReceipt>, (StatusCode, String)> {
    let action_kind = match payload.action.as_str() {
        "approve" => ReviewActionKind::Approve,
        "reject" => ReviewActionKind::Reject,
        "supersede" => ReviewActionKind::Supersede,
        "retire" => ReviewActionKind::Retire,
        _ => return Err((StatusCode::BAD_REQUEST, "unknown action".to_string())),
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
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
}

async fn jobs_list(State(state): State<AppState>) -> Json<Vec<JobRow>> {
    Json(db::list_jobs(&state.pool).await.unwrap_or_default())
}

async fn jobs_create(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Result<Json<JobRecord>, (StatusCode, String)> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let request = EnqueueRequest {
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
        .enqueue(request)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))
}

async fn create_private_message(
    State(state): State<AppState>,
    Json(payload): Json<PrivateMessageRequest>,
) -> Result<Json<ConversationEnvelope>, (StatusCode, String)> {
    let goal = payload.goal.unwrap_or_else(|| payload.body.clone());
    process_private_message(&state, &payload.agent_id, &payload.body, &goal)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::BAD_REQUEST, error))
}

async fn create_space_message(
    State(state): State<AppState>,
    Json(payload): Json<SpaceMessageRequest>,
) -> Result<Json<ConversationEnvelope>, (StatusCode, String)> {
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
    .map_err(|error| (StatusCode::BAD_REQUEST, error))
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
    let owner_root = match owner.kind {
        OwnerKind::Agent => state
            .home_dir
            .join(format!("agents/{}/artifacts/runs/{run_id}", owner.id)),
        OwnerKind::Space => state
            .home_dir
            .join(format!("spaces/{}/artifacts/runs/{run_id}", owner.id)),
        OwnerKind::Global => state.home_dir.join(format!("global/extensions/{run_id}")),
    };

    let _ = fs::create_dir_all(&owner_root);
    let relative_path = match owner.kind {
        OwnerKind::Agent => format!("agents/{}/artifacts/runs/{run_id}/summary.json", owner.id),
        OwnerKind::Space => format!("spaces/{}/artifacts/runs/{run_id}/summary.json", owner.id),
        OwnerKind::Global => format!("global/extensions/{run_id}/summary.json"),
    };

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
    Path(key): Path<String>,
) -> Result<Json<ConfigEntry>, (StatusCode, String)> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err((StatusCode::NOT_FOUND, format!("unknown config key '{key}'")));
    }
    state
        .system_config
        .store
        .get(&key)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?
        .map(Json)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("config '{key}' not initialized")))
}

async fn config_put(
    State(state): State<AppState>,
    Path(key): Path<String>,
    Json(payload): Json<ConfigPutPayload>,
) -> Result<Json<ConfigEntry>, (StatusCode, String)> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err((StatusCode::NOT_FOUND, format!("unknown config key '{key}'")));
    }
    let entry = state
        .system_config
        .store
        .put(&key, &payload.payload, payload.updated_by.as_deref())
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let applied = state
        .system_config
        .apply(&key, &payload.payload)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;
    if !applied {
        return Err((StatusCode::BAD_REQUEST, format!("unsupported key '{key}'")));
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
    let have: std::collections::HashSet<String> =
        rows.iter().map(|r| r.key.clone()).collect();
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
