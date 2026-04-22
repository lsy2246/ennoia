use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ennoia_kernel::{MemoryPolicy, OwnerKind, OwnerRef};
use ennoia_memory::{
    initialize_memory_schema, MemoryKind, MemoryStore, RecallMode, RecallQuery, RememberRequest,
    ReviewAction, ReviewActionKind, SqliteMemoryStore, Stability,
};
use ennoia_paths::RuntimePaths;
use serde::Deserialize;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

#[derive(Clone)]
struct MemoryApiState {
    store: Arc<dyn MemoryStore>,
}

#[derive(Deserialize)]
struct ListMemoriesQuery {
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Deserialize)]
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
    sources: Vec<ennoia_memory::MemorySource>,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    entities: Vec<String>,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
struct ReviewPayload {
    target_memory_id: String,
    reviewer: String,
    action: String,
    #[serde(default)]
    notes: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = parse_options(std::env::args().skip(1).collect());
    let home_dir = options
        .home
        .unwrap_or_else(|| RuntimePaths::resolve(None).home().to_path_buf());
    let runtime_paths = RuntimePaths::new(home_dir);
    runtime_paths.ensure_layout()?;

    let memory_db = runtime_paths.extension_sqlite_db("memory", "memory.db");
    if let Some(parent) = memory_db.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let connect_options = SqliteConnectOptions::new()
        .filename(memory_db)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    initialize_memory_schema(&pool).await?;
    let state = MemoryApiState {
        store: Arc::new(SqliteMemoryStore::new(
            pool,
            Arc::new(MemoryPolicy::builtin()),
        )),
    };

    let router = Router::new()
        .route("/health", get(health))
        .route("/memories", get(list_memories))
        .route("/memories/remember", post(remember_memory))
        .route("/memories/recall", post(recall_memories))
        .route("/memories/review", post(review_memory))
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", options.port)).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "memory-extension" }))
}

async fn list_memories(
    State(state): State<MemoryApiState>,
    Query(query): Query<ListMemoriesQuery>,
) -> ApiResult<Vec<ennoia_memory::MemoryRecord>> {
    state
        .store
        .list_memories(query.limit.unwrap_or(100))
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn remember_memory(
    State(state): State<MemoryApiState>,
    Json(payload): Json<RememberPayload>,
) -> ApiResult<ennoia_memory::RememberReceipt> {
    state
        .store
        .remember(RememberRequest {
            owner: OwnerRef {
                kind: owner_kind_from(&payload.owner_kind),
                id: payload.owner_id,
            },
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
        })
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn recall_memories(
    State(state): State<MemoryApiState>,
    Json(payload): Json<RecallPayload>,
) -> ApiResult<ennoia_memory::RecallResult> {
    state
        .store
        .recall(RecallQuery {
            owner: OwnerRef {
                kind: owner_kind_from(&payload.owner_kind),
                id: payload.owner_id,
            },
            conversation_id: payload.conversation_id,
            run_id: payload.run_id,
            query_text: payload.query_text,
            namespace_prefix: payload.namespace_prefix,
            memory_kind: payload.memory_kind.as_deref().map(MemoryKind::from_str),
            mode: payload
                .mode
                .as_deref()
                .map(recall_mode_from)
                .unwrap_or(RecallMode::Hybrid),
            limit: payload.limit.unwrap_or(20),
        })
        .await
        .map(Json)
        .map_err(internal_error)
}

async fn review_memory(
    State(state): State<MemoryApiState>,
    Json(payload): Json<ReviewPayload>,
) -> ApiResult<ennoia_memory::ReviewReceipt> {
    state
        .store
        .review(ReviewAction {
            target_memory_id: payload.target_memory_id,
            reviewer: payload.reviewer,
            action: review_action_from(&payload.action),
            notes: payload.notes,
        })
        .await
        .map(Json)
        .map_err(internal_error)
}

fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn recall_mode_from(value: &str) -> RecallMode {
    match value {
        "namespace" => RecallMode::Namespace,
        "fts" => RecallMode::Fts,
        _ => RecallMode::Hybrid,
    }
}

fn review_action_from(value: &str) -> ReviewActionKind {
    match value {
        "reject" => ReviewActionKind::Reject,
        "supersede" => ReviewActionKind::Supersede,
        "retire" => ReviewActionKind::Retire,
        _ => ReviewActionKind::Approve,
    }
}

fn internal_error(error: ennoia_memory::MemoryError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

struct Options {
    home: Option<PathBuf>,
    port: u16,
}

fn parse_options(args: Vec<String>) -> Options {
    let mut home = None;
    let mut port = 3911;
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
