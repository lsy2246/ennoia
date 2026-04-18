use serde::{Deserialize, Serialize};

/// NextAction lists the possible outcomes of a runtime decision.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NextAction {
    StayPending,
    EnterPlanning,
    Dispatch,
    StayRunning,
    EnterBlocked,
    EnterReviewing,
    Complete,
    Fail,
    Cancel,
}

impl NextAction {
    pub fn as_str(self) -> &'static str {
        match self {
            NextAction::StayPending => "stay_pending",
            NextAction::EnterPlanning => "enter_planning",
            NextAction::Dispatch => "dispatch",
            NextAction::StayRunning => "stay_running",
            NextAction::EnterBlocked => "enter_blocked",
            NextAction::EnterReviewing => "enter_reviewing",
            NextAction::Complete => "complete",
            NextAction::Fail => "fail",
            NextAction::Cancel => "cancel",
        }
    }
}

/// Decision is the runtime engine's chosen next step, with its policy trail.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Decision {
    pub next_action: NextAction,
    pub policy_rule_id: String,
    pub reason: String,
}

/// DecisionSnapshot is the audit record stored for every engine invocation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DecisionSnapshot {
    pub id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub stage: String,
    pub signals_json: String,
    pub next_action: String,
    pub policy_rule_id: String,
    pub at: String,
}
