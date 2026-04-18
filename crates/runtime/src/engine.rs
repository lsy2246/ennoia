use std::sync::Arc;

use ennoia_kernel::{Decision, DecisionEngine, RunStage, Signals, StagePolicy, StageMachine};

use crate::stage::PolicyStageMachine;

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
