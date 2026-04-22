use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::State,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ennoia_kernel::{ConversationMessageHookPayload, HookDispatchResponse, HookEventEnvelope};
use ennoia_paths::RuntimePaths;
use ennoia_policy::PolicySet;
use ennoia_workflow::{
    orchestrator::OrchestratorService,
    pipeline::{run_conversation_workflow, WorkflowRuntime},
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
        .route(
            "/hooks/conversation-message-created",
            post(conversation_message_created),
        )
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

async fn conversation_message_created(
    State(state): State<WorkflowApiState>,
    Json(event): Json<HookEventEnvelope>,
) -> ApiResult<HookDispatchResponse> {
    let payload: ConversationMessageHookPayload = serde_json::from_value(event.payload)
        .map_err(|error| (StatusCode::BAD_REQUEST, error.to_string()))?;
    let output = run_conversation_workflow(&state.runtime, payload)
        .await
        .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error))?;

    Ok(Json(HookDispatchResponse {
        handled: true,
        result: Some(
            serde_json::to_value(output)
                .map_err(|error| (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()))?,
        ),
        message: None,
    }))
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
