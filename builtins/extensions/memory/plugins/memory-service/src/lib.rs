//! Memory module owns its domain model, store contract and sqlite implementation.

pub mod model;
pub mod schema;
pub mod sqlite;

use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

use crate::model as memory_model;
use ennoia_kernel::{
    ContextFrame, ContextLayer, ExtensionRpcResponse, HookDispatchResponse, HookEventEnvelope,
    OwnerKind, OwnerRef,
};
use ennoia_paths::RuntimePaths;
use schema::initialize_memory_schema;
use serde::Deserialize;
use serde::Serialize;
use serde_json::Value as JsonValue;
use sqlite::SqliteMemoryStore;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

pub use model::*;
pub use sqlite::SqliteMemoryStore as MemoryStoreImpl;

#[derive(Debug, Deserialize)]
struct Invocation {
    method: String,
    #[serde(default)]
    params: JsonValue,
    #[serde(default)]
    context: JsonValue,
}

#[derive(Debug, Deserialize)]
struct ListMemoriesPayload {
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct EpisodesListPayload {
    #[serde(default)]
    owner_kind: Option<String>,
    #[serde(default)]
    owner_id: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
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
    sources: Vec<memory_model::MemorySource>,
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

#[derive(Debug, Deserialize)]
struct AssemblePayload {
    owner_kind: String,
    owner_id: String,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    recent_messages: Vec<String>,
    #[serde(default)]
    active_tasks: Vec<String>,
    #[serde(default)]
    budget_chars: Option<u32>,
}

#[derive(Debug, Serialize)]
struct MemoryWorkspaceSummary {
    pending_review_count: usize,
    active_memory_count: usize,
    episode_count: i64,
    graph_nodes_count: i64,
    session_state_count: i64,
}

struct MemoryServiceState {
    store: SqliteMemoryStore,
}

pub fn module_name() -> &'static str {
    "memory"
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime_paths = RuntimePaths::resolve(None);
    runtime_paths.ensure_layout()?;

    let database_path = runtime_paths.extension_sqlite_db("memory", "memory.db");
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(database_path)
                .create_if_missing(true),
        )
        .await?;
    initialize_memory_schema(&pool).await?;

    let state = MemoryServiceState {
        store: SqliteMemoryStore::new(
            pool,
            std::sync::Arc::new(ennoia_kernel::MemoryPolicy::builtin()),
        ),
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }

        let response = match serde_json::from_str::<Invocation>(line.trim_end()) {
            Ok(invocation) => handle_invocation(&state, invocation).await,
            Err(error) => ExtensionRpcResponse::failure("invalid_request", error.to_string()),
        };

        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }

    Ok(())
}

async fn handle_invocation(
    state: &MemoryServiceState,
    invocation: Invocation,
) -> ExtensionRpcResponse {
    let path = invocation.method.trim_matches('/');
    let _context = invocation.context;
    match path {
        "memory/workspace" => match workspace_summary(state).await {
            Ok(summary) => ExtensionRpcResponse::success(serde_json::json!(summary)),
            Err(error) => error,
        },
        "memory/memories/list" => match parse_json::<ListMemoriesPayload>(invocation.params) {
            Ok(payload) => match state
                .store
                .list_memories(payload.limit.unwrap_or(100))
                .await
            {
                Ok(memories) => ExtensionRpcResponse::success(serde_json::json!(memories)),
                Err(error) => {
                    ExtensionRpcResponse::failure("memory_list_failed", error.to_string())
                }
            },
            Err(error) => error,
        },
        "memory/episodes/list" => match parse_json::<EpisodesListPayload>(invocation.params) {
            Ok(payload) => match list_episodes(state, payload).await {
                Ok(episodes) => ExtensionRpcResponse::success(serde_json::json!(episodes)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "memory/memories/remember" => match parse_json::<RememberPayload>(invocation.params) {
            Ok(payload) => match remember_memory(state, payload).await {
                Ok(receipt) => ExtensionRpcResponse::success(serde_json::json!(receipt)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "memory/memories/recall" => match parse_json::<RecallPayload>(invocation.params) {
            Ok(payload) => match recall_memories(state, payload).await {
                Ok(result) => ExtensionRpcResponse::success(serde_json::json!(result)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "memory/memories/review" => match parse_json::<ReviewPayload>(invocation.params) {
            Ok(payload) => match review_memory(state, payload).await {
                Ok(result) => ExtensionRpcResponse::success(serde_json::json!(result)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "memory/context/assemble" => match parse_json::<AssemblePayload>(invocation.params) {
            Ok(payload) => match assemble_context(state, payload).await {
                Ok(result) => ExtensionRpcResponse::success(serde_json::json!(result)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "hooks/conversation/created"
        | "hooks/conversation/message/created"
        | "hooks/conversation/deleted" => {
            match parse_json::<HookEventEnvelope>(invocation.params) {
                Ok(event) => match ingest_hook_event(state, path, event).await {
                    Ok(result) => ExtensionRpcResponse::success(serde_json::json!(result)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        _ => ExtensionRpcResponse::failure(
            "method_not_found",
            format!("memory worker method '{path}' not found"),
        ),
    }
}

async fn workspace_summary(
    state: &MemoryServiceState,
) -> Result<MemoryWorkspaceSummary, ExtensionRpcResponse> {
    let pending_review_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memories WHERE review_state IN ('pending_review', 'pending') OR status = 'pending_review'",
    )
    .fetch_one(state.store.pool())
    .await
    .map_err(sql_error("memory_workspace_failed"))?;
    let active_memory_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memories WHERE status = 'active'")
            .fetch_one(state.store.pool())
            .await
            .map_err(sql_error("memory_workspace_failed"))?;
    let episode_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM episodes")
        .fetch_one(state.store.pool())
        .await
        .map_err(sql_error("memory_workspace_failed"))?;
    let graph_nodes_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gm_nodes")
        .fetch_one(state.store.pool())
        .await
        .map_err(sql_error("memory_workspace_failed"))?;
    let session_state_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM session_states")
        .fetch_one(state.store.pool())
        .await
        .map_err(sql_error("memory_workspace_failed"))?;

    Ok(MemoryWorkspaceSummary {
        pending_review_count: pending_review_count.max(0) as usize,
        active_memory_count: active_memory_count.max(0) as usize,
        episode_count,
        graph_nodes_count,
        session_state_count,
    })
}

async fn list_episodes(
    state: &MemoryServiceState,
    payload: EpisodesListPayload,
) -> Result<Vec<model::EpisodeRecord>, ExtensionRpcResponse> {
    let owner = payload
        .owner_kind
        .zip(payload.owner_id)
        .map(|(kind, id)| OwnerRef::new(owner_kind_from(&kind), id))
        .unwrap_or_else(|| OwnerRef::global("global"));
    state
        .store
        .list_episodes_for_owner(&owner, payload.limit.unwrap_or(50))
        .await
        .map_err(memory_error("memory_episode_list_failed"))
}

async fn remember_memory(
    state: &MemoryServiceState,
    payload: RememberPayload,
) -> Result<model::RememberReceipt, ExtensionRpcResponse> {
    state
        .store
        .remember(memory_model::RememberRequest {
            owner: OwnerRef::new(owner_kind_from(&payload.owner_kind), payload.owner_id),
            namespace: payload.namespace,
            memory_kind: memory_model::MemoryKind::from_str(&payload.memory_kind),
            stability: memory_model::Stability::from_str(&payload.stability),
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
        .map_err(memory_error("memory_remember_failed"))
}

async fn recall_memories(
    state: &MemoryServiceState,
    payload: RecallPayload,
) -> Result<model::RecallResult, ExtensionRpcResponse> {
    state
        .store
        .recall(memory_model::RecallQuery {
            owner: OwnerRef::new(owner_kind_from(&payload.owner_kind), payload.owner_id),
            conversation_id: payload.conversation_id,
            run_id: payload.run_id,
            query_text: payload.query_text,
            namespace_prefix: payload.namespace_prefix,
            memory_kind: payload
                .memory_kind
                .as_deref()
                .map(memory_model::MemoryKind::from_str),
            mode: payload
                .mode
                .as_deref()
                .map(recall_mode_from)
                .unwrap_or(memory_model::RecallMode::Hybrid),
            limit: payload.limit.unwrap_or(20),
        })
        .await
        .map_err(memory_error("memory_recall_failed"))
}

async fn review_memory(
    state: &MemoryServiceState,
    payload: ReviewPayload,
) -> Result<model::ReviewReceipt, ExtensionRpcResponse> {
    state
        .store
        .review(memory_model::ReviewAction {
            target_memory_id: payload.target_memory_id,
            reviewer: payload.reviewer,
            action: review_action_from(&payload.action),
            notes: payload.notes,
        })
        .await
        .map_err(memory_error("memory_review_failed"))
}

async fn assemble_context(
    state: &MemoryServiceState,
    payload: AssemblePayload,
) -> Result<ennoia_kernel::RunContext, ExtensionRpcResponse> {
    state
        .store
        .assemble_context(memory_model::AssembleRequest {
            owner: OwnerRef::new(owner_kind_from(&payload.owner_kind), payload.owner_id),
            conversation_id: payload.conversation_id,
            run_id: payload.run_id,
            recent_messages: payload.recent_messages,
            active_tasks: payload.active_tasks,
            budget_chars: payload.budget_chars,
        })
        .await
        .map_err(memory_error("memory_context_assemble_failed"))
}

async fn ingest_hook_event(
    state: &MemoryServiceState,
    path: &str,
    event: HookEventEnvelope,
) -> Result<HookDispatchResponse, ExtensionRpcResponse> {
    match path {
        "hooks/conversation/created" => {
            let owner = hook_owner(&event);
            let title = event
                .payload
                .get("conversation")
                .and_then(|item| item.get("title"))
                .and_then(JsonValue::as_str)
                .unwrap_or("新会话")
                .to_string();
            state
                .store
                .record_episode(memory_model::EpisodeRequest {
                    owner: owner.clone(),
                    namespace: event_namespace(&event),
                    conversation_id: event
                        .resource
                        .conversation_id
                        .clone()
                        .or_else(|| Some(event.resource.id.clone())),
                    run_id: None,
                    episode_kind: memory_model::EpisodeKind::Decision,
                    role: Some("system".to_string()),
                    content: format!("会话已创建：{title}"),
                    content_type: Some("text/plain".to_string()),
                    source_uri: None,
                    entities: collect_entities(&event, &title),
                    tags: vec!["conversation".to_string(), "created".to_string()],
                    importance: Some(0.3),
                    occurred_at: Some(event.occurred_at.clone()),
                })
                .await
                .map_err(memory_error("memory_hook_ingest_failed"))?;
            Ok(HookDispatchResponse {
                handled: true,
                result: Some(serde_json::json!({ "ingested": true })),
                message: Some("conversation created ingested".to_string()),
            })
        }
        "hooks/conversation/message/created" => {
            let owner = hook_owner(&event);
            let body = event
                .payload
                .get("message")
                .and_then(|item| item.get("body"))
                .and_then(JsonValue::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if body.is_empty() {
                return Ok(HookDispatchResponse {
                    handled: false,
                    result: None,
                    message: Some("message body empty; skipped".to_string()),
                });
            }
            let sender = event
                .payload
                .get("message")
                .and_then(|item| item.get("sender"))
                .and_then(JsonValue::as_str)
                .unwrap_or("operator")
                .to_string();
            let role = event
                .payload
                .get("message")
                .and_then(|item| item.get("role"))
                .and_then(JsonValue::as_str)
                .unwrap_or("operator")
                .to_string();
            let conversation_id = event.resource.conversation_id.clone().or_else(|| {
                event
                    .payload
                    .get("conversation")
                    .and_then(|item| item.get("id"))
                    .and_then(JsonValue::as_str)
                    .map(str::to_string)
            });

            state
                .store
                .record_episode(memory_model::EpisodeRequest {
                    owner: owner.clone(),
                    namespace: event_namespace(&event),
                    conversation_id: conversation_id.clone(),
                    run_id: None,
                    episode_kind: memory_model::EpisodeKind::Message,
                    role: Some(role.clone()),
                    content: body.clone(),
                    content_type: Some("text/plain".to_string()),
                    source_uri: None,
                    entities: collect_entities(&event, &body),
                    tags: vec![
                        "conversation".to_string(),
                        "message".to_string(),
                        sender.clone(),
                    ],
                    importance: Some(0.4),
                    occurred_at: Some(event.occurred_at.clone()),
                })
                .await
                .map_err(memory_error("memory_hook_ingest_failed"))?;

            if let Some(conversation_id) = conversation_id {
                state
                    .store
                    .upsert_frame(ContextFrame {
                        id: format!("frame-recent-{conversation_id}"),
                        owner,
                        namespace: format!("conversation/{conversation_id}/recent"),
                        layer: ContextLayer::Core,
                        frame_kind: "recent_message".to_string(),
                        content: format!("{sender}: {body}"),
                        source_memory_ids: Vec::new(),
                        budget_chars: Some(1024),
                        ttl_seconds: Some(21_600),
                        created_at: String::new(),
                        updated_at: String::new(),
                    })
                    .await
                    .map_err(memory_error("memory_hook_frame_failed"))?;
            }

            Ok(HookDispatchResponse {
                handled: true,
                result: Some(serde_json::json!({ "ingested": true })),
                message: Some("conversation message ingested".to_string()),
            })
        }
        "hooks/conversation/deleted" => Ok(HookDispatchResponse {
            handled: true,
            result: Some(serde_json::json!({ "deleted": true })),
            message: Some("conversation delete acknowledged".to_string()),
        }),
        _ => Err(ExtensionRpcResponse::failure(
            "hook_not_supported",
            format!("unsupported hook path '{path}'"),
        )),
    }
}

fn parse_json<T>(value: JsonValue) -> Result<T, ExtensionRpcResponse>
where
    T: for<'de> Deserialize<'de>,
{
    if value.is_null() {
        serde_json::from_value(serde_json::json!({}))
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string()))
    } else {
        serde_json::from_value(value)
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string()))
    }
}

fn sql_error(code: &'static str) -> impl Fn(sqlx::Error) -> ExtensionRpcResponse {
    move |error| ExtensionRpcResponse::failure(code, error.to_string())
}

fn memory_error(code: &'static str) -> impl Fn(model::MemoryError) -> ExtensionRpcResponse {
    move |error| ExtensionRpcResponse::failure(code, error.to_string())
}

fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn recall_mode_from(value: &str) -> memory_model::RecallMode {
    match value {
        "namespace" => memory_model::RecallMode::Namespace,
        "fts" => memory_model::RecallMode::Fts,
        _ => memory_model::RecallMode::Hybrid,
    }
}

fn review_action_from(value: &str) -> memory_model::ReviewActionKind {
    match value {
        "reject" => memory_model::ReviewActionKind::Reject,
        "supersede" => memory_model::ReviewActionKind::Supersede,
        "retire" => memory_model::ReviewActionKind::Retire,
        _ => memory_model::ReviewActionKind::Approve,
    }
}

fn hook_owner(event: &HookEventEnvelope) -> OwnerRef {
    event
        .owner
        .clone()
        .unwrap_or_else(|| OwnerRef::global("global"))
}

fn event_namespace(event: &HookEventEnvelope) -> String {
    event
        .resource
        .conversation_id
        .clone()
        .map(|conversation_id| format!("conversation/{conversation_id}"))
        .unwrap_or_else(|| format!("events/{}", event.event.replace('.', "/")))
}

fn collect_entities(event: &HookEventEnvelope, fallback: &str) -> Vec<String> {
    let mut entities = event
        .payload
        .get("conversation")
        .and_then(|item| item.get("participants"))
        .and_then(JsonValue::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(JsonValue::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if entities.is_empty() && !fallback.trim().is_empty() {
        entities.push(fallback.to_string());
    }
    entities
}
