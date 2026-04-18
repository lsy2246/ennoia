use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use ennoia_extension_host::{
    ExtensionRegistrySnapshot, RegisteredExtensionSnapshot, RegisteredPageContribution,
    RegisteredPanelContribution,
};
use ennoia_kernel::{
    ArtifactKind, ArtifactSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, ThreadKind,
    ThreadSpec,
};
use ennoia_memory::{MemoryKind, MemoryRecord, MemoryService};
use ennoia_orchestrator::{OrchestratorService, RunRequest, RunTrigger};
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::app::AppState;
use crate::db::{self, JobRecord};

pub fn build_router(state: AppState) -> Router {
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
        .route("/api/v1/tasks", get(tasks))
        .route("/api/v1/artifacts", get(artifacts))
        .route("/api/v1/memories", get(memories))
        .route("/api/v1/jobs", get(jobs).post(create_job))
        .route("/api/v1/runs/private", post(create_private_run))
        .route("/api/v1/runs/space", post(create_space_run))
        .layer(CorsLayer::very_permissive())
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
    counts: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct PrivateRunRequest {
    agent_id: String,
    goal: String,
    message: String,
}

#[derive(Debug, Deserialize)]
struct SpaceRunRequest {
    space_id: String,
    addressed_agents: Vec<String>,
    goal: String,
    message: String,
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
    schedule_kind: String,
    schedule_value: String,
    description: String,
}

#[derive(Debug, Serialize)]
struct ConversationEnvelope {
    thread: ThreadSpec,
    message: MessageSpec,
    run: ennoia_kernel::RunSpec,
    tasks: Vec<ennoia_kernel::TaskSpec>,
    artifacts: Vec<ArtifactSpec>,
    context: ennoia_memory::ContextView,
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
            "jobs": job_count
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

async fn tasks(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::TaskSpec>> {
    Json(db::list_tasks(&state.pool).await.unwrap_or_default())
}

async fn artifacts(State(state): State<AppState>) -> Json<Vec<ArtifactSpec>> {
    Json(db::list_artifacts(&state.pool).await.unwrap_or_default())
}

async fn memories(State(state): State<AppState>) -> Json<Vec<MemoryRecord>> {
    Json(db::list_memories(&state.pool).await.unwrap_or_default())
}

async fn jobs(State(state): State<AppState>) -> Json<Vec<JobRecord>> {
    Json(db::list_jobs(&state.pool).await.unwrap_or_default())
}

async fn create_private_message(
    State(state): State<AppState>,
    Json(payload): Json<PrivateMessageRequest>,
) -> Json<ConversationEnvelope> {
    let goal = payload.goal.unwrap_or_else(|| payload.body.clone());
    Json(
        process_private_message(&state, &payload.agent_id, &payload.body, &goal)
            .await
            .unwrap_or_else(error_envelope),
    )
}

async fn create_space_message(
    State(state): State<AppState>,
    Json(payload): Json<SpaceMessageRequest>,
) -> Json<ConversationEnvelope> {
    let goal = payload.goal.unwrap_or_else(|| payload.body.clone());
    Json(
        process_space_message(
            &state,
            &payload.space_id,
            &payload.addressed_agents,
            &payload.body,
            &goal,
        )
        .await
        .unwrap_or_else(error_envelope),
    )
}

async fn create_private_run(
    State(state): State<AppState>,
    Json(payload): Json<PrivateRunRequest>,
) -> Json<ConversationEnvelope> {
    Json(
        process_private_message(&state, &payload.agent_id, &payload.message, &payload.goal)
            .await
            .unwrap_or_else(error_envelope),
    )
}

async fn create_space_run(
    State(state): State<AppState>,
    Json(payload): Json<SpaceRunRequest>,
) -> Json<ConversationEnvelope> {
    Json(
        process_space_message(
            &state,
            &payload.space_id,
            &payload.addressed_agents,
            &payload.message,
            &payload.goal,
        )
        .await
        .unwrap_or_else(error_envelope),
    )
}

async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Json<JobRecord> {
    let normalized = JobRecord {
        id: new_id("job"),
        owner_kind: payload.owner_kind,
        owner_id: payload.owner_id,
        schedule_kind: normalize_schedule_kind(&payload.schedule_kind),
        schedule_value: payload.schedule_value,
        description: payload.description,
        status: "pending".to_string(),
    };
    let _ = db::insert_job(&state.pool, &normalized).await;
    Json(normalized)
}

async fn process_private_message(
    state: &AppState,
    agent_id: &str,
    body: &str,
    goal: &str,
) -> Result<ConversationEnvelope, String> {
    let timestamp = current_timestamp();
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
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    let message = MessageSpec {
        id: new_id("message"),
        thread_id: thread.id.clone(),
        sender: "user".to_string(),
        role: MessageRole::User,
        body: body.to_string(),
        mentions: vec![agent_id.to_string()],
        created_at: timestamp.clone(),
    };
    let context = build_context(state, &owner, &thread.id).await;
    let plan = OrchestratorService::new().plan_run(
        RunRequest {
            owner: owner.clone(),
            thread: thread.clone(),
            message: message.clone(),
            trigger: RunTrigger::DirectMessage,
            goal: goal.to_string(),
            addressed_agents: vec![agent_id.to_string()],
        },
        context,
    );

    db::insert_planned_run(&state.pool, &plan)
        .await
        .map_err(|error| error.to_string())?;

    let memory = MemoryRecord {
        id: new_id("memory"),
        owner: owner.clone(),
        thread_id: Some(thread.id.clone()),
        run_id: Some(plan.run.id.clone()),
        kind: MemoryKind::Working,
        source: "private_message".to_string(),
        content: body.to_string(),
        summary: format!("Private request for {}: {}", owner.id, goal),
        created_at: timestamp,
    };
    db::insert_memory(&state.pool, &memory)
        .await
        .map_err(|error| error.to_string())?;

    let artifact = persist_run_artifact(state, &owner, &plan.run.id, goal);
    db::insert_artifact(&state.pool, &artifact)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ConversationEnvelope {
        thread: plan.thread,
        message: plan.message,
        run: plan.run,
        tasks: plan.tasks,
        artifacts: vec![artifact],
        context: plan.context,
    })
}

async fn process_space_message(
    state: &AppState,
    space_id: &str,
    addressed_agents: &[String],
    body: &str,
    goal: &str,
) -> Result<ConversationEnvelope, String> {
    let timestamp = current_timestamp();
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
        created_at: timestamp.clone(),
        updated_at: timestamp.clone(),
    };
    let message = MessageSpec {
        id: new_id("message"),
        thread_id: thread.id.clone(),
        sender: "user".to_string(),
        role: MessageRole::User,
        body: body.to_string(),
        mentions: resolved_agents.clone(),
        created_at: timestamp.clone(),
    };
    let context = build_context(state, &owner, &thread.id).await;
    let plan = OrchestratorService::new().plan_run(
        RunRequest {
            owner: owner.clone(),
            thread: thread.clone(),
            message: message.clone(),
            trigger: RunTrigger::SpaceMessage,
            goal: goal.to_string(),
            addressed_agents: resolved_agents,
        },
        context,
    );

    db::insert_planned_run(&state.pool, &plan)
        .await
        .map_err(|error| error.to_string())?;

    let memory = MemoryRecord {
        id: new_id("memory"),
        owner: owner.clone(),
        thread_id: Some(thread.id.clone()),
        run_id: Some(plan.run.id.clone()),
        kind: MemoryKind::Working,
        source: "space_message".to_string(),
        content: body.to_string(),
        summary: format!("Space request for {}: {}", owner.id, goal),
        created_at: timestamp,
    };
    db::insert_memory(&state.pool, &memory)
        .await
        .map_err(|error| error.to_string())?;

    let artifact = persist_run_artifact(state, &owner, &plan.run.id, goal);
    db::insert_artifact(&state.pool, &artifact)
        .await
        .map_err(|error| error.to_string())?;

    Ok(ConversationEnvelope {
        thread: plan.thread,
        message: plan.message,
        run: plan.run,
        tasks: plan.tasks,
        artifacts: vec![artifact],
        context: plan.context,
    })
}

async fn build_context(
    state: &AppState,
    owner: &OwnerRef,
    thread_id: &str,
) -> ennoia_memory::ContextView {
    let mut memory_service = MemoryService::new();
    for record in db::load_memories_for_owner(&state.pool, owner)
        .await
        .unwrap_or_default()
    {
        memory_service.remember(record);
    }
    for record in db::load_memories_for_thread(&state.pool, thread_id)
        .await
        .unwrap_or_default()
    {
        memory_service.remember(record);
    }

    let recent_messages = db::list_messages_for_thread(&state.pool, thread_id)
        .await
        .unwrap_or_default()
        .into_iter()
        .rev()
        .take(5)
        .map(|message| format!("{}: {}", message.sender, message.body))
        .collect();

    let active_tasks = db::list_active_tasks_for_owner(&state.pool, owner, 5)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|task| task.title)
        .collect();

    memory_service.build_context(owner, Some(thread_id), recent_messages, active_tasks)
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
        id: new_id("artifact"),
        owner: owner.clone(),
        run_id: run_id.to_string(),
        kind: ArtifactKind::Report,
        relative_path,
        created_at: current_timestamp(),
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

fn error_envelope(message: String) -> ConversationEnvelope {
    let timestamp = current_timestamp();
    ConversationEnvelope {
        thread: ThreadSpec {
            id: "thread-error".to_string(),
            kind: ThreadKind::Private,
            owner: OwnerRef {
                kind: OwnerKind::Global,
                id: "system".to_string(),
            },
            space_id: None,
            title: "error".to_string(),
            participants: vec!["system".to_string()],
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        },
        message: MessageSpec {
            id: "message-error".to_string(),
            thread_id: "thread-error".to_string(),
            sender: "system".to_string(),
            role: MessageRole::System,
            body: message.clone(),
            mentions: Vec::new(),
            created_at: timestamp.clone(),
        },
        run: ennoia_kernel::RunSpec {
            id: "run-error".to_string(),
            owner: OwnerRef {
                kind: OwnerKind::Global,
                id: "system".to_string(),
            },
            thread_id: "thread-error".to_string(),
            trigger: "system".to_string(),
            status: ennoia_kernel::RunStatus::Blocked,
            goal: message,
            created_at: timestamp.clone(),
            updated_at: timestamp,
        },
        tasks: Vec::new(),
        artifacts: Vec::new(),
        context: ennoia_memory::ContextView::default(),
    }
}

fn normalize_schedule_kind(value: &str) -> String {
    match value {
        "maintenance" => "maintenance".to_string(),
        "cron" => "cron".to_string(),
        _ => "delay".to_string(),
    }
}

fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", current_timestamp())
}

fn current_timestamp() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_millis();
    format!("{millis}")
}
