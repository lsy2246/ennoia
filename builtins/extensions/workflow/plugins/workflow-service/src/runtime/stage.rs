use std::sync::Arc;

use super::StageMachine;
use ennoia_kernel::{Decision, NextAction, RunStage, Signals, StagePolicy, StageTransition};

/// PolicyStageMachine drives transitions from the declarative StagePolicy.
#[derive(Debug, Clone)]
pub struct PolicyStageMachine {
    policy: Arc<StagePolicy>,
}

impl PolicyStageMachine {
    pub fn new(policy: Arc<StagePolicy>) -> Self {
        Self { policy }
    }
}

impl StageMachine for PolicyStageMachine {
    fn decide(&self, stage: RunStage, signals: &Signals) -> (Decision, StageTransition) {
        match self.policy.evaluate(stage, signals) {
            Some(rule) => {
                let next_stage = apply_next_action(stage, rule.then.next_action);
                let decision = Decision {
                    next_action: rule.then.next_action,
                    policy_rule_id: rule.id.clone(),
                    reason: rule.then.reason.clone(),
                };
                let transition = StageTransition {
                    from: stage,
                    to: next_stage,
                    policy_rule_id: rule.id.clone(),
                    reason: rule.then.reason.clone(),
                };
                (decision, transition)
            }
            None => {
                let decision = Decision {
                    next_action: NextAction::StayPending,
                    policy_rule_id: "R_DEFAULT_STAY".to_string(),
                    reason: "no-rule-matched".to_string(),
                };
                let transition = StageTransition {
                    from: stage,
                    to: stage,
                    policy_rule_id: "R_DEFAULT_STAY".to_string(),
                    reason: "no-rule-matched".to_string(),
                };
                (decision, transition)
            }
        }
    }
}

/// apply_next_action maps a NextAction onto the resulting RunStage.
pub fn apply_next_action(_from: RunStage, action: NextAction) -> RunStage {
    match action {
        NextAction::StayPending => RunStage::Pending,
        NextAction::EnterPlanning => RunStage::Planning,
        NextAction::Dispatch => RunStage::Dispatched,
        NextAction::StayRunning => RunStage::Running,
        NextAction::EnterBlocked => RunStage::Blocked,
        NextAction::EnterReviewing => RunStage::Reviewing,
        NextAction::Complete => RunStage::Completed,
        NextAction::Fail => RunStage::Failed,
        NextAction::Cancel => RunStage::Cancelled,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::{ExecutionSignals, Signals};

    #[test]
    fn policy_machine_enters_planning() {
        let machine = PolicyStageMachine::new(Arc::new(StagePolicy::builtin()));
        let (decision, transition) = machine.decide(RunStage::Pending, &Signals::default());
        assert_eq!(decision.next_action, NextAction::EnterPlanning);
        assert_eq!(transition.to, RunStage::Planning);
    }

    #[test]
    fn policy_machine_dispatches_when_ready() {
        let machine = PolicyStageMachine::new(Arc::new(StagePolicy::builtin()));
        let signals = Signals {
            execution: ExecutionSignals {
                plan_ready: true,
                agent_available: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let (decision, transition) = machine.decide(RunStage::Planning, &signals);
        assert_eq!(decision.next_action, NextAction::Dispatch);
        assert_eq!(transition.to, RunStage::Dispatched);
    }
}
