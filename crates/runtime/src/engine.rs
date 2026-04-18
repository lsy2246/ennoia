use std::sync::Arc;

use ennoia_kernel::{Decision, RunStage, Signals};
use ennoia_policy::StagePolicy;

use crate::stage::{PolicyStageMachine, StageMachine};

/// DecisionEngine produces a Decision for a given stage + signals.
pub trait DecisionEngine: Send + Sync {
    fn decide(&self, stage: RunStage, signals: &Signals) -> Decision;
}

/// DefaultDecisionEngine delegates to a PolicyStageMachine and emits its Decision part.
#[derive(Debug, Clone)]
pub struct DefaultDecisionEngine {
    machine: PolicyStageMachine,
}

impl DefaultDecisionEngine {
    pub fn new(policy: Arc<StagePolicy>) -> Self {
        Self {
            machine: PolicyStageMachine::new(policy),
        }
    }
}

impl DecisionEngine for DefaultDecisionEngine {
    fn decide(&self, stage: RunStage, signals: &Signals) -> Decision {
        self.machine.decide(stage, signals).0
    }
}
