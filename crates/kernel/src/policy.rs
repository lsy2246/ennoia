//! Policy shapes (MemoryPolicy, StagePolicy, GlobPattern) consulted at runtime.
//!
//! The kernel only owns the *data* of a policy. Loading it from disk is the job
//! of the `ennoia-policy` crate (toml → these types).

use serde::{Deserialize, Serialize};

use crate::{NextAction, RunStage, Signals};

// ========== GlobPattern ==========

/// GlobPattern is a minimal prefix/star matcher for namespaces and paths.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GlobPattern(String);

impl GlobPattern {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self(pattern.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn matches(&self, value: &str) -> bool {
        match_pattern(&self.0, value)
    }
}

fn match_pattern(pattern: &str, value: &str) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }
    if pattern == "*" || pattern == "**" {
        return true;
    }

    let pattern_parts: Vec<&str> = pattern.split('/').collect();
    let value_parts: Vec<&str> = value.split('/').collect();

    match_parts(&pattern_parts, &value_parts)
}

fn match_parts(pattern: &[&str], value: &[&str]) -> bool {
    if pattern.is_empty() {
        return value.is_empty();
    }

    let head = pattern[0];
    let rest = &pattern[1..];

    if head == "**" {
        if rest.is_empty() {
            return true;
        }
        for i in 0..=value.len() {
            if match_parts(rest, &value[i..]) {
                return true;
            }
        }
        return false;
    }

    if value.is_empty() {
        return false;
    }

    if head == "*" || head == value[0] {
        return match_parts(rest, &value[1..]);
    }

    false
}

// ========== MemoryPolicy ==========

/// MemoryPolicy controls what may be written where.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryPolicy {
    #[serde(default)]
    pub truth_namespaces: Vec<GlobPattern>,
    #[serde(default = "default_true")]
    pub require_sources_for_long_term: bool,
    #[serde(default)]
    pub forbidden_namespaces: Vec<GlobPattern>,
    #[serde(default = "default_assemble_budget")]
    pub assemble_budget_chars: u32,
}

impl Default for MemoryPolicy {
    fn default() -> Self {
        Self::builtin()
    }
}

impl MemoryPolicy {
    pub fn builtin() -> Self {
        Self {
            truth_namespaces: vec![
                GlobPattern::new("user/**"),
                GlobPattern::new("agents/**"),
                GlobPattern::new("interaction/**"),
                GlobPattern::new("work/**"),
            ],
            require_sources_for_long_term: true,
            forbidden_namespaces: vec![
                GlobPattern::new("conversation/**"),
                GlobPattern::new("tmp/**"),
            ],
            assemble_budget_chars: 4000,
        }
    }

    pub fn is_truth_namespace(&self, namespace: &str) -> bool {
        self.truth_namespaces.iter().any(|g| g.matches(namespace))
    }

    pub fn is_forbidden(&self, namespace: &str) -> bool {
        self.forbidden_namespaces
            .iter()
            .any(|g| g.matches(namespace))
    }
}

fn default_true() -> bool {
    true
}

fn default_assemble_budget() -> u32 {
    4000
}

// ========== StagePolicy ==========

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRule {
    pub id: String,
    pub when: RuntimeRuleCondition,
    pub then: RuntimeRuleTarget,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeRuleTarget {
    pub next_action: NextAction,
    pub reason: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ExecutionSignals, Signals};

    #[test]
    fn star_matches_single_segment() {
        let pattern = GlobPattern::new("user/*");
        assert!(pattern.matches("user/profile"));
        assert!(!pattern.matches("user/profile/deep"));
        assert!(!pattern.matches("agents/coder"));
    }

    #[test]
    fn double_star_matches_any_depth() {
        let pattern = GlobPattern::new("agents/**");
        assert!(pattern.matches("agents/coder"));
        assert!(pattern.matches("agents/coder/skills"));
    }

    #[test]
    fn builtin_memory_policy_recognizes_user_truth() {
        let policy = MemoryPolicy::builtin();
        assert!(policy.is_truth_namespace("user/profile"));
        assert!(!policy.is_truth_namespace("conversation/scratch"));
    }

    #[test]
    fn pending_without_block_enters_planning() {
        let policy = StagePolicy::builtin();
        let rule = policy
            .evaluate(RunStage::Pending, &Signals::default())
            .unwrap();
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
