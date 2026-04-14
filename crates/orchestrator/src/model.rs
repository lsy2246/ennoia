use ennoia_kernel::{OwnerRef, RunSpec, TaskSpec, ThreadSpec};
use ennoia_memory::ContextView;
use serde::{Deserialize, Serialize};

/// RunTrigger explains what started a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum RunTrigger {
    DirectMessage,
    SpaceMessage,
    ScheduledJob,
}

/// RunRequest is the input shape used to build a planned run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRequest {
    pub owner: OwnerRef,
    pub thread: ThreadSpec,
    pub trigger: RunTrigger,
    pub goal: String,
    pub addressed_agents: Vec<String>,
}

/// PlannedRun is the normalized result emitted by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlannedRun {
    pub run: RunSpec,
    pub tasks: Vec<TaskSpec>,
    pub context: ContextView,
}
