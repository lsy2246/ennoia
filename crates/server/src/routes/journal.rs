use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;

use ennoia_kernel::{
    ArtifactSpec, ConversationSpec, ConversationTopology, HookResourceRef, LaneSpec, MessageRole,
    MessageSpec, OwnerRef, RunSpec, TaskSpec, HOOK_EVENT_CONVERSATION_CREATED,
    HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};
use ennoia_paths::RuntimePaths;

use super::*;

const JOURNAL_DISABLED: &str = "journal_disabled";

#[derive(Debug, Deserialize)]
pub(super) struct CreateConversationPayload {
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
pub(super) struct ConversationCreateResponse {
    conversation: ConversationSpec,
    default_lane: LaneSpec,
}

#[derive(Debug, Serialize)]
pub(super) struct ConversationDetailResponse {
    conversation: ConversationSpec,
    lanes: Vec<LaneSpec>,
}

#[derive(Debug, Deserialize)]
pub(super) struct ConversationMessagePayload {
    body: String,
    #[serde(default)]
    lane_id: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    goal: Option<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(super) struct ConversationMessageResponse {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
    #[serde(default)]
    runs: Vec<RunSpec>,
    #[serde(default)]
    tasks: Vec<TaskSpec>,
    #[serde(default)]
    artifacts: Vec<ArtifactSpec>,
}

#[derive(Debug, Serialize)]
pub(super) struct JournalWorkspaceResponse {
    conversations: Vec<ConversationSpec>,
    message_count: usize,
}

#[derive(Debug, Serialize, Deserialize)]
struct JournalEvent {
    id: String,
    event: String,
    occurred_at: String,
    resource: HookResourceRef,
    payload: serde_json::Value,
}

struct PersistedOperatorMessage {
    conversation: ConversationSpec,
    lane: LaneSpec,
    message: MessageSpec,
}

pub(super) async fn conversations_list(
    State(state): State<AppState>,
) -> Result<Json<Vec<ConversationSpec>>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    store.list_conversations().map(Json).map_err(to_api_error)
}

pub(super) async fn journal_workspace(
    State(state): State<AppState>,
) -> Result<Json<JournalWorkspaceResponse>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    let conversations = store.list_conversations().map_err(to_api_error)?;
    let mut message_count = 0usize;
    for conversation in &conversations {
        message_count += store
            .list_messages(&conversation.id)
            .map(|items| items.len())
            .unwrap_or_default();
    }
    Ok(Json(JournalWorkspaceResponse {
        conversations,
        message_count,
    }))
}

pub(super) async fn conversations_create(
    State(state): State<AppState>,
    Json(payload): Json<CreateConversationPayload>,
) -> Result<Json<ConversationCreateResponse>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    let topology = topology_from_payload(&payload)?;
    let agent_ids = payload
        .agent_ids
        .into_iter()
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();
    if agent_ids.is_empty() {
        return Err(ApiError::bad_request("at least one agent is required"));
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
        owner: owner.clone(),
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
        conversation_id: conversation_id.clone(),
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
        updated_at: now.clone(),
    };

    store
        .upsert_conversation(&conversation)
        .map_err(to_api_error)?;
    store
        .write_lanes(&conversation.id, &[lane.clone()])
        .map_err(to_api_error)?;
    store
        .append_event(
            &conversation.id,
            &JournalEvent {
                id: format!("evt-{}", Uuid::new_v4()),
                event: HOOK_EVENT_CONVERSATION_CREATED.to_string(),
                occurred_at: now,
                resource: HookResourceRef {
                    kind: "conversation".to_string(),
                    id: conversation.id.clone(),
                    conversation_id: Some(conversation.id.clone()),
                    lane_id: None,
                    run_id: None,
                    task_id: None,
                    artifact_id: None,
                },
                payload: serde_json::to_value(&conversation)
                    .map_err(|error| ApiError::internal(error.to_string()))?,
            },
        )
        .map_err(to_api_error)?;

    Ok(Json(ConversationCreateResponse {
        conversation,
        default_lane: lane,
    }))
}

pub(super) async fn conversation_detail(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<ConversationDetailResponse>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    validate_segment(&conversation_id)?;
    let conversation = store
        .get_conversation(&conversation_id)
        .map_err(to_api_error)?
        .ok_or_else(|| ApiError::not_found("conversation not found"))?;
    let lanes = store.list_lanes(&conversation_id).map_err(to_api_error)?;
    Ok(Json(ConversationDetailResponse {
        conversation,
        lanes,
    }))
}

pub(super) async fn conversation_delete(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    validate_segment(&conversation_id)?;
    if store
        .delete_conversation(&conversation_id)
        .map_err(to_api_error)?
    {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found("conversation not found"))
    }
}

pub(super) async fn conversation_messages(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<MessageSpec>>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    validate_segment(&conversation_id)?;
    if store
        .get_conversation(&conversation_id)
        .map_err(to_api_error)?
        .is_none()
    {
        return Err(ApiError::not_found("conversation not found"));
    }
    store
        .list_messages(&conversation_id)
        .map(Json)
        .map_err(to_api_error)
}

pub(super) async fn conversation_lanes(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
) -> Result<Json<Vec<LaneSpec>>, ApiError> {
    let _guard = lock_journal(&state).await?;
    let store = JournalStore::new(state.runtime_paths.clone());
    validate_segment(&conversation_id)?;
    if store
        .get_conversation(&conversation_id)
        .map_err(to_api_error)?
        .is_none()
    {
        return Err(ApiError::not_found("conversation not found"));
    }
    store
        .list_lanes(&conversation_id)
        .map(Json)
        .map_err(to_api_error)
}

pub(super) async fn conversation_messages_create(
    State(state): State<AppState>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<ConversationMessagePayload>,
) -> Result<Json<ConversationMessageResponse>, ApiError> {
    let workflow_payload = {
        let _guard = lock_journal(&state).await?;
        let store = JournalStore::new(state.runtime_paths.clone());
        persist_operator_message(&store, &conversation_id, payload)?
    };

    Ok(Json(ConversationMessageResponse {
        conversation: workflow_payload.conversation,
        lane: workflow_payload.lane,
        message: workflow_payload.message,
        runs: Vec::new(),
        tasks: Vec::new(),
        artifacts: Vec::new(),
    }))
}

async fn lock_journal(state: &AppState) -> Result<tokio::sync::MutexGuard<'_, ()>, ApiError> {
    if !state.server_config.journal.enabled {
        return Err(ApiError::forbidden(JOURNAL_DISABLED));
    }
    Ok(state.journal_lock.lock().await)
}

fn persist_operator_message(
    store: &JournalStore,
    conversation_id: &str,
    payload: ConversationMessagePayload,
) -> Result<PersistedOperatorMessage, ApiError> {
    validate_segment(conversation_id)?;
    let conversation = store
        .get_conversation(conversation_id)
        .map_err(to_api_error)?
        .ok_or_else(|| ApiError::not_found("conversation not found"))?;
    let lanes = store.list_lanes(conversation_id).map_err(to_api_error)?;
    let lane = select_lane(&lanes, payload.lane_id.as_deref())
        .ok_or_else(|| ApiError::bad_request("lane not found"))?;
    let target_agents = resolve_addressed_agents(&conversation, &lane, payload.addressed_agents);
    if target_agents.is_empty() {
        return Err(ApiError::bad_request(
            "no addressed agents resolved for this message",
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
    store.append_message(&message).map_err(to_api_error)?;
    let conversation = ConversationSpec {
        updated_at: now.clone(),
        ..conversation
    };
    store
        .upsert_conversation(&conversation)
        .map_err(to_api_error)?;
    let lane = LaneSpec {
        updated_at: now.clone(),
        ..lane
    };
    let lanes = lanes
        .into_iter()
        .map(|item| {
            if item.id == lane.id {
                lane.clone()
            } else {
                item
            }
        })
        .collect::<Vec<_>>();
    store
        .write_lanes(conversation_id, &lanes)
        .map_err(to_api_error)?;
    store
        .append_event(
            conversation_id,
            &JournalEvent {
                id: format!("evt-{}", Uuid::new_v4()),
                event: HOOK_EVENT_CONVERSATION_MESSAGE_CREATED.to_string(),
                occurred_at: now,
                resource: HookResourceRef {
                    kind: "message".to_string(),
                    id: message.id.clone(),
                    conversation_id: Some(conversation.id.clone()),
                    lane_id: Some(lane.id.clone()),
                    run_id: None,
                    task_id: None,
                    artifact_id: None,
                },
                payload: serde_json::to_value(&message)
                    .map_err(|error| ApiError::internal(error.to_string()))?,
            },
        )
        .map_err(to_api_error)?;

    Ok(PersistedOperatorMessage {
        conversation,
        lane,
        message,
    })
}

struct JournalStore {
    paths: Arc<RuntimePaths>,
}

impl JournalStore {
    fn new(paths: Arc<RuntimePaths>) -> Self {
        Self { paths }
    }

    fn list_conversations(&self) -> std::io::Result<Vec<ConversationSpec>> {
        let path = self.index_file();
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }

    fn get_conversation(&self, conversation_id: &str) -> std::io::Result<Option<ConversationSpec>> {
        validate_segment_io(conversation_id)?;
        let path = self
            .conversation_dir(conversation_id)
            .join("conversation.json");
        if !path.exists() {
            return Ok(None);
        }
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents)
            .map(Some)
            .map_err(std::io::Error::other)
    }

    fn upsert_conversation(&self, conversation: &ConversationSpec) -> std::io::Result<()> {
        validate_segment_io(&conversation.id)?;
        let dir = self.conversation_dir(&conversation.id);
        fs::create_dir_all(&dir)?;
        write_json_atomic(&dir.join("conversation.json"), conversation)?;
        let mut conversations = self.list_conversations()?;
        if let Some(current) = conversations
            .iter_mut()
            .find(|item| item.id == conversation.id)
        {
            *current = conversation.clone();
        } else {
            conversations.push(conversation.clone());
        }
        conversations.sort_by(|left, right| right.updated_at.cmp(&left.updated_at));
        write_json_atomic(&self.index_file(), &conversations)
    }

    fn delete_conversation(&self, conversation_id: &str) -> std::io::Result<bool> {
        validate_segment_io(conversation_id)?;
        let mut conversations = self.list_conversations()?;
        let before = conversations.len();
        conversations.retain(|item| item.id != conversation_id);
        if before == conversations.len() {
            return Ok(false);
        }
        write_json_atomic(&self.index_file(), &conversations)?;
        let dir = self.conversation_dir(conversation_id);
        if dir.exists() {
            fs::remove_dir_all(dir)?;
        }
        Ok(true)
    }

    fn list_lanes(&self, conversation_id: &str) -> std::io::Result<Vec<LaneSpec>> {
        validate_segment_io(conversation_id)?;
        let path = self.conversation_dir(conversation_id).join("lanes.json");
        if !path.exists() {
            return Ok(Vec::new());
        }
        let contents = fs::read_to_string(path)?;
        serde_json::from_str(&contents).map_err(std::io::Error::other)
    }

    fn write_lanes(&self, conversation_id: &str, lanes: &[LaneSpec]) -> std::io::Result<()> {
        validate_segment_io(conversation_id)?;
        let dir = self.conversation_dir(conversation_id);
        fs::create_dir_all(&dir)?;
        write_json_atomic(&dir.join("lanes.json"), lanes)
    }

    fn list_messages(&self, conversation_id: &str) -> std::io::Result<Vec<MessageSpec>> {
        validate_segment_io(conversation_id)?;
        read_jsonl(
            &self
                .conversation_dir(conversation_id)
                .join("messages.jsonl"),
        )
    }

    fn append_message(&self, message: &MessageSpec) -> std::io::Result<()> {
        validate_segment_io(&message.conversation_id)?;
        append_jsonl(
            &self
                .conversation_dir(&message.conversation_id)
                .join("messages.jsonl"),
            message,
        )
    }

    fn append_event(&self, conversation_id: &str, event: &JournalEvent) -> std::io::Result<()> {
        validate_segment_io(conversation_id)?;
        append_jsonl(
            &self.conversation_dir(conversation_id).join("events.jsonl"),
            event,
        )
    }

    fn index_file(&self) -> PathBuf {
        self.paths.journal_index_dir().join("conversations.json")
    }

    fn conversation_dir(&self, conversation_id: &str) -> PathBuf {
        self.paths.journal_conversations_dir().join(conversation_id)
    }
}

fn write_json_atomic<T>(path: &StdPath, value: &T) -> std::io::Result<()>
where
    T: Serialize + ?Sized,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    let contents = serde_json::to_string_pretty(value).map_err(std::io::Error::other)?;
    fs::write(&tmp, contents)?;
    if path.exists() {
        fs::remove_file(path)?;
    }
    fs::rename(tmp, path)
}

fn append_jsonl<T>(path: &StdPath, value: &T) -> std::io::Result<()>
where
    T: Serialize,
{
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_string(value).map_err(std::io::Error::other)?;
    writeln!(file, "{line}")?;
    file.flush()
}

fn read_jsonl<T>(path: &StdPath) -> std::io::Result<Vec<T>>
where
    T: serde::de::DeserializeOwned,
{
    if !path.exists() {
        return Ok(Vec::new());
    }
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut items = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        items.push(serde_json::from_str(&line).map_err(std::io::Error::other)?);
    }
    Ok(items)
}

fn topology_from_payload(
    payload: &CreateConversationPayload,
) -> Result<ConversationTopology, ApiError> {
    let requested = match payload.topology.as_str() {
        "direct" => ConversationTopology::Direct,
        "group" => ConversationTopology::Group,
        _ => return Err(ApiError::bad_request("invalid conversation topology")),
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

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn validate_segment(value: &str) -> Result<(), ApiError> {
    validate_segment_io(value).map_err(|error| ApiError::bad_request(error.to_string()))
}

fn validate_segment_io(value: &str) -> std::io::Result<()> {
    let valid = !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.'));
    if valid {
        Ok(())
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "invalid journal id segment",
        ))
    }
}

fn to_api_error(error: std::io::Error) -> ApiError {
    ApiError::internal(error.to_string())
}
