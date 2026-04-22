use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ennoia_contract::behavior::{
    BehaviorRunDetailResponse, BehaviorRunRequest, BehaviorRunResponse, BehaviorStatusResponse,
};
use ennoia_paths::RuntimePaths;
use ennoia_policy::PolicySet;
use ennoia_workflow::{
    orchestrator::OrchestratorService,
    pipeline::{run_behavior, WorkflowRuntime},
    runtime::{
        builtin_pipeline, initialize_workflow_schema, PolicyStageMachine, RuntimeStore,
        SqliteRuntimeStore,
    },
};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

#[derive(Clone)]
struct WorkflowApiState {
    runtime: WorkflowRuntime,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = parse_options(std::env::args().skip(1).collect());
    let home_dir = options
        .home
        .unwrap_or_else(|| RuntimePaths::resolve(None).home().to_path_buf());
    let runtime_paths = Arc::new(RuntimePaths::new(home_dir));
    runtime_paths.ensure_layout()?;

    let connect_options = SqliteConnectOptions::new()
        .filename(runtime_paths.extension_sqlite_db("workflow", "workflow.db"))
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    initialize_workflow_schema(&pool).await?;

    let policies =
        PolicySet::load(runtime_paths.policies_dir()).unwrap_or_else(|_| PolicySet::builtin());
    let stage_machine = Arc::new(PolicyStageMachine::new(Arc::new(policies.stage)));
    let runtime_store: Arc<dyn RuntimeStore> = Arc::new(SqliteRuntimeStore::new(pool.clone()));
    let orchestrator = OrchestratorService::new(stage_machine, builtin_pipeline());
    let state = WorkflowApiState {
        runtime: WorkflowRuntime {
            runtime_paths,
            pool,
            runtime_store,
            orchestrator,
            agents_fallback: Vec::new(),
        },
    };

    let router = Router::new()
        .route("/health", get(health))
        .route("/status", get(status))
        .route("/runs", post(runs_create))
        .route("/runs/{run_id}", get(run_detail))
        .route("/runs/{run_id}/tasks", get(run_tasks))
        .route("/runs/{run_id}/artifacts", get(run_artifacts))
        .route("/runs/{run_id}/handoffs", get(run_handoffs))
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", options.port)).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "workflow-extension"
    }))
}

async fn status(State(_state): State<WorkflowApiState>) -> Json<BehaviorStatusResponse> {
    Json(BehaviorStatusResponse {
        extension_id: "workflow".to_string(),
        behavior_id: "default".to_string(),
        healthy: true,
        version: env!("CARGO_PKG_VERSION").to_string(),
        interfaces: vec![
            "runs".to_string(),
            "tasks".to_string(),
            "artifacts".to_string(),
            "handoffs".to_string(),
            "status".to_string(),
        ],
    })
}

async fn runs_create(
    State(state): State<WorkflowApiState>,
    Json(payload): Json<BehaviorRunRequest>,
) -> ApiResult<BehaviorRunResponse> {
    let output = run_behavior(&state.runtime, payload)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;
    Ok(Json(output))
}

async fn run_detail(
    State(state): State<WorkflowApiState>,
    Path(run_id): Path<String>,
) -> ApiResult<BehaviorRunDetailResponse> {
    let run = state
        .runtime
        .runtime_store
        .get_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "run not found".to_string()))?;
    let tasks = state
        .runtime
        .runtime_store
        .list_tasks_for_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let artifacts = state
        .runtime
        .runtime_store
        .list_artifacts_for_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let handoffs = state
        .runtime
        .runtime_store
        .list_handoffs_for_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let stage_events = state
        .runtime
        .runtime_store
        .list_stage_events_for_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?;
    let gate_verdicts = state
        .runtime
        .runtime_store
        .list_gate_verdicts_for_run(&run_id)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?
        .into_iter()
        .map(|record| ennoia_kernel::GateVerdict {
            gate_name: record.gate_name,
            allow: record.verdict != "deny",
            severity: match record.verdict.as_str() {
                "warn" => ennoia_kernel::GateSeverity::Warn,
                "deny" => ennoia_kernel::GateSeverity::Deny,
                _ => ennoia_kernel::GateSeverity::Info,
            },
            reason: record.reason.unwrap_or_default(),
        })
        .collect();

    Ok(Json(BehaviorRunDetailResponse {
        run,
        tasks,
        artifacts,
        handoffs,
        stage_events,
        gate_verdicts,
    }))
}

async fn run_tasks(
    State(state): State<WorkflowApiState>,
    Path(run_id): Path<String>,
) -> ApiResult<Vec<ennoia_kernel::TaskSpec>> {
    state
        .runtime
        .runtime_store
        .list_tasks_for_run(&run_id)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

async fn run_artifacts(
    State(state): State<WorkflowApiState>,
    Path(run_id): Path<String>,
) -> ApiResult<Vec<ennoia_kernel::ArtifactSpec>> {
    state
        .runtime
        .runtime_store
        .list_artifacts_for_run(&run_id)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

async fn run_handoffs(
    State(state): State<WorkflowApiState>,
    Path(run_id): Path<String>,
) -> ApiResult<Vec<ennoia_kernel::HandoffSpec>> {
    state
        .runtime
        .runtime_store
        .list_handoffs_for_run(&run_id)
        .await
        .map(Json)
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))
}

struct Options {
    home: Option<PathBuf>,
    port: u16,
}

fn parse_options(args: Vec<String>) -> Options {
    let mut home = None;
    let mut port = 3921;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--home" if index + 1 < args.len() => {
                home = Some(PathBuf::from(&args[index + 1]));
                index += 1;
            }
            "--port" if index + 1 < args.len() => {
                if let Ok(value) = args[index + 1].parse::<u16>() {
                    port = value;
                }
                index += 1;
            }
            _ => {}
        }
        index += 1;
    }
    Options { home, port }
}
