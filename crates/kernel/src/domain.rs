use serde::{Deserialize, Serialize};

/// OwnerKind distinguishes whether a resource belongs globally, to an agent, or to a space.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OwnerKind {
    Global,
    Agent,
    Space,
}

/// OwnerRef is the shared owner envelope used across runs, artifacts, jobs and memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnerRef {
    pub kind: OwnerKind,
    pub id: String,
}

/// ThreadKind describes the conversation topology.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThreadKind {
    Private,
    Space,
}

/// MessageRole describes who produced one message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MessageRole {
    User,
    Agent,
    System,
}

/// RunStatus tracks high-level run execution state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunStatus {
    Pending,
    Running,
    Blocked,
    Completed,
}

/// TaskStatus tracks each planned task state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskStatus {
    Pending,
    Running,
    Failed,
    Completed,
}

/// TaskKind tracks the purpose of one task unit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TaskKind {
    Response,
    Collaboration,
    Maintenance,
}

/// ArtifactKind distinguishes stored output types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ArtifactKind {
    Screenshot,
    Har,
    Report,
    Export,
    Log,
}

/// AgentSpec is the runtime representation of an agent participant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSpec {
    pub id: String,
    pub display_name: String,
    pub default_model: String,
}

/// SpaceSpec describes a collaboration space.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SpaceSpec {
    pub id: String,
    pub display_name: String,
    pub mention_policy: String,
    pub default_agents: Vec<String>,
}

/// ThreadSpec describes one private or space thread.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThreadSpec {
    pub id: String,
    pub kind: ThreadKind,
    pub owner: OwnerRef,
    pub space_id: Option<String>,
    pub title: String,
    pub participants: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// MessageSpec is the normalized message input shape for orchestrator flows.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MessageSpec {
    pub id: String,
    pub thread_id: String,
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
    pub thread_id: String,
    pub trigger: String,
    pub status: RunStatus,
    pub goal: String,
    pub created_at: String,
    pub updated_at: String,
}

/// TaskSpec captures one execution unit inside a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskSpec {
    pub id: String,
    pub run_id: String,
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
    pub kind: ArtifactKind,
    pub relative_path: String,
    pub created_at: String,
}
