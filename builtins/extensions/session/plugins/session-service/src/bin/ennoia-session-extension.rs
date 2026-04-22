use std::path::PathBuf;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::get,
    Json, Router,
};
use ennoia_kernel::{
    ConversationMessageHookPayload, ConversationSpec, ConversationTopology, HandoffSpec, LaneSpec,
    MessageRole, MessageSpec, OwnerRef,
};
use ennoia_paths::RuntimePaths;
use ennoia_session::{schema::initialize_session_schema, store::SessionStore};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use tokio::net::TcpListener;
use uuid::Uuid;

type ApiResult<T> = Result<Json<T>, (StatusCode, String)>;

#[derive(Clone)]
struct SessionApiState {
    store: SessionStore,
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

#[derive(Debug, Serialize)]
struct ConversationMessageResponse {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
    runs: Vec<serde_json::Value>,
    tasks: Vec<serde_json::Value>,
    artifacts: Vec<serde_json::Value>,
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let options = parse_options(std::env::args().skip(1).collect());
    let home_dir = options
        .home
        .unwrap_or_else(|| RuntimePaths::resolve(None).home().to_path_buf());
    let runtime_paths = RuntimePaths::new(home_dir);
    runtime_paths.ensure_layout()?;

    let db_path = runtime_paths.extension_sqlite_db("session", "session.db");
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let connect_options = SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(true);
    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;
    initialize_session_schema(&pool).await?;

    let state = SessionApiState {
        store: SessionStore::new(pool),
    };
    let router = Router::new()
        .route("/health", get(health))
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
        .route(
            "/lanes/{lane_id}/handoffs",
            get(lane_handoffs).post(lane_handoffs_create),
        )
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", options.port)).await?;
    axum::serve(listener, router).await?;
    Ok(())
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "service": "session-extension" }))
}

async fn conversations_list(State(state): State<SessionApiState>) -> Json<Vec<ConversationSpec>> {
    Json(state.store.list_conversations().await.unwrap_or_default())
}

async fn conversations_create(
    State(state): State<SessionApiState>,
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
        .store
        .upsert_conversation(&conversation)
        .await
        .map_err(internal)?;
    state.store.upsert_lane(&lane).await.map_err(internal)?;
    Ok(Json(ConversationCreateResponse {
        conversation,
        default_lane: lane,
    }))
}

async fn conversation_detail(
    State(state): State<SessionApiState>,
    Path(conversation_id): Path<String>,
) -> ApiResult<ConversationDetailResponse> {
    let conversation = state
        .store
        .get_conversation(&conversation_id)
        .await
        .map_err(internal)?
        .ok_or_else(not_found)?;
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(internal)?;
    Ok(Json(ConversationDetailResponse {
        conversation,
        lanes,
    }))
}

async fn conversation_delete(
    State(state): State<SessionApiState>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    if state
        .store
        .delete_conversation(&conversation_id)
        .await
        .map_err(internal)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(not_found())
    }
}

async fn conversation_messages(
    State(state): State<SessionApiState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<MessageSpec>> {
    Json(
        state
            .store
            .list_messages(&conversation_id)
            .await
            .unwrap_or_default(),
    )
}

async fn conversation_lanes(
    State(state): State<SessionApiState>,
    Path(conversation_id): Path<String>,
) -> Json<Vec<LaneSpec>> {
    Json(
        state
            .store
            .list_lanes(&conversation_id)
            .await
            .unwrap_or_default(),
    )
}

async fn conversation_messages_create(
    State(state): State<SessionApiState>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<ConversationMessagePayload>,
) -> ApiResult<ConversationMessageResponse> {
    let conversation = state
        .store
        .get_conversation(&conversation_id)
        .await
        .map_err(internal)?
        .ok_or_else(not_found)?;
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(internal)?;
    let lane = select_lane(&lanes, payload.lane_id.as_deref())
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "lane not found".to_string()))?;
    let workflow_payload = persist_operator_message(&state.store, conversation, lane, payload)
        .await
        .map_err(|error| (StatusCode::BAD_REQUEST, error))?;

    Ok(Json(ConversationMessageResponse {
        conversation: workflow_payload.conversation,
        lane: workflow_payload.lane,
        message: workflow_payload.message,
        runs: Vec::new(),
        tasks: Vec::new(),
        artifacts: Vec::new(),
    }))
}

async fn lane_handoffs(
    State(state): State<SessionApiState>,
    Path(lane_id): Path<String>,
) -> Json<Vec<HandoffSpec>> {
    Json(
        state
            .store
            .list_handoffs(&lane_id)
            .await
            .unwrap_or_default(),
    )
}

async fn lane_handoffs_create(
    State(state): State<SessionApiState>,
    Path(lane_id): Path<String>,
    Json(payload): Json<HandoffPayload>,
) -> ApiResult<HandoffSpec> {
    let handoff = HandoffSpec {
        id: format!("handoff-{}", Uuid::new_v4()),
        from_lane_id: lane_id,
        to_lane_id: payload.to_lane_id,
        from_agent_id: payload.from_agent_id,
        to_agent_id: payload.to_agent_id,
        summary: payload.summary,
        instructions: payload.instructions,
        status: payload.status.unwrap_or_else(|| "open".to_string()),
        created_at: now_iso(),
    };
    state
        .store
        .insert_handoff(&handoff)
        .await
        .map_err(internal)?;
    Ok(Json(handoff))
}

async fn persist_operator_message(
    store: &SessionStore,
    conversation: ConversationSpec,
    lane: LaneSpec,
    payload: ConversationMessagePayload,
) -> Result<ConversationMessageHookPayload, String> {
    let now = now_iso();
    let target_agents = resolve_addressed_agents(&conversation, &lane, payload.addressed_agents);
    if target_agents.is_empty() {
        return Err("no addressed agents resolved for this message".to_string());
    }
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
    store
        .insert_message(&message)
        .await
        .map_err(|error| error.to_string())?;

    let conversation = ConversationSpec {
        updated_at: now.clone(),
        ..conversation
    };
    store
        .upsert_conversation(&conversation)
        .await
        .map_err(|error| error.to_string())?;
    let lane = LaneSpec {
        updated_at: now,
        ..lane
    };
    store
        .upsert_lane(&lane)
        .await
        .map_err(|error| error.to_string())?;
    Ok(ConversationMessageHookPayload {
        conversation,
        lane,
        message,
        goal: payload.goal.unwrap_or(payload.body),
    })
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

fn internal(error: impl std::fmt::Display) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, error.to_string())
}

fn not_found() -> (StatusCode, String) {
    (StatusCode::NOT_FOUND, "conversation not found".to_string())
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

struct Options {
    home: Option<PathBuf>,
    port: u16,
}

fn parse_options(args: Vec<String>) -> Options {
    let mut home = None;
    let mut port = 3901;
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
