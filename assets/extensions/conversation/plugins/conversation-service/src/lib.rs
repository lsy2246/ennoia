pub mod conversations;
pub mod schema;

use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};

use std::collections::{HashMap, HashSet};

use chrono::{DateTime, Utc};
use conversations::ConversationStore;
use ennoia_kernel::{
    ConversationBranchSpec, ConversationSpec, ConversationTopology, ExtensionRpcResponse, LaneSpec,
    MessageRole, MessageSpec, OwnerKind, OwnerRef, RuntimeProfile,
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
struct BranchLookupPayload {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    branch_id: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct UpdateBranchPayload {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    branch_id: String,
    #[serde(default)]
    name: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct DeleteBranchPayload {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    branch_id: String,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct CreateBranchPayload {
    #[serde(default)]
    conversation_id: String,
    #[serde(default)]
    from_branch_id: Option<String>,
    #[serde(default)]
    source_message_id: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    activate: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct ConversationMessagePayload {
    #[serde(default)]
    body: String,
    #[serde(default)]
    format: Option<String>,
    #[serde(default)]
    lane_id: Option<String>,
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
    #[serde(default)]
    mentions: Vec<String>,
    #[serde(default)]
    sender: Option<String>,
    #[serde(default)]
    role: Option<String>,
    #[serde(default)]
    branch_id: Option<String>,
    #[serde(default)]
    parent_message_id: Option<String>,
    #[serde(default)]
    fork_from_message_id: Option<String>,
    #[serde(default)]
    rewrite_from_message_id: Option<String>,
    #[serde(default)]
    branch_name: Option<String>,
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
    branches: Vec<ConversationBranchView>,
    messages: Vec<MessageSpec>,
}

#[derive(Debug, Serialize)]
struct ConversationMessageResponse {
    conversation: ConversationSpec,
    lane: LaneSpec,
    branch: ConversationBranchSpec,
    message: MessageSpec,
    addressed_agents: Vec<String>,
    runs: Vec<JsonValue>,
    tasks: Vec<JsonValue>,
    artifacts: Vec<JsonValue>,
}

#[derive(Debug, Serialize)]
struct ConversationBranchView {
    #[serde(flatten)]
    branch: ConversationBranchSpec,
    is_active: bool,
    depth: usize,
    own_message_count: usize,
    visible_message_count: usize,
    last_message_at: Option<String>,
    last_activity_at: String,
    source_preview: Option<String>,
}

struct ConversationServiceState {
    store: ConversationStore,
    runtime_paths: RuntimePaths,
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
        runtime_paths,
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
        "conversation.list" => match state.store.list_conversations().await {
            Ok(conversations) => ExtensionRpcResponse::success(serde_json::json!(conversations)),
            Err(error) => {
                ExtensionRpcResponse::failure("conversation_list_failed", error.to_string())
            }
        },
        "conversation.create" => match parse_json::<CreateConversationPayload>(invocation.params) {
            Ok(payload) => match create_conversation(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "conversation.get" => match parse_json::<ConversationLookupPayload>(invocation.params) {
            Ok(payload) => match conversation_detail(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "conversation.delete" => match parse_json::<ConversationLookupPayload>(invocation.params) {
            Ok(payload) => match delete_conversation(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "lane.list" => match parse_json::<ConversationLookupPayload>(invocation.params) {
            Ok(payload) => match list_lanes(state, payload).await {
                Ok(lanes) => ExtensionRpcResponse::success(serde_json::json!(lanes)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "branch.list" => match parse_json::<ConversationLookupPayload>(invocation.params) {
            Ok(payload) => match list_branches(state, payload).await {
                Ok(branches) => ExtensionRpcResponse::success(serde_json::json!(branches)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "branch.create" => match parse_json::<CreateBranchPayload>(invocation.params) {
            Ok(payload) => match create_branch(state, payload).await {
                Ok(branch) => ExtensionRpcResponse::success(serde_json::json!(branch)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "branch.switch" => match parse_json::<BranchLookupPayload>(invocation.params) {
            Ok(payload) => match switch_branch(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "branch.update" => match parse_json::<UpdateBranchPayload>(invocation.params) {
            Ok(payload) => match update_branch(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "branch.delete" => match parse_json::<DeleteBranchPayload>(invocation.params) {
            Ok(payload) => match delete_branch(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "message.list" => match parse_json::<BranchLookupPayload>(invocation.params) {
            Ok(payload) => match list_messages(state, payload).await {
                Ok(messages) => ExtensionRpcResponse::success(serde_json::json!(messages)),
                Err(error) => error,
            },
            Err(error) => error,
        },
        "message.append" => match parse_json::<AppendMessageParams>(invocation.params) {
            Ok(payload) => match append_routed_message(state, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => error,
            },
            Err(error) => error,
        },
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
    let branch_id = format!("branch-{}", Uuid::new_v4());
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
        active_branch_id: Some(branch_id.clone()),
        default_lane_id: Some(branch_id.clone()),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let lane = LaneSpec {
        id: branch_id.clone(),
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
    let branch = ConversationBranchSpec {
        id: branch_id,
        conversation_id: conversation.id.clone(),
        name: lane.name.clone(),
        kind: "main".to_string(),
        status: "active".to_string(),
        parent_branch_id: None,
        source_message_id: None,
        inherit_mode: "inclusive".to_string(),
        created_at: lane.created_at.clone(),
        updated_at: lane.updated_at.clone(),
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
    state.store.upsert_branch(&branch).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_branch_create_failed", error.to_string())
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
    let branches = state
        .store
        .list_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let all_branches = state
        .store
        .list_all_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let all_messages = state
        .store
        .list_messages(&conversation_id, None)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })?;
    let messages = if let Some(active_branch_id) = conversation.active_branch_id.as_deref() {
        state
            .store
            .list_messages(&conversation_id, Some(active_branch_id))
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
            })?
    } else {
        Vec::new()
    };
    let active_branch_id = conversation.active_branch_id.clone();

    Ok(ConversationDetailResponse {
        conversation,
        lanes,
        branches: build_branch_views(
            &branches,
            &all_branches,
            &all_messages,
            active_branch_id.as_deref(),
        ),
        messages,
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

async fn list_branches(
    state: &ConversationServiceState,
    payload: ConversationLookupPayload,
) -> Result<Vec<ConversationBranchView>, ExtensionRpcResponse> {
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
    let branches = state
        .store
        .list_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let all_branches = state
        .store
        .list_all_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let all_messages = state
        .store
        .list_messages(&conversation_id, None)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })?;
    Ok(build_branch_views(
        &branches,
        &all_branches,
        &all_messages,
        conversation.active_branch_id.as_deref(),
    ))
}

async fn list_messages(
    state: &ConversationServiceState,
    payload: BranchLookupPayload,
) -> Result<Vec<MessageSpec>, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let active_branch_id = if let Some(branch_id) = payload.branch_id.as_deref() {
        Some(required_non_empty(branch_id, "branch_id")?)
    } else {
        state
            .store
            .get_conversation(&conversation_id)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_get_failed", error.to_string())
            })?
            .and_then(|conversation| conversation.active_branch_id)
    };
    let Some(active_branch_id) = active_branch_id else {
        return Ok(Vec::new());
    };
    state
        .store
        .list_messages(&conversation_id, Some(active_branch_id.as_str()))
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })
}

async fn create_branch(
    state: &ConversationServiceState,
    payload: CreateBranchPayload,
) -> Result<ConversationBranchSpec, ExtensionRpcResponse> {
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
    let all_messages = state
        .store
        .list_messages(&conversation_id, None)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })?;
    let parent_branch_id = payload
        .from_branch_id
        .clone()
        .or_else(|| conversation.active_branch_id.clone())
        .or_else(|| conversation.default_lane_id.clone())
        .ok_or_else(|| {
            ExtensionRpcResponse::failure(
                "branch_parent_required",
                "conversation has no active branch",
            )
        })?;
    let parent_lane = select_lane(&lanes, Some(&parent_branch_id))
        .ok_or_else(|| ExtensionRpcResponse::failure("lane_not_found", "lane not found"))?;
    let mode = normalize_branch_mode(payload.mode.as_deref());
    let now = now_iso();
    let branch_id = format!("branch-{}", Uuid::new_v4());
    let source_preview = payload
        .source_message_id
        .as_deref()
        .and_then(|id| find_message_body(&all_messages, id))
        .map(ToOwned::to_owned);
    let branch_name = payload
        .name
        .clone()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| {
            default_branch_name(&mode, &parent_lane.name, source_preview.as_deref(), &now)
        });
    let lane = LaneSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        space_id: conversation.space_id.clone(),
        name: branch_name.clone(),
        lane_type: "branch".to_string(),
        status: "active".to_string(),
        goal: parent_lane.goal.clone(),
        participants: parent_lane.participants.clone(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let branch = ConversationBranchSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        name: branch_name,
        kind: mode.clone(),
        status: "active".to_string(),
        parent_branch_id: Some(parent_branch_id.clone()),
        source_message_id: payload.source_message_id.clone(),
        inherit_mode: inherit_mode_for_branch_mode(&mode).to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    state.store.upsert_lane(&lane).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_lane_create_failed", error.to_string())
    })?;
    state.store.upsert_branch(&branch).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_branch_create_failed", error.to_string())
    })?;

    if payload.activate.unwrap_or(true) {
        let updated = ConversationSpec {
            active_branch_id: Some(branch.id.clone()),
            default_lane_id: Some(branch.id.clone()),
            updated_at: now,
            ..conversation
        };
        state
            .store
            .upsert_conversation(&updated)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
            })?;
    }

    Ok(branch)
}

async fn switch_branch(
    state: &ConversationServiceState,
    payload: BranchLookupPayload,
) -> Result<ConversationDetailResponse, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let branch_id = payload
        .branch_id
        .as_deref()
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_id_required", "branch_id is required"))
        .and_then(|value| required_non_empty(value, "branch_id"))?;
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
    let branch = state
        .store
        .get_branch(&branch_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branch_get_failed", error.to_string())
        })?
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_not_found", "branch not found"))?;
    if branch.conversation_id != conversation_id {
        return Err(ExtensionRpcResponse::failure(
            "branch_mismatch",
            "branch does not belong to the conversation",
        ));
    }

    let updated = ConversationSpec {
        active_branch_id: Some(branch_id.clone()),
        default_lane_id: Some(branch_id),
        updated_at: now_iso(),
        ..conversation
    };
    state
        .store
        .upsert_conversation(&updated)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
        })?;
    conversation_detail(state, ConversationLookupPayload { conversation_id }).await
}

async fn append_routed_message(
    state: &ConversationServiceState,
    payload: AppendMessageParams,
) -> Result<ConversationMessageResponse, ExtensionRpcResponse> {
    let role = payload
        .message
        .role
        .as_deref()
        .map(message_role_from)
        .unwrap_or(MessageRole::Operator);
    let default_sender = if role == MessageRole::Operator {
        "operator"
    } else {
        "agent"
    };
    append_message(state, payload, role, default_sender).await
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

    let mut conversation = state
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
    let branches = state
        .store
        .list_all_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let all_messages = state
        .store
        .list_messages(&conversation_id, None)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_messages_failed", error.to_string())
        })?;

    let target_branch_id = payload
        .message
        .branch_id
        .clone()
        .or(payload.message.lane_id.clone())
        .or_else(|| conversation.active_branch_id.clone())
        .or_else(|| conversation.default_lane_id.clone());
    let active_branches = branches
        .iter()
        .filter(|branch| branch.status != "deleted")
        .cloned()
        .collect::<Vec<_>>();
    let mut lane = if let Some(target_branch_id) = target_branch_id.as_deref() {
        select_lane(&lanes, Some(target_branch_id))
            .ok_or_else(|| ExtensionRpcResponse::failure("lane_not_found", "lane not found"))?
    } else {
        let created = create_root_runtime_branch(
            state,
            &conversation,
            payload.message.branch_name.as_deref(),
        )
        .await?;
        conversation = ConversationSpec {
            active_branch_id: Some(created.1.id.clone()),
            default_lane_id: Some(created.0.id.clone()),
            updated_at: created.0.updated_at.clone(),
            ..conversation
        };
        state
            .store
            .upsert_conversation(&conversation)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
            })?;
        created.0
    };
    let mut branch = select_branch(&active_branches, Some(&lane.id))
        .or_else(|| select_branch(&active_branches, target_branch_id.as_deref()))
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_not_found", "branch not found"))?;

    let branch_mode = if payload
        .message
        .rewrite_from_message_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        Some("rewrite".to_string())
    } else if payload
        .message
        .fork_from_message_id
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty())
    {
        Some("fork".to_string())
    } else {
        None
    };
    let will_create_branch = branch_mode.is_some();
    let mut conversation = conversation;
    if let Some(mode) = branch_mode {
        let source_message_id = payload
            .message
            .rewrite_from_message_id
            .clone()
            .or(payload.message.fork_from_message_id.clone());
        if let Some(source_id) = source_message_id.as_deref() {
            ensure_message_exists(&all_messages, source_id)?;
        }

        let created = create_runtime_branch(
            state,
            &conversation,
            &lane,
            payload.message.branch_name.as_deref(),
            &mode,
            source_message_id.clone(),
            source_message_id
                .as_deref()
                .and_then(|id| find_message_body(&all_messages, id)),
        )
        .await?;
        lane = created.0;
        branch = created.1;
        conversation = ConversationSpec {
            active_branch_id: Some(branch.id.clone()),
            default_lane_id: Some(lane.id.clone()),
            updated_at: lane.updated_at.clone(),
            ..conversation
        };
        state
            .store
            .upsert_conversation(&conversation)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
            })?;
    }
    if let Some(parent_message_id) = payload.message.parent_message_id.as_deref() {
        ensure_message_exists(&all_messages, parent_message_id)?;
    }
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
        .map(|value| normalize_operator_sender(&state.runtime_paths, role, value, default_sender))
        .unwrap_or_else(|| resolve_default_sender(&state.runtime_paths, role, default_sender));
    let explicit_mentions = payload
        .message
        .mentions
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    let format = normalize_message_format(payload.message.format.as_deref());
    let message = MessageSpec {
        id: format!("msg-{}", Uuid::new_v4()),
        conversation_id: conversation.id.clone(),
        branch_id: Some(branch.id.clone()),
        lane_id: Some(lane.id.clone()),
        sender,
        role,
        body,
        format,
        mentions: explicit_mentions,
        parent_message_id: payload.message.parent_message_id.clone(),
        reply_to_message_id: payload.message.fork_from_message_id.clone(),
        rewrite_from_message_id: payload.message.rewrite_from_message_id.clone(),
        created_at: now.clone(),
    };
    state
        .store
        .insert_message(&message)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("message_append_failed", error.to_string())
        })?;

    let should_activate_branch = should_activate_message_branch(
        role,
        will_create_branch,
        conversation.active_branch_id.as_deref(),
    );
    let conversation = ConversationSpec {
        active_branch_id: if should_activate_branch {
            Some(branch.id.clone())
        } else {
            conversation.active_branch_id.clone()
        },
        default_lane_id: if should_activate_branch {
            Some(lane.id.clone())
        } else {
            conversation.default_lane_id.clone()
        },
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
    let branch = ConversationBranchSpec {
        updated_at: lane.updated_at.clone(),
        ..branch
    };
    state.store.upsert_branch(&branch).await.map_err(|error| {
        ExtensionRpcResponse::failure("branch_update_failed", error.to_string())
    })?;

    Ok(ConversationMessageResponse {
        conversation,
        lane,
        branch,
        message,
        addressed_agents: target_agents,
        runs: Vec::new(),
        tasks: Vec::new(),
        artifacts: Vec::new(),
    })
}

async fn create_runtime_branch(
    state: &ConversationServiceState,
    conversation: &ConversationSpec,
    parent_lane: &LaneSpec,
    requested_name: Option<&str>,
    mode: &str,
    source_message_id: Option<String>,
    source_preview: Option<&str>,
) -> Result<(LaneSpec, ConversationBranchSpec), ExtensionRpcResponse> {
    let now = now_iso();
    let branch_id = format!("branch-{}", Uuid::new_v4());
    let branch_name = requested_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_branch_name(mode, &parent_lane.name, source_preview, &now));
    let lane = LaneSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        space_id: conversation.space_id.clone(),
        name: branch_name.clone(),
        lane_type: "branch".to_string(),
        status: "active".to_string(),
        goal: parent_lane.goal.clone(),
        participants: parent_lane.participants.clone(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let branch = ConversationBranchSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        name: branch_name.clone(),
        kind: mode.to_string(),
        status: "active".to_string(),
        parent_branch_id: Some(parent_lane.id.clone()),
        source_message_id: source_message_id.clone(),
        inherit_mode: inherit_mode_for_branch_mode(mode).to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
    };

    state.store.upsert_lane(&lane).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_lane_create_failed", error.to_string())
    })?;
    state.store.upsert_branch(&branch).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_branch_create_failed", error.to_string())
    })?;

    Ok((lane, branch))
}

async fn create_root_runtime_branch(
    state: &ConversationServiceState,
    conversation: &ConversationSpec,
    requested_name: Option<&str>,
) -> Result<(LaneSpec, ConversationBranchSpec), ExtensionRpcResponse> {
    let now = now_iso();
    let branch_id = format!("branch-{}", Uuid::new_v4());
    let branch_name = requested_name
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| default_branch_name("main", "会话", None, &now));
    let participants = if conversation.participants.is_empty() {
        vec!["operator".to_string()]
    } else {
        conversation.participants.clone()
    };
    let lane = LaneSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        space_id: conversation.space_id.clone(),
        name: branch_name.clone(),
        lane_type: "branch".to_string(),
        status: "active".to_string(),
        goal: "继续推进当前问题".to_string(),
        participants,
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let branch = ConversationBranchSpec {
        id: branch_id.clone(),
        conversation_id: conversation.id.clone(),
        name: branch_name,
        kind: "main".to_string(),
        status: "active".to_string(),
        parent_branch_id: None,
        source_message_id: None,
        inherit_mode: "inclusive".to_string(),
        created_at: now.clone(),
        updated_at: now,
    };
    state.store.upsert_lane(&lane).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_lane_create_failed", error.to_string())
    })?;
    state.store.upsert_branch(&branch).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_branch_create_failed", error.to_string())
    })?;
    Ok((lane, branch))
}

async fn update_branch(
    state: &ConversationServiceState,
    payload: UpdateBranchPayload,
) -> Result<ConversationBranchSpec, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let branch_id = required_non_empty(&payload.branch_id, "branch_id")?;
    let branch = state
        .store
        .get_branch(&branch_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branch_get_failed", error.to_string())
        })?
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_not_found", "branch not found"))?;
    if branch.conversation_id != conversation_id {
        return Err(ExtensionRpcResponse::failure(
            "branch_mismatch",
            "branch does not belong to the conversation",
        ));
    }
    if branch.status == "deleted" {
        return Err(ExtensionRpcResponse::failure(
            "branch_deleted",
            "branch has been deleted",
        ));
    }
    let next_name = payload
        .name
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_name_required", "name is required"))?
        .to_string();
    let updated_at = now_iso();
    let updated = ConversationBranchSpec {
        name: next_name.clone(),
        updated_at: updated_at.clone(),
        ..branch.clone()
    };
    state.store.upsert_branch(&updated).await.map_err(|error| {
        ExtensionRpcResponse::failure("conversation_branch_update_failed", error.to_string())
    })?;
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_lanes_failed", error.to_string())
        })?;
    if let Some(lane) = lanes.into_iter().find(|item| item.id == branch_id) {
        let updated_lane = LaneSpec {
            name: next_name,
            updated_at,
            ..lane
        };
        state
            .store
            .upsert_lane(&updated_lane)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_lane_update_failed", error.to_string())
            })?;
    }
    Ok(updated)
}

async fn delete_branch(
    state: &ConversationServiceState,
    payload: DeleteBranchPayload,
) -> Result<ConversationDetailResponse, ExtensionRpcResponse> {
    let conversation_id = required_conversation_id(&payload.conversation_id)?;
    let branch_id = required_non_empty(&payload.branch_id, "branch_id")?;
    let mode = normalize_branch_delete_mode(payload.mode.as_deref());
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
    let all_branches = state
        .store
        .list_all_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let target = all_branches
        .iter()
        .find(|branch| branch.id == branch_id)
        .cloned()
        .ok_or_else(|| ExtensionRpcResponse::failure("branch_not_found", "branch not found"))?;
    if target.status == "deleted" {
        return Err(ExtensionRpcResponse::failure(
            "branch_deleted",
            "branch has already been deleted",
        ));
    }
    let deleted_ids = if mode == "delete_tree" {
        collect_branch_subtree_ids(&all_branches, &branch_id)
    } else {
        let mut ids = HashSet::new();
        ids.insert(branch_id.clone());
        ids
    };
    let now = now_iso();
    let lanes = state
        .store
        .list_lanes(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_lanes_failed", error.to_string())
        })?;

    if mode == "detach_children" {
        let direct_children = all_branches
            .iter()
            .filter(|branch| {
                branch.parent_branch_id.as_deref() == Some(branch_id.as_str())
                    && branch.status != "deleted"
            })
            .cloned()
            .collect::<Vec<_>>();
        for child in direct_children {
            let updated = ConversationBranchSpec {
                parent_branch_id: target.parent_branch_id.clone(),
                updated_at: now.clone(),
                ..child
            };
            state.store.upsert_branch(&updated).await.map_err(|error| {
                ExtensionRpcResponse::failure(
                    "conversation_branch_update_failed",
                    error.to_string(),
                )
            })?;
        }
    }

    for branch in all_branches
        .iter()
        .filter(|branch| deleted_ids.contains(&branch.id))
        .cloned()
    {
        let updated = ConversationBranchSpec {
            status: "deleted".to_string(),
            updated_at: now.clone(),
            ..branch
        };
        state.store.upsert_branch(&updated).await.map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branch_delete_failed", error.to_string())
        })?;
    }
    for lane in lanes
        .into_iter()
        .filter(|lane| deleted_ids.contains(&lane.id))
    {
        let updated_lane = LaneSpec {
            status: "deleted".to_string(),
            updated_at: now.clone(),
            ..lane
        };
        state
            .store
            .upsert_lane(&updated_lane)
            .await
            .map_err(|error| {
                ExtensionRpcResponse::failure("conversation_lane_update_failed", error.to_string())
            })?;
    }

    let refreshed_branches = state
        .store
        .list_all_branches(&conversation_id)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_branches_failed", error.to_string())
        })?;
    let next_active_branch_id =
        choose_next_active_branch(&conversation, &refreshed_branches, &branch_id, &deleted_ids);
    let updated_conversation = ConversationSpec {
        active_branch_id: next_active_branch_id.clone(),
        default_lane_id: next_active_branch_id.clone(),
        updated_at: now,
        ..conversation
    };
    state
        .store
        .upsert_conversation(&updated_conversation)
        .await
        .map_err(|error| {
            ExtensionRpcResponse::failure("conversation_update_failed", error.to_string())
        })?;
    conversation_detail(state, ConversationLookupPayload { conversation_id }).await
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

fn required_non_empty(value: &str, field: &str) -> Result<String, ExtensionRpcResponse> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ExtensionRpcResponse::failure(
            format!("{field}_required"),
            format!("{field} is required"),
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

fn select_branch(
    branches: &[ConversationBranchSpec],
    branch_id: Option<&str>,
) -> Option<ConversationBranchSpec> {
    branch_id
        .and_then(|id| branches.iter().find(|branch| branch.id == id).cloned())
        .or_else(|| branches.first().cloned())
}

fn select_lane(lanes: &[LaneSpec], lane_id: Option<&str>) -> Option<LaneSpec> {
    lane_id
        .and_then(|id| lanes.iter().find(|lane| lane.id == id).cloned())
        .or_else(|| lanes.first().cloned())
}

fn ensure_message_exists(
    messages: &[MessageSpec],
    message_id: &str,
) -> Result<(), ExtensionRpcResponse> {
    if messages.iter().any(|message| message.id == message_id) {
        return Ok(());
    }
    Err(ExtensionRpcResponse::failure(
        "message_not_found",
        "source message not found",
    ))
}

fn find_message_body<'a>(messages: &'a [MessageSpec], message_id: &str) -> Option<&'a str> {
    messages
        .iter()
        .find(|message| message.id == message_id)
        .map(|message| message.body.as_str())
}

fn normalize_branch_mode(mode: Option<&str>) -> String {
    match mode.unwrap_or("fork") {
        "rewrite" => "rewrite".to_string(),
        "reset" => "reset".to_string(),
        _ => "fork".to_string(),
    }
}

fn normalize_branch_delete_mode(mode: Option<&str>) -> &'static str {
    match mode.unwrap_or("detach_children") {
        "delete_tree" => "delete_tree",
        _ => "detach_children",
    }
}

fn should_activate_message_branch(
    role: MessageRole,
    created_branch: bool,
    current_active_branch_id: Option<&str>,
) -> bool {
    role == MessageRole::Operator || created_branch || current_active_branch_id.is_none()
}

fn inherit_mode_for_branch_mode(mode: &str) -> &'static str {
    match mode {
        "rewrite" => "exclusive",
        "reset" => "none",
        _ => "inclusive",
    }
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

fn collect_branch_subtree_ids(
    branches: &[ConversationBranchSpec],
    root_id: &str,
) -> HashSet<String> {
    let mut pending = vec![root_id.to_string()];
    let mut visited = HashSet::new();
    while let Some(branch_id) = pending.pop() {
        if !visited.insert(branch_id.clone()) {
            continue;
        }
        for child in branches
            .iter()
            .filter(|branch| branch.parent_branch_id.as_deref() == Some(branch_id.as_str()))
        {
            pending.push(child.id.clone());
        }
    }
    visited
}

fn choose_next_active_branch(
    conversation: &ConversationSpec,
    branches: &[ConversationBranchSpec],
    deleted_branch_id: &str,
    deleted_ids: &HashSet<String>,
) -> Option<String> {
    let active_branches = branches
        .iter()
        .filter(|branch| branch.status != "deleted")
        .cloned()
        .collect::<Vec<_>>();
    if active_branches.is_empty() {
        return None;
    }

    let direct_children = active_branches
        .iter()
        .filter(|branch| branch.parent_branch_id.as_deref() == Some(deleted_branch_id))
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
        .map(|branch| branch.id.clone());
    if direct_children.is_some() {
        return direct_children;
    }

    let target_parent = branches
        .iter()
        .find(|branch| branch.id == deleted_branch_id)
        .and_then(|branch| branch.parent_branch_id.clone());
    if let Some(parent_id) = target_parent {
        let sibling = active_branches
            .iter()
            .filter(|branch| {
                branch.parent_branch_id == Some(parent_id.clone())
                    && !deleted_ids.contains(&branch.id)
            })
            .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
            .map(|branch| branch.id.clone());
        if sibling.is_some() {
            return sibling;
        }
        if active_branches.iter().any(|branch| branch.id == parent_id) {
            return Some(parent_id);
        }
    }

    if let Some(current_id) = conversation.active_branch_id.as_deref() {
        if !deleted_ids.contains(current_id)
            && active_branches.iter().any(|branch| branch.id == current_id)
        {
            return Some(current_id.to_string());
        }
    }

    active_branches
        .iter()
        .max_by(|left, right| left.updated_at.cmp(&right.updated_at))
        .map(|branch| branch.id.clone())
}

fn build_branch_views(
    branches: &[ConversationBranchSpec],
    all_branches: &[ConversationBranchSpec],
    messages: &[MessageSpec],
    active_branch_id: Option<&str>,
) -> Vec<ConversationBranchView> {
    let branch_map = all_branches
        .iter()
        .cloned()
        .map(|branch| (branch.id.clone(), branch))
        .collect::<HashMap<_, _>>();
    branches
        .iter()
        .cloned()
        .map(|branch| {
            let own_messages = messages
                .iter()
                .filter(|message| {
                    message.branch_id.as_deref() == Some(branch.id.as_str())
                        || message.lane_id.as_deref() == Some(branch.id.as_str())
                })
                .collect::<Vec<_>>();
            let visible_messages =
                conversations::filter_visible_messages(messages, all_branches, &branch.id);
            let last_message_at = visible_messages
                .last()
                .map(|message| message.created_at.clone())
                .or_else(|| {
                    own_messages
                        .last()
                        .map(|message| message.created_at.clone())
                });
            ConversationBranchView {
                is_active: active_branch_id == Some(branch.id.as_str()),
                depth: branch_visible_depth(&branch, &branch_map),
                own_message_count: own_messages.len(),
                visible_message_count: visible_messages.len(),
                last_message_at: last_message_at.clone(),
                last_activity_at: last_message_at.unwrap_or_else(|| branch.updated_at.clone()),
                source_preview: branch_source_preview(&branch, messages),
                branch,
            }
        })
        .collect()
}

fn branch_visible_depth(
    branch: &ConversationBranchSpec,
    branch_map: &HashMap<String, ConversationBranchSpec>,
) -> usize {
    let mut depth = 0;
    let mut current = branch
        .parent_branch_id
        .as_ref()
        .and_then(|id| branch_map.get(id));
    let mut visiting = HashSet::new();
    while let Some(parent) = current {
        if !visiting.insert(parent.id.clone()) {
            break;
        }
        if parent.status != "deleted" {
            depth += 1;
        }
        current = parent
            .parent_branch_id
            .as_ref()
            .and_then(|id| branch_map.get(id));
    }
    depth
}

fn branch_source_preview(
    branch: &ConversationBranchSpec,
    messages: &[MessageSpec],
) -> Option<String> {
    if let Some(message_id) = branch.source_message_id.as_deref() {
        if let Some(body) = find_message_body(messages, message_id) {
            let summary = summarize_branch_source(body);
            if !summary.is_empty() {
                return Some(summary);
            }
        }
    }
    None
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

fn default_branch_name(
    mode: &str,
    parent_name: &str,
    source_preview: Option<&str>,
    created_at: &str,
) -> String {
    let summary = source_preview
        .map(summarize_branch_source)
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| summarize_branch_source(parent_name));
    let time = branch_time_label(created_at);
    match mode {
        "rewrite" => format!("改写 · {summary} · {time}"),
        "main" => format!("继续对话 · {time}"),
        _ => format!("分支 · {summary} · {time}"),
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

fn normalize_message_format(value: Option<&str>) -> String {
    match value.map(str::trim).filter(|item| !item.is_empty()) {
        Some("plain") => "plain".to_string(),
        Some("markdown") => "markdown".to_string(),
        Some("html") => "html".to_string(),
        Some("json") => "json".to_string(),
        Some("code") => "code".to_string(),
        Some("diagram") => "diagram".to_string(),
        _ => "markdown".to_string(),
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn resolve_default_sender(
    runtime_paths: &RuntimePaths,
    role: MessageRole,
    default_sender: &str,
) -> String {
    if role == MessageRole::Operator {
        resolve_operator_display_name(runtime_paths).unwrap_or_else(|| default_sender.to_string())
    } else {
        default_sender.to_string()
    }
}

fn normalize_operator_sender(
    runtime_paths: &RuntimePaths,
    role: MessageRole,
    sender: String,
    default_sender: &str,
) -> String {
    if role != MessageRole::Operator {
        return sender.trim().to_string();
    }

    let trimmed = sender.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case(default_sender)
        || trimmed.eq_ignore_ascii_case("operator")
        || trimmed.eq_ignore_ascii_case("user")
    {
        return resolve_default_sender(runtime_paths, role, default_sender);
    }

    trimmed.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    async fn test_state() -> ConversationServiceState {
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(":memory:")
                    .create_if_missing(true),
            )
            .await
            .expect("connect in-memory conversation db");
        initialize_conversation_schema(&pool)
            .await
            .expect("initialize conversation schema");
        let runtime_home =
            std::env::temp_dir().join(format!("ennoia-conversation-test-{}", Uuid::new_v4()));
        ConversationServiceState {
            store: ConversationStore::new(pool),
            runtime_paths: RuntimePaths::new(runtime_home),
        }
    }

    fn create_payload() -> CreateConversationPayload {
        CreateConversationPayload {
            topology: "direct".to_string(),
            title: Some("Test".to_string()),
            space_id: None,
            agent_ids: vec!["lsy".to_string()],
            lane_name: Some("Main".to_string()),
            lane_type: None,
            lane_goal: None,
        }
    }

    fn operator_message_payload(conversation_id: &str, body: &str) -> AppendMessageParams {
        AppendMessageParams {
            conversation_id: conversation_id.to_string(),
            message: ConversationMessagePayload {
                body: body.to_string(),
                role: Some("operator".to_string()),
                addressed_agents: vec!["lsy".to_string()],
                ..Default::default()
            },
        }
    }

    fn rewrite_payload(
        conversation_id: &str,
        parent_branch_id: &str,
        source_message_id: &str,
        body: &str,
    ) -> AppendMessageParams {
        AppendMessageParams {
            conversation_id: conversation_id.to_string(),
            message: ConversationMessagePayload {
                body: body.to_string(),
                role: Some("operator".to_string()),
                branch_id: Some(parent_branch_id.to_string()),
                lane_id: Some(parent_branch_id.to_string()),
                rewrite_from_message_id: Some(source_message_id.to_string()),
                addressed_agents: vec!["lsy".to_string()],
                ..Default::default()
            },
        }
    }

    fn agent_message_payload(
        conversation_id: &str,
        branch_id: &str,
        body: &str,
    ) -> AppendMessageParams {
        AppendMessageParams {
            conversation_id: conversation_id.to_string(),
            message: ConversationMessagePayload {
                body: body.to_string(),
                role: Some("agent".to_string()),
                sender: Some("lsy".to_string()),
                branch_id: Some(branch_id.to_string()),
                lane_id: Some(branch_id.to_string()),
                addressed_agents: vec!["operator".to_string()],
                ..Default::default()
            },
        }
    }

    fn html_agent_message_payload(
        conversation_id: &str,
        branch_id: &str,
        body: &str,
    ) -> AppendMessageParams {
        AppendMessageParams {
            conversation_id: conversation_id.to_string(),
            message: ConversationMessagePayload {
                body: body.to_string(),
                format: Some("html".to_string()),
                role: Some("agent".to_string()),
                sender: Some("lsy".to_string()),
                branch_id: Some(branch_id.to_string()),
                lane_id: Some(branch_id.to_string()),
                addressed_agents: vec!["operator".to_string()],
                ..Default::default()
            },
        }
    }

    #[tokio::test]
    async fn appends_html_message_format() {
        let state = test_state().await;
        let created = create_conversation(&state, create_payload())
            .await
            .expect("create conversation");
        let conversation_id = created.conversation.id.clone();
        let branch_id = created.default_lane.id.clone();

        let appended = append_routed_message(
            &state,
            html_agent_message_payload(
                &conversation_id,
                &branch_id,
                "<section><h2>Summary</h2></section>",
            ),
        )
        .await
        .expect("append html message");
        let messages = list_messages(
            &state,
            BranchLookupPayload {
                conversation_id,
                branch_id: None,
            },
        )
        .await
        .expect("list messages");

        assert_eq!(appended.message.format, "html");
        assert_eq!(messages[0].format, "html");
        assert_eq!(messages[0].body, "<section><h2>Summary</h2></section>");
    }

    #[tokio::test]
    async fn background_agent_message_does_not_steal_active_branch_from_new_rewrite() {
        let state = test_state().await;
        let created = create_conversation(&state, create_payload())
            .await
            .expect("create conversation");
        let conversation_id = created.conversation.id.clone();
        let root_branch_id = created.default_lane.id.clone();

        let first_operator =
            append_routed_message(&state, operator_message_payload(&conversation_id, "first"))
                .await
                .expect("append first operator message");
        let first_rewrite = append_routed_message(
            &state,
            rewrite_payload(
                &conversation_id,
                &root_branch_id,
                &first_operator.message.id,
                "rewrite one",
            ),
        )
        .await
        .expect("append first rewrite");
        let first_rewrite_branch_id = first_rewrite.branch.id.clone();
        let second_rewrite = append_routed_message(
            &state,
            rewrite_payload(
                &conversation_id,
                &first_rewrite_branch_id,
                &first_rewrite.message.id,
                "rewrite two",
            ),
        )
        .await
        .expect("append second rewrite");
        let second_rewrite_branch_id = second_rewrite.branch.id.clone();

        append_routed_message(
            &state,
            agent_message_payload(
                &conversation_id,
                &first_rewrite_branch_id,
                "late reply from first rewrite",
            ),
        )
        .await
        .expect("append late agent message");

        let refreshed = state
            .store
            .get_conversation(&conversation_id)
            .await
            .expect("load conversation")
            .expect("conversation exists");
        assert_eq!(
            refreshed.active_branch_id.as_deref(),
            Some(second_rewrite_branch_id.as_str())
        );
    }
}

fn resolve_operator_display_name(runtime_paths: &RuntimePaths) -> Option<String> {
    let contents = std::fs::read_to_string(runtime_paths.profile_config_file()).ok()?;
    let profile = toml::from_str::<RuntimeProfile>(&contents).ok()?;
    let trimmed = profile.display_name.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn branch_time_label(value: &str) -> String {
    DateTime::parse_from_rfc3339(value)
        .map(|item| item.format("%H:%M").to_string())
        .unwrap_or_else(|_| value.chars().skip(11).take(5).collect())
}

fn summarize_branch_source(value: &str) -> String {
    let normalized = value.split_whitespace().collect::<Vec<_>>().join(" ");
    if normalized.chars().count() <= 18 {
        return normalized;
    }
    normalized.chars().take(18).collect::<String>() + "…"
}

pub fn owner_kind_from(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}
