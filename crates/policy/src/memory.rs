use serde::{Deserialize, Serialize};

use crate::glob::GlobPattern;

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
            forbidden_namespaces: vec![GlobPattern::new("session/**"), GlobPattern::new("tmp/**")],
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

#[cfg(test)]
mod tests {
    use super::MemoryPolicy;

    #[test]
    fn builtin_policy_recognizes_user_truth() {
        let policy = MemoryPolicy::builtin();
        assert!(policy.is_truth_namespace("user/profile"));
        assert!(policy.is_truth_namespace("agents/coder/truth"));
        assert!(!policy.is_truth_namespace("session/scratch"));
    }

    #[test]
    fn builtin_policy_blocks_forbidden_namespace() {
        let policy = MemoryPolicy::builtin();
        assert!(policy.is_forbidden("session/transient"));
        assert!(!policy.is_forbidden("user/profile"));
    }
}
