use serde::{Deserialize, Serialize};

/// RunStage is the canonical lifecycle for a Run.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum RunStage {
    Pending,
    Planning,
    Dispatched,
    Running,
    Blocked,
    Reviewing,
    Completed,
    Failed,
    Cancelled,
}

impl RunStage {
    pub fn as_str(self) -> &'static str {
        match self {
            RunStage::Pending => "pending",
            RunStage::Planning => "planning",
            RunStage::Dispatched => "dispatched",
            RunStage::Running => "running",
            RunStage::Blocked => "blocked",
            RunStage::Reviewing => "reviewing",
            RunStage::Completed => "completed",
            RunStage::Failed => "failed",
            RunStage::Cancelled => "cancelled",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "planning" => RunStage::Planning,
            "dispatched" => RunStage::Dispatched,
            "running" => RunStage::Running,
            "blocked" => RunStage::Blocked,
            "reviewing" => RunStage::Reviewing,
            "completed" => RunStage::Completed,
            "failed" => RunStage::Failed,
            "cancelled" => RunStage::Cancelled,
            _ => RunStage::Pending,
        }
    }

    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            RunStage::Completed | RunStage::Failed | RunStage::Cancelled
        )
    }
}

/// StageTransition describes one proposed stage change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StageTransition {
    pub from: RunStage,
    pub to: RunStage,
    pub policy_rule_id: String,
    pub reason: String,
}

/// RunStageEvent is the audit record of an actual stage change.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunStageEvent {
    pub id: String,
    pub run_id: String,
    pub from_stage: Option<RunStage>,
    pub to_stage: RunStage,
    pub policy_rule_id: Option<String>,
    pub reason: Option<String>,
    pub at: String,
}
