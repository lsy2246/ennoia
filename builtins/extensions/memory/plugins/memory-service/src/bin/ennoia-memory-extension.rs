use std::path::PathBuf;
use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use ennoia_kernel::{
    ConversationSpec, ConversationTopology, LaneSpec, MemoryPolicy, MessageRole, MessageSpec,
    OwnerKind, OwnerRef,
};
use ennoia_memory::{
    conversations::ConversationStore, initialize_memory_schema, EpisodeKind, EpisodeRequest,
    MemoryKind, MemoryStore, RecallMode, RecallQuery, RememberRequest, ReviewAction,
    ReviewActionKind, SqliteMemoryStore, Stability,
};
use ennoia_paths::RuntimePaths;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::SqlitePool;
use tokio::net::TcpListener;
use uuid::Uuid;

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

#[derive(Clone)]
struct MemoryApiState {
    store: Arc<dyn MemoryStore>,
    conversations: ConversationStore,
    pool: SqlitePool,
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
    lane_id: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ConversationMessageResponse {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
    runs: Vec<serde_json::Value>,
    tasks: Vec<serde_json::Value>,
    artifacts: Vec<serde_json::Value>,
}

#[derive(Debug, Serialize)]
struct MemoryWorkspaceSummary {
    conversations: Vec<ConversationSpec>,
    pending_review_count: usize,
    active_memory_count: usize,
    message_count: i64,
    graph_nodes_count: i64,
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
            pool.clone(),
            Arc::new(MemoryPolicy::builtin()),
        )),
        conversations: ConversationStore::new(pool.clone()),
        pool,
    };

    let router = Router::new()
        .route("/health", get(health))
        .route("/workspace", get(workspace_summary))
        .route("/memories", get(list_memories))
        .route("/memories/remember", post(remember_memory))
        .route("/memories/recall", post(recall_memories))
        .route("/memories/review", post(review_memory))
        .route(
            "/conversations",
            get(conversations_list).post(conversations_create),
        )
        .route(
            "/conversations/{conversation_id}",
            get(conversation_detail).delete(conversation_delete),
        )
        .route(
            "/conversations/{conversation_id}/messages",
            get(conversation_messages).post(conversation_messages_create),
        )
        .route(
            "/conversations/{conversation_id}/lanes",
            get(conversation_lanes),
        )
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", options.port)).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "memory-extension" }))
}

async fn workspace_summary(
    State(state): State<MemoryApiState>,
) -> ApiResult<MemoryWorkspaceSummary> {
    let conversations = state
        .conversations
        .list_conversations()
        .await
        .map_err(internal_sql)?;
    let pending_review_count: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM memories WHERE review_state IN ('pending_review', 'pending') OR status = 'pending_review'",
    )
    .fetch_one(&state.pool)
    .await
    .map_err(internal_sql)?;
    let active_memory_count: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM memories WHERE status = 'active'")
            .fetch_one(&state.pool)
            .await
            .map_err(internal_sql)?;
    let message_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM messages")
        .fetch_one(&state.pool)
        .await
        .map_err(internal_sql)?;
    let graph_nodes_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM gm_nodes")
        .fetch_one(&state.pool)
        .await
        .map_err(internal_sql)?;

    Ok(Json(MemoryWorkspaceSummary {
        conversations,
        pending_review_count: pending_review_count.max(0) as usize,
        active_memory_count: active_memory_count.max(0) as usize,
        message_count,
        graph_nodes_count,
    }))
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

async fn conversations_list(State(state): State<MemoryApiState>) -> Json<Vec<ConversationSpec>> {
    Json(
        state
            .conversations
            .list_conversations()
            .await
            .unwrap_or_default(),
    )
}

async fn conversations_create(
    State(state): State<MemoryApiState>,
    Json(payload): Json<CreateConversationPayload>,
) -> ApiResult<ConversationCreateResponse> {
    let topology = topology_from_payload(&payload)?;
    let agent_ids = payload
        .agent_ids
        .into_iter()
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    if agent_ids.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "at least one agent is required".to_string(),
        ));
    }

    let now = now_iso();
    let conversation_id = format!("conv-{}", Uuid::new_v4());
    let lane_id = format!("lane-{}", Uuid::new_v4());
    let participants = build_participants(&agent_ids);
    let owner = match topology {
        ConversationTopology::Direct => OwnerRef::agent(agent_ids[0].clone()),
        ConversationTopology::Group => payload
            .space_id
            .clone()
            .map(OwnerRef::space)
            .unwrap_or_else(|| OwnerRef::global("global")),
    };
    let conversation = ConversationSpec {
        id: conversation_id.clone(),
        topology,
        owner,
        space_id: payload.space_id.clone(),
        title: payload
            .title
            .unwrap_or_else(|| default_conversation_title(&agent_ids)),
        participants: participants.clone(),
        default_lane_id: Some(lane_id.clone()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let lane = LaneSpec {
        id: lane_id,
        conversation_id,
        space_id: payload.space_id,
        name: payload
            .lane_name
            .unwrap_or_else(|| default_lane_name(&agent_ids)),
        lane_type: payload.lane_type.unwrap_or_else(|| "primary".to_string()),
        status: "active".to_string(),
        goal: payload
            .lane_goal
            .unwrap_or_else(|| default_lane_goal(&agent_ids)),
        participants,
        created_at: now.clone(),
        updated_at: now,
    };
    state
        .conversations
        .upsert_conversation(&conversation)
        .await
        .map_err(internal_sql)?;
    state
        .conversations
        .upsert_lane(&lane)
        .await
        .map_err(internal_sql)?;

    Ok(Json(ConversationCreateResponse {
        conversation,
        default_lane: lane,
    }))
}

async fn conversation_detail(
    State(state): State<MemoryApiState>,
    Path(conversation_id): Path<String>,
) -> ApiResult<ConversationDetailResponse> {
    let conversation = state
        .conversations
        .get_conversation(&conversation_id)
        .await
        .map_err(internal_sql)?
        .ok_or_else(not_found)?;
    let lanes = state
        .conversations
        .list_lanes(&conversation_id)
        .await
        .map_err(internal_sql)?;
    Ok(Json(ConversationDetailResponse {
        conversation,
        lanes,
    }))
}

async fn conversation_delete(
    State(state): State<MemoryApiState>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if state
        .conversations
        .delete_conversation(&conversation_id)
        .await
        .map_err(internal_sql)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

async fn conversation_messages(
    State(state): State<MemoryApiState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<MessageSpec>> {
    Json(
        state
            .conversations
            .list_messages(&conversation_id)
            .await
            .unwrap_or_default(),
    )
}

async fn conversation_lanes(
    State(state): State<MemoryApiState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<LaneSpec>> {
    Json(
        state
            .conversations
            .list_lanes(&conversation_id)
            .await
            .unwrap_or_default(),
    )
}

async fn conversation_messages_create(
    State(state): State<MemoryApiState>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<ConversationMessagePayload>,
) -> ApiResult<ConversationMessageResponse> {
    let _goal = payload.goal.clone();
    let conversation = state
        .conversations
        .get_conversation(&conversation_id)
        .await
        .map_err(internal_sql)?
        .ok_or_else(not_found)?;
    let lanes = state
        .conversations
        .list_lanes(&conversation_id)
        .await
        .map_err(internal_sql)?;
    let lane = select_lane(&lanes, payload.lane_id.as_deref())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "lane not found".to_string()))?;
    let target_agents = resolve_addressed_agents(&conversation, &lane, payload.addressed_agents);
    if target_agents.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "no addressed agents resolved for this message".to_string(),
        ));
    }

    let now = now_iso();
    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        conversation_id: conversation.id.clone(),
        lane_id: Some(lane.id.clone()),
        sender: "operator".to_string(),
        role: MessageRole::Operator,
        body: payload.body.clone(),
        mentions: target_agents,
        created_at: now.clone(),
    };
    state
        .conversations
        .insert_message(&message)
        .await
        .map_err(internal_sql)?;
    let conversation = ConversationSpec {
        updated_at: now.clone(),
        ..conversation
    };
    state
        .conversations
        .upsert_conversation(&conversation)
        .await
        .map_err(internal_sql)?;
    let lane = LaneSpec {
        updated_at: now.clone(),
        ..lane
    };
    state
        .conversations
        .upsert_lane(&lane)
        .await
        .map_err(internal_sql)?;

    let namespace = format!("recent/conversations/{}/L1#core", conversation.id);
    state
        .store
        .record_episode(EpisodeRequest {
            owner: conversation.owner.clone(),
            namespace,
            conversation_id: Some(conversation.id.clone()),
            run_id: None,
            episode_kind: EpisodeKind::Message,
            role: Some(message_role_str(message.role).to_string()),
            content: message.body.clone(),
            content_type: Some("text/plain".to_string()),
            source_uri: None,
            entities: Vec::new(),
            tags: vec!["conversation".to_string(), message.sender.clone()],
            importance: Some(0.4),
            occurred_at: Some(message.created_at.clone()),
        })
        .await
        .map_err(internal_error)?;

    Ok(Json(ConversationMessageResponse {
        conversation,
        lane,
        message,
        runs: Vec::new(),
        tasks: Vec::new(),
        artifacts: Vec::new(),
    }))
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

fn message_role_str(role: MessageRole) -> &'static str {
    match role {
        MessageRole::Operator => "operator",
        MessageRole::Agent => "agent",
        MessageRole::System => "system",
        MessageRole::Tool => "tool",
    }
}

fn topology_from_payload(
    payload: &CreateConversationPayload,
) -> Result<ConversationTopology, (StatusCode, String)> {
    let requested = match payload.topology.as_str() {
        "direct" => ConversationTopology::Direct,
        "group" => ConversationTopology::Group,
        _ => {
            return Err((
                StatusCode::BAD_REQUEST,
                "invalid conversation topology".to_string(),
            ))
        }
    };
    if payload.agent_ids.len() > 1 {
        Ok(ConversationTopology::Group)
    } else {
        Ok(requested)
    }
}

fn build_participants(agent_ids: &[String]) -> Vec<String> {
    let mut participants = vec!["operator".to_string()];
    participants.extend(agent_ids.iter().cloned());
    participants
}

fn select_lane(lanes: &[LaneSpec], lane_id: Option<&str>) -> Option<LaneSpec> {
    lane_id
        .and_then(|id| lanes.iter().find(|lane| lane.id == id).cloned())
        .or_else(|| lanes.first().cloned())
}

fn resolve_addressed_agents(
    conversation: &ConversationSpec,
    lane: &LaneSpec,
    addressed_agents: Vec<String>,
) -> Vec<String> {
    if !addressed_agents.is_empty() {
        return addressed_agents;
    }
    let source = if lane.participants.is_empty() {
        &conversation.participants
    } else {
        &lane.participants
    };
    source
        .iter()
        .filter(|participant| participant.as_str() != "operator")
        .cloned()
        .collect()
}

fn default_conversation_title(agent_ids: &[String]) -> String {
    if agent_ids.len() <= 1 {
        format!(
            "与 {} 的会话",
            agent_ids.first().cloned().unwrap_or_default()
        )
    } else {
        format!(
            "{} 协作会话",
            agent_ids
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join("、")
        )
    }
}

fn default_lane_name(agent_ids: &[String]) -> String {
    if agent_ids.len() <= 1 {
        "私聊".to_string()
    } else {
        "群聊".to_string()
    }
}

fn default_lane_goal(agent_ids: &[String]) -> String {
    if agent_ids.len() <= 1 {
        "与目标 Agent 持续推进当前问题".to_string()
    } else {
        "在多 Agent 协作中持续推进当前问题".to_string()
    }
}

fn not_found() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "conversation not found".to_string())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn internal_error(error: ennoia_memory::MemoryError) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn internal_sql(error: sqlx::Error) -> (StatusCode, String) {
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
