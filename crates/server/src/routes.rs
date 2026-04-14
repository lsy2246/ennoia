use std::fs;

use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use ennoia_kernel::{OwnerKind, OwnerRef};
use ennoia_memory::{MemoryKind, MemoryRecord, MemoryService};
use ennoia_orchestrator::OrchestratorService;
use serde::{Deserialize, Serialize};
use tower_http::cors::CorsLayer;

use crate::app::AppState;
use crate::db::{self, JobRecord};

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/v1/overview", get(overview))
        .route("/api/v1/extensions", get(extensions))
        .route("/api/v1/agents", get(agents))
        .route("/api/v1/spaces", get(spaces))
        .route("/api/v1/runs", get(runs))
        .route("/api/v1/tasks", get(tasks))
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
struct CreateJobRequest {
    owner_kind: String,
    owner_id: String,
    schedule_kind: String,
    schedule_value: String,
    description: String,
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "ok",
        app: "Ennoia",
    })
}

async fn overview(State(state): State<AppState>) -> Json<OverviewResponse> {
    let run_count = db::count_rows(&state.pool, "runs").await.unwrap_or(0);
    let task_count = db::count_rows(&state.pool, "tasks").await.unwrap_or(0);
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
            "runs": run_count,
            "tasks": task_count,
            "memories": memory_count,
            "jobs": job_count
        }),
    })
}

async fn extensions(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    Json(
        state
            .extensions
            .items()
            .iter()
            .map(|item| {
                serde_json::json!({
                    "id": item.manifest.id,
                    "version": item.manifest.version,
                    "install_dir": item.install_dir,
                    "pages": item.manifest.contributes.pages,
                    "panels": item.manifest.contributes.panels,
                    "commands": item.manifest.contributes.commands
                })
            })
            .collect(),
    )
}

async fn agents(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::AgentConfig>> {
    Json(state.agents)
}

async fn spaces(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::SpaceSpec>> {
    Json(state.spaces)
}

async fn runs(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    Json(db::list_runs(&state.pool).await.unwrap_or_default())
}

async fn tasks(State(state): State<AppState>) -> Json<Vec<serde_json::Value>> {
    Json(db::list_tasks(&state.pool).await.unwrap_or_default())
}

async fn memories(State(state): State<AppState>) -> Json<Vec<MemoryRecord>> {
    Json(db::list_memories(&state.pool).await.unwrap_or_default())
}

async fn jobs(State(state): State<AppState>) -> Json<Vec<JobRecord>> {
    Json(db::list_jobs(&state.pool).await.unwrap_or_default())
}

async fn create_private_run(
    State(state): State<AppState>,
    Json(payload): Json<PrivateRunRequest>,
) -> Json<serde_json::Value> {
    let owner = OwnerRef {
        kind: OwnerKind::Agent,
        id: payload.agent_id.clone(),
    };
    let context = build_context(&state, &owner).await;
    let orchestrator = OrchestratorService::new();
    let plan = orchestrator.plan_private_run(&payload.agent_id, &payload.goal, context);

    let _ = db::insert_planned_run(&state.pool, &plan, &payload.goal).await;
    let _ = db::insert_memory(
        &state.pool,
        &MemoryRecord {
            id: format!("memory-{}", plan.run.id),
            owner: owner.clone(),
            kind: MemoryKind::Working,
            source: "private_run".to_string(),
            content: payload.message,
            summary: format!("Private request for {}: {}", owner.id, payload.goal),
        },
    )
    .await;
    persist_run_artifact(&state, &owner, &plan.run.id, &payload.goal);

    Json(serde_json::json!({
        "run": plan.run,
        "tasks": plan.tasks,
        "context": plan.context
    }))
}

async fn create_space_run(
    State(state): State<AppState>,
    Json(payload): Json<SpaceRunRequest>,
) -> Json<serde_json::Value> {
    let owner = OwnerRef {
        kind: OwnerKind::Space,
        id: payload.space_id.clone(),
    };
    let context = build_context(&state, &owner).await;
    let orchestrator = OrchestratorService::new();
    let plan = orchestrator.plan_space_run(
        &payload.space_id,
        &payload.addressed_agents,
        &payload.goal,
        context,
    );

    let _ = db::insert_planned_run(&state.pool, &plan, &payload.goal).await;
    let _ = db::insert_memory(
        &state.pool,
        &MemoryRecord {
            id: format!("memory-{}", plan.run.id),
            owner: owner.clone(),
            kind: MemoryKind::Working,
            source: "space_run".to_string(),
            content: payload.message,
            summary: format!("Space request for {}: {}", owner.id, payload.goal),
        },
    )
    .await;
    persist_run_artifact(&state, &owner, &plan.run.id, &payload.goal);

    Json(serde_json::json!({
        "run": plan.run,
        "tasks": plan.tasks,
        "context": plan.context
    }))
}

async fn create_job(
    State(state): State<AppState>,
    Json(payload): Json<CreateJobRequest>,
) -> Json<JobRecord> {
    let normalized = JobRecord {
        id: format!(
            "job-{}",
            db::count_rows(&state.pool, "jobs").await.unwrap_or(0) + 1
        ),
        owner_kind: payload.owner_kind,
        owner_id: payload.owner_id,
        schedule_kind: normalize_schedule_kind(&payload.schedule_kind),
        schedule_value: payload.schedule_value,
        description: payload.description,
        status: "Pending".to_string(),
    };
    let _ = db::insert_job(&state.pool, &normalized).await;
    Json(normalized)
}

async fn build_context(state: &AppState, owner: &OwnerRef) -> ennoia_memory::ContextView {
    let mut memory_service = MemoryService::new();
    for record in db::load_memories_for_owner(&state.pool, owner)
        .await
        .unwrap_or_default()
    {
        memory_service.remember(record);
    }

    let active_tasks = db::list_tasks(&state.pool)
        .await
        .unwrap_or_default()
        .into_iter()
        .take(5)
        .map(|task| task["title"].as_str().unwrap_or_default().to_string())
        .collect();

    memory_service.build_context(owner, Vec::new(), active_tasks)
}

fn persist_run_artifact(state: &AppState, owner: &OwnerRef, run_id: &str, goal: &str) {
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
    let _ = fs::write(
        owner_root.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": run_id,
            "owner": owner,
            "goal": goal
        }))
        .unwrap_or_default(),
    );
}

fn normalize_schedule_kind(value: &str) -> String {
    match value {
        "maintenance" => "maintenance".to_string(),
        "cron" => "cron".to_string(),
        _ => "delay".to_string(),
    }
}
