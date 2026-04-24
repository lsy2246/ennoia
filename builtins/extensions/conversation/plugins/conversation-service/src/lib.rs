pub mod conversations;
pub mod schema;

use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

use chrono::Utc;
use conversations::ConversationStore;
use ennoia_kernel::{
    ConversationSpec, ConversationTopology, ExtensionRpcResponse, LaneSpec, MessageRole,
    MessageSpec, OwnerKind, OwnerRef,
};
use ennoia_paths::RuntimePaths;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use uuid::Uuid;

use crate::schema::initialize_conversation_schema;

#[derive(Debug, Deserialize)]
struct Invocation {
    method: String,
    #[serde(default)]
    params: JsonValue,
    #[serde(default)]
    context: JsonValue,
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

#[derive(Debug, Deserialize, Default)]
struct ConversationLookupPayload {
    #[serde(default)]
    conversation_id: String,
}

#[derive(Debug, Deserialize, Default)]
struct ConversationMessagePayload {
    #[serde(default)]
    body: String,
    #[serde(default)]
    lane_id: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
    #[serde(default)]
    sender: Option<String>,
    #[serde(default)]
    role: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct AppendMessageParams {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    message: ConversationMessagePayload,
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

#[derive(Debug, Serialize)]
struct ConversationMessageResponse {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
    runs: Vec<JsonValue>,
    tasks: Vec<JsonValue>,
    artifacts: Vec<JsonValue>,
}

struct ConversationServiceState {
    store: ConversationStore,
}

pub fn module_name() -> &'static str {
    "conversation"
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime_paths = RuntimePaths::resolve(None);
    runtime_paths.ensure_layout()?;

    let database_path = runtime_paths.extension_sqlite_db("conversation", "conversation.db");
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
    initialize_conversation_schema(&pool).await?;
    let state = ConversationServiceState {
        store: ConversationStore::new(pool),
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
    state: &ConversationServiceState,
    invocation: Invocation,
) -> ExtensionRpcResponse {
    let path = invocation.method.trim_matches('/');
    let _context = invocation.context;
    match path {
        "conversation/conversations/list" => match state.store.list_conversations().await {
            Ok(conversations) => ExtensionRpcResponse::success(serde_json::json!(conversations)),
            Err(error) => {
                ExtensionRpcResponse::failure("conversation_list_failed", error.to_string())
            }
        },
        "conversation/conversations/create" => {
            match parse_json::<CreateConversationPayload>(invocation.params) {
                Ok(payload) => match create_conversation(state, payload).await {
                    Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/conversations/get" => {
            match parse_json::<ConversationLookupPayload>(invocation.params) {
                Ok(payload) => match conversation_detail(state, payload).await {
                    Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/conversations/delete" => {
            match parse_json::<ConversationLookupPayload>(invocation.params) {
                Ok(payload) => match delete_conversation(state, payload).await {
                    Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/lanes/list-by-conversation" => {
            match parse_json::<ConversationLookupPayload>(invocation.params) {
                Ok(payload) => match list_lanes(state, payload).await {
                    Ok(lanes) => ExtensionRpcResponse::success(serde_json::json!(lanes)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/messages/list" => {
            match parse_json::<ConversationLookupPayload>(invocation.params) {
                Ok(payload) => match list_messages(state, payload).await {
                    Ok(messages) => ExtensionRpcResponse::success(serde_json::json!(messages)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/messages/append-user" => {
            match parse_json::<AppendMessageParams>(invocation.params) {
                Ok(payload) => match append_user_message(state, payload).await {
                    Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        "conversation/messages/append-agent" => {
            match parse_json::<AppendMessageParams>(invocation.params) {
                Ok(payload) => match append_agent_message(state, payload).await {
                    Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                    Err(error) => error,
                },
                Err(error) => error,
            }
        }
        _ => ExtensionRpcResponse::failure(
            "method_not_found",
            format!("conversation worker method '{path}' not found"),
        ),
    }
}

async fn create_conversation(
    state: &ConversationServiceState,
    payload: CreateConversationPayload,
) -> Result<ConversationCreateResponse, ExtensionRpcResponse> {
    let topology = topology_from_payload(&payload)?;
    let agent_ids = payload
        .agent_ids
        .into_iter()
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    if agent_ids.is_empty() {
        return Err(ExtensionRpcResponse::failure(
            "conversation_agent_required",
            "at least one agent is required",
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
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_create_failed", error.to_string())
        })?;
    state.store.upsert_lane(&lane).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_lane_create_failed", error.to_string())
    })?;

    Ok(ConversationCreateResponse {
        conversation,
        default_lane: lane,
    })
}

async fn conversation_detail(
    state: &ConversationServiceState,
    payload: ConversationLookupPayload,
) -> Result<ConversationDetailResponse, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let conversation = state
        .store
        .get_conversation(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_get_failed", error.to_string())
        })?
        .ok_or_else(|| {
            ExtensionRpcResponse::failure("conversation_not_found", "conversation not found")
        })?;
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_lanes_failed", error.to_string())
        })?;

    Ok(ConversationDetailResponse {
        conversation,
        lanes,
    })
}

async fn delete_conversation(
    state: &ConversationServiceState,
    payload: ConversationLookupPayload,
) -> Result<JsonValue, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let deleted = state
        .store
        .delete_conversation(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_delete_failed", error.to_string())
        })?;
    Ok(serde_json::json!({ "deleted": deleted }))
}

async fn list_lanes(
    state: &ConversationServiceState,
    payload: ConversationLookupPayload,
) -> Result<Vec<LaneSpec>, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_lanes_failed", error.to_string())
        })
}

async fn list_messages(
    state: &ConversationServiceState,
    payload: ConversationLookupPayload,
) -> Result<Vec<MessageSpec>, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    state
        .store
        .list_messages(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })
}

async fn append_user_message(
    state: &ConversationServiceState,
    payload: AppendMessageParams,
) -> Result<ConversationMessageResponse, ExtensionRpcResponse> {
    append_message(state, payload, MessageRole::Operator, "operator").await
}

async fn append_agent_message(
    state: &ConversationServiceState,
    payload: AppendMessageParams,
) -> Result<ConversationMessageResponse, ExtensionRpcResponse> {
    append_message(state, payload, MessageRole::Agent, "agent").await
}

async fn append_message(
    state: &ConversationServiceState,
    payload: AppendMessageParams,
    default_role: MessageRole,
    default_sender: &str,
) -> Result<ConversationMessageResponse, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let body = payload.message.body.trim().to_string();
    let _goal = payload.message.goal.as_deref();
    if body.is_empty() {
        return Err(ExtensionRpcResponse::failure(
            "message_body_required",
            "message body is required",
        ));
    }

    let conversation = state
        .store
        .get_conversation(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_get_failed", error.to_string())
        })?
        .ok_or_else(|| {
            ExtensionRpcResponse::failure("conversation_not_found", "conversation not found")
        })?;
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_lanes_failed", error.to_string())
        })?;
    let lane = select_lane(&lanes, payload.message.lane_id.as_deref())
        .ok_or_else(|| ExtensionRpcResponse::failure("lane_not_found", "lane not found"))?;
    let target_agents =
        resolve_addressed_agents(&conversation, &lane, payload.message.addressed_agents);
    if target_agents.is_empty() {
        return Err(ExtensionRpcResponse::failure(
            "message_target_required",
            "no addressed agents resolved for this message",
        ));
    }

    let now = now_iso();
    let role = payload
        .message
        .role
        .as_deref()
        .map(message_role_from)
        .unwrap_or(default_role);
    let sender = payload
        .message
        .sender
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| default_sender.to_string());
    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        conversation_id: conversation.id.clone(),
        lane_id: Some(lane.id.clone()),
        sender,
        role,
        body,
        mentions: target_agents,
        created_at: now.clone(),
    };
    state
        .store
        .insert_message(&message)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("message_append_failed", error.to_string())
        })?;

    let conversation = ConversationSpec {
        updated_at: now.clone(),
        ..conversation
    };
    state
        .store
        .upsert_conversation(&conversation)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
        })?;
    let lane = LaneSpec {
        updated_at: now,
        ..lane
    };
    state
        .store
        .upsert_lane(&lane)
        .await
        .map_err(|error| ExtensionRpcResponse::failure("lane_update_failed", error.to_string()))?;

    Ok(ConversationMessageResponse {
        conversation,
        lane,
        message,
        runs: Vec::new(),
        tasks: Vec::new(),
        artifacts: Vec::new(),
    })
}

fn parse_json<T>(value: JsonValue) -> Result<T, ExtensionRpcResponse>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(value)
        .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string()))
}

fn required_conversation_id(value: &str) -> Result<String, ExtensionRpcResponse> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ExtensionRpcResponse::failure(
            "conversation_id_required",
            "conversation_id is required",
        ));
    }
    Ok(trimmed.to_string())
}

fn topology_from_payload(
    payload: &CreateConversationPayload,
) -> Result<ConversationTopology, ExtensionRpcResponse> {
    let requested = match payload.topology.as_str() {
        "direct" => ConversationTopology::Direct,
        "group" => ConversationTopology::Group,
        _ => {
            return Err(ExtensionRpcResponse::failure(
                "conversation_topology_invalid",
                "invalid conversation topology",
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
    let mut participants = Vec::new();
    push_unique(&mut participants, "operator");
    for agent_id in agent_ids {
        push_unique(&mut participants, agent_id);
    }
    participants
}

fn push_unique(values: &mut Vec<String>, value: impl AsRef<str>) {
    let value = value.as_ref().trim();
    if value.is_empty() || values.iter().any(|item| item == value) {
        return;
    }
    values.push(value.to_string());
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
        let mut resolved = Vec::new();
        for agent_id in addressed_agents {
            push_unique(&mut resolved, agent_id);
        }
        return resolved;
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

fn message_role_from(value: &str) -> MessageRole {
    match value {
        "agent" => MessageRole::Agent,
        "system" => MessageRole::System,
        "tool" => MessageRole::Tool,
        _ => MessageRole::Operator,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}
