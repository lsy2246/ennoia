use ennoia_kernel::{NextAction, RunStage, Signals};
use serde::{Deserialize, Serialize};

/// StagePolicy holds the ordered rule list consulted by the runtime decision engine.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StagePolicy {
    pub rules: Vec<RuntimeRule>,
}

impl Default for StagePolicy {
    fn default() -> Self {
        Self::builtin()
    }
}

impl StagePolicy {
    pub fn builtin() -> Self {
        Self {
            rules: vec![
                RuntimeRule {
                    id: "R_BLOCKED".to_string(),
                    when: RuntimeRuleCondition {
                        execution_blocked: Some(true),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::EnterBlocked,
                        reason: "execution-blocked".to_string(),
                    },
                },
                RuntimeRule {
                    id: "R_PENDING_TO_PLANNING".to_string(),
                    when: RuntimeRuleCondition {
                        stage: Some(RunStage::Pending),
                        execution_blocked: Some(false),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::EnterPlanning,
                        reason: "trigger-received".to_string(),
                    },
                },
                RuntimeRule {
                    id: "R_PLANNING_TO_DISPATCH".to_string(),
                    when: RuntimeRuleCondition {
                        stage: Some(RunStage::Planning),
                        execution_plan_ready: Some(true),
                        execution_agent_available: Some(true),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::Dispatch,
                        reason: "plan-ready-agent-available".to_string(),
                    },
                },
                RuntimeRule {
                    id: "R_DISPATCHED_TO_RUNNING".to_string(),
                    when: RuntimeRuleCondition {
                        stage: Some(RunStage::Dispatched),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::StayRunning,
                        reason: "dispatch-acknowledged".to_string(),
                    },
                },
                RuntimeRule {
                    id: "R_RUNNING_TO_REVIEW".to_string(),
                    when: RuntimeRuleCondition {
                        stage: Some(RunStage::Running),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::EnterReviewing,
                        reason: "work-handed-off".to_string(),
                    },
                },
                RuntimeRule {
                    id: "R_REVIEW_TO_COMPLETE".to_string(),
                    when: RuntimeRuleCondition {
                        stage: Some(RunStage::Reviewing),
                        ..Default::default()
                    },
                    then: RuntimeRuleTarget {
                        next_action: NextAction::Complete,
                        reason: "review-passed".to_string(),
                    },
                },
            ],
        }
    }

    pub fn evaluate(&self, stage: RunStage, signals: &Signals) -> Option<&RuntimeRule> {
        self.rules
            .iter()
            .find(|rule| rule.when.matches(stage, signals))
    }
}

/// RuntimeRule is one entry in the stage/decision policy.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRule {
    pub id: String,
    pub when: RuntimeRuleCondition,
    pub then: RuntimeRuleTarget,
}

/// RuntimeRuleCondition describes what must hold for a rule to fire.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRuleCondition {
    pub stage: Option<RunStage>,
    pub execution_plan_ready: Option<bool>,
    pub execution_agent_available: Option<bool>,
    pub execution_blocked: Option<bool>,
    pub evidence_sufficient: Option<bool>,
}

impl RuntimeRuleCondition {
    pub fn matches(&self, stage: RunStage, signals: &Signals) -> bool {
        if let Some(required) = self.stage {
            if required != stage {
                return false;
            }
        }
        if let Some(required) = self.execution_plan_ready {
            if required != signals.execution.plan_ready {
                return false;
            }
        }
        if let Some(required) = self.execution_agent_available {
            if required != signals.execution.agent_available {
                return false;
            }
        }
        if let Some(required) = self.execution_blocked {
            if required != signals.execution.blocked {
                return false;
            }
        }
        if let Some(required) = self.evidence_sufficient {
            if required != signals.evidence.local_evidence_sufficient {
                return false;
            }
        }
        true
    }
}

/// RuntimeRuleTarget is the outcome when a rule fires.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRuleTarget {
    pub next_action: NextAction,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::{ExecutionSignals, Signals};

    #[test]
    fn pending_without_block_enters_planning() {
        let policy = StagePolicy::builtin();
        let signals = Signals::default();
        let rule = policy.evaluate(RunStage::Pending, &signals).unwrap();
        assert_eq!(rule.id, "R_PENDING_TO_PLANNING");
    }

    #[test]
    fn blocked_execution_wins_over_stage() {
        let policy = StagePolicy::builtin();
        let signals = Signals {
            execution: ExecutionSignals {
                blocked: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let rule = policy.evaluate(RunStage::Pending, &signals).unwrap();
        assert_eq!(rule.id, "R_BLOCKED");
    }

    #[test]
    fn planning_with_ready_plan_dispatches() {
        let policy = StagePolicy::builtin();
        let signals = Signals {
            execution: ExecutionSignals {
                plan_ready: true,
                agent_available: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let rule = policy.evaluate(RunStage::Planning, &signals).unwrap();
        assert_eq!(rule.id, "R_PLANNING_TO_DISPATCH");
    }
}
