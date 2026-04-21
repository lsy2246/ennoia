use serde::{Deserialize, Serialize};

/// OwnerKind distinguishes whether a resource belongs globally, to an agent, or to a space.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum OwnerKind {
    Global,
    Agent,
    Space,
}

/// OwnerRef is the shared owner envelope used across runs, artifacts, jobs and memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct OwnerRef {
    pub kind: OwnerKind,
    pub id: String,
}

impl OwnerRef {
    pub fn new(kind: OwnerKind, id: impl Into<String>) -> Self {
        Self {
            kind,
            id: id.into(),
        }
    }

    pub fn global(id: impl Into<String>) -> Self {
        Self::new(OwnerKind::Global, id)
    }

    pub fn agent(id: impl Into<String>) -> Self {
        Self::new(OwnerKind::Agent, id)
    }

    pub fn space(id: impl Into<String>) -> Self {
        Self::new(OwnerKind::Space, id)
    }
}

/// RuntimeProfile is the single local operator profile for this runtime instance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeProfile {
    pub id: String,
    pub display_name: String,
    pub locale: String,
    pub time_zone: String,
    pub default_space_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// ConversationTopology describes whether a conversation is one-to-one or multi-agent.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConversationTopology {
    Direct,
    Group,
}

/// ParticipantType distinguishes the operator from agents.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantType {
    Operator,
    Agent,
}

/// ParticipantRef is the stable envelope for a conversation participant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct ParticipantRef {
    pub kind: ParticipantType,
    pub id: String,
}

/// MessageRole describes who produced one message.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    Operator,
    Agent,
    System,
    Tool,
}

/// TaskStatus tracks each planned task state.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

/// TaskKind tracks the purpose of one task unit.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    Response,
    Collaboration,
    Maintenance,
    Workflow,
}

/// ArtifactKind distinguishes stored output types.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactKind {
    Screenshot,
    Har,
    Report,
    Export,
    Log,
    Summary,
    Handoff,
}

/// AgentSpec is the runtime representation of an agent participant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSpec {
    pub id: String,
    pub display_name: String,
    pub role_kind: String,
    pub default_model: String,
}

/// SpaceSpec describes one project/work container.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpaceSpec {
    pub id: String,
    pub display_name: String,
    pub description: String,
    pub primary_goal: String,
    pub mention_policy: String,
    pub default_agents: Vec<String>,
}

/// ConversationSpec describes one direct or group conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConversationSpec {
    pub id: String,
    pub topology: ConversationTopology,
    pub owner: OwnerRef,
    pub space_id: Option<String>,
    pub title: String,
    pub participants: Vec<String>,
    pub default_lane_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// LaneSpec is one work line inside a conversation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LaneSpec {
    pub id: String,
    pub conversation_id: String,
    pub space_id: Option<String>,
    pub name: String,
    pub lane_type: String,
    pub status: String,
    pub goal: String,
    pub participants: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MessageSpec is the normalized message input shape for orchestrator flows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageSpec {
    pub id: String,
    pub conversation_id: String,
    pub lane_id: Option<String>,
    pub sender: String,
    pub role: MessageRole,
    pub body: String,
    pub mentions: Vec<String>,
    pub created_at: String,
}

/// RunSpec captures the base run metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunSpec {
    pub id: String,
    pub owner: OwnerRef,
    pub conversation_id: String,
    pub lane_id: Option<String>,
    pub trigger: String,
    pub stage: crate::stage::RunStage,
    pub goal: String,
    pub created_at: String,
    pub updated_at: String,
}

/// TaskSpec captures one execution unit inside a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSpec {
    pub id: String,
    pub run_id: String,
    pub conversation_id: String,
    pub lane_id: Option<String>,
    pub task_kind: TaskKind,
    pub title: String,
    pub assigned_agent_id: String,
    pub status: TaskStatus,
    pub created_at: String,
    pub updated_at: String,
}

/// ArtifactSpec stores the minimum metadata needed to locate a produced artifact.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtifactSpec {
    pub id: String,
    pub owner: OwnerRef,
    pub run_id: String,
    pub conversation_id: Option<String>,
    pub lane_id: Option<String>,
    pub kind: ArtifactKind,
    pub relative_path: String,
    pub created_at: String,
}

/// HandoffSpec describes one cross-lane transfer package.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HandoffSpec {
    pub id: String,
    pub from_lane_id: String,
    pub to_lane_id: String,
    pub from_agent_id: Option<String>,
    pub to_agent_id: Option<String>,
    pub summary: String,
    pub instructions: String,
    pub status: String,
    pub created_at: String,
}
