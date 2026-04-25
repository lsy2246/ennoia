use ennoia_contract::behavior::BehaviorTrigger;
use ennoia_kernel::{
    Decision, DecisionSnapshot, GateRecord, GateVerdict, OwnerRef, RunContext, RunSpec,
    RunStageEvent, Signals, TaskSpec,
};
use serde::{Deserialize, Serialize};

/// RunRequest is the input shape used to build a planned run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunRequest {
    pub owner: OwnerRef,
    pub conversation_id: String,
    #[serde(default)]
    pub lane_id: Option<String>,
    pub trigger: BehaviorTrigger,
    pub goal: String,
    #[serde(default)]
    pub requested_model_id: Option<String>,
    #[serde(default)]
    pub requested_max_turns: Option<u32>,
    #[serde(default)]
    pub participants: Vec<String>,
    pub addressed_agents: Vec<String>,
}

/// PlannedRun is the normalized result emitted by the orchestrator.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlannedRun {
    pub run: RunSpec,
    pub tasks: Vec<TaskSpec>,
    pub context: RunContext,
    pub signals: Signals,
    pub decision: Decision,
    pub stage_event: RunStageEvent,
    pub gate_verdicts: Vec<GateVerdict>,
    pub gate_records: Vec<GateRecord>,
    pub decision_snapshot: DecisionSnapshot,
}
