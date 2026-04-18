use serde::{Deserialize, Serialize};

/// IntentSignals describe the incoming request shape.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct IntentSignals {
    pub trigger: String,
    pub mention_count: u32,
    pub goal_len: u32,
    pub has_question_mark: bool,
}

/// EvidenceSignals describe the state of recalled knowledge.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceSignals {
    pub recalled_memory_count: u32,
    pub source_count: u32,
    pub freshness_days: Option<u32>,
    pub local_evidence_sufficient: bool,
}

/// ExecutionSignals describe whether the run is ready to dispatch.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionSignals {
    pub plan_ready: bool,
    pub agent_available: bool,
    pub blocked: bool,
    pub blocked_reason: Option<String>,
}

/// Signals is the unified input for the runtime decision engine.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signals {
    pub intent: IntentSignals,
    pub evidence: EvidenceSignals,
    pub execution: ExecutionSignals,
}
