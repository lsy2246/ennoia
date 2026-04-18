use ennoia_kernel::{
    ContextView, Decision, DecisionSnapshot, GateRecord, GateVerdict, MessageSpec, OwnerRef,
    RunSpec, RunStageEvent, Signals, TaskSpec, ThreadSpec,
};
use serde::{Deserialize, Serialize};

/// RunTrigger explains what started a run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RunTrigger {
    DirectMessage,
    SpaceMessage,
    ScheduledJob,
}

impl RunTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            RunTrigger::DirectMessage => "direct_message",
            RunTrigger::SpaceMessage => "space_message",
            RunTrigger::ScheduledJob => "scheduled_job",
        }
    }
}

/// RunRequest is the input shape used to build a planned run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRequest {
    pub owner: OwnerRef,
    pub thread: ThreadSpec,
    pub message: MessageSpec,
    pub trigger: RunTrigger,
    pub goal: String,
    pub addressed_agents: Vec<String>,
}

/// PlannedRun is the normalized result emitted by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedRun {
    pub thread: ThreadSpec,
    pub message: MessageSpec,
    pub run: RunSpec,
    pub tasks: Vec<TaskSpec>,
    pub context: ContextView,
    pub signals: Signals,
    pub decision: Decision,
    pub stage_event: RunStageEvent,
    pub gate_verdicts: Vec<GateVerdict>,
    pub gate_records: Vec<GateRecord>,
    pub decision_snapshot: DecisionSnapshot,
}
