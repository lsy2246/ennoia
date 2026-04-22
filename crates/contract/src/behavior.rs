use ennoia_kernel::{
    ArtifactSpec, Decision, GateVerdict, HandoffSpec, OwnerRef, RunContext, RunSpec, RunStageEvent,
    TaskSpec,
};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BehaviorTrigger {
    Manual,
    Message,
    Handoff,
    Schedule,
    External,
}

impl BehaviorTrigger {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::Message => "message",
            Self::Handoff => "handoff",
            Self::Schedule => "schedule",
            Self::External => "external",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehaviorSourceRef {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorRunRequest {
    pub owner: OwnerRef,
    pub goal: String,
    pub trigger: BehaviorTrigger,
    #[serde(default)]
    pub participants: Vec<String>,
    #[serde(default)]
    pub addressed_agents: Vec<String>,
    #[serde(default)]
    pub context: RunContext,
    #[serde(default)]
    pub source_refs: Vec<BehaviorSourceRef>,
    #[serde(default)]
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorRunResponse {
    pub run: RunSpec,
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactSpec>,
    #[serde(default)]
    pub handoffs: Vec<HandoffSpec>,
    #[serde(default)]
    pub stage_events: Vec<RunStageEvent>,
    pub decision: Decision,
    #[serde(default)]
    pub gate_verdicts: Vec<GateVerdict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BehaviorRunDetailResponse {
    pub run: RunSpec,
    #[serde(default)]
    pub tasks: Vec<TaskSpec>,
    #[serde(default)]
    pub artifacts: Vec<ArtifactSpec>,
    #[serde(default)]
    pub handoffs: Vec<HandoffSpec>,
    #[serde(default)]
    pub stage_events: Vec<RunStageEvent>,
    #[serde(default)]
    pub gate_verdicts: Vec<GateVerdict>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehaviorStatusResponse {
    pub extension_id: String,
    pub behavior_id: String,
    pub healthy: bool,
    pub version: String,
    #[serde(default)]
    pub interfaces: Vec<String>,
}
