use serde::{Deserialize, Serialize};

use crate::policy::GlobPattern;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityPermissionMetadata {
    pub action: String,
    pub target_kind: String,
    #[serde(default)]
    pub risk_level: String,
    #[serde(default)]
    pub default_decision: String,
    #[serde(default)]
    pub scope_kind: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionTarget {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionScope {
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub host: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionTrigger {
    pub kind: String,
    #[serde(default)]
    pub user_initiated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionRequest {
    pub agent_id: String,
    pub action: String,
    pub target: PermissionTarget,
    #[serde(default)]
    pub scope: PermissionScope,
    pub trigger: PermissionTrigger,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionDecision {
    pub decision: String,
    #[serde(default)]
    pub matched_rule_id: Option<String>,
    pub reason: String,
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub grant_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPermissionRule {
    pub id: String,
    pub effect: String,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub extension_scope: Vec<String>,
    #[serde(default)]
    pub conversation_scope: Option<String>,
    #[serde(default)]
    pub run_scope: Option<String>,
    #[serde(default)]
    pub path_include: Vec<GlobPattern>,
    #[serde(default)]
    pub path_exclude: Vec<GlobPattern>,
    #[serde(default)]
    pub host_scope: Vec<GlobPattern>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPermissionPolicy {
    #[serde(default = "default_policy_mode")]
    pub mode: String,
    #[serde(default)]
    pub rules: Vec<AgentPermissionRule>,
}

impl Default for AgentPermissionPolicy {
    fn default() -> Self {
        Self {
            mode: default_policy_mode(),
            rules: Vec::new(),
        }
    }
}

impl AgentPermissionPolicy {
    pub fn builtin_worker(agent_id: &str) -> Self {
        Self {
            mode: default_policy_mode(),
            rules: vec![
                AgentPermissionRule {
                    id: "builtin-core-chat".to_string(),
                    effect: "allow".to_string(),
                    actions: vec![
                        "provider.generate".to_string(),
                        "conversation.read".to_string(),
                        "conversation.write".to_string(),
                        "conversation.branch.create".to_string(),
                        "conversation.branch.switch".to_string(),
                        "memory.read".to_string(),
                        "memory.write".to_string(),
                        "memory.review".to_string(),
                        "run.create".to_string(),
                        "run.read".to_string(),
                        "artifact.read".to_string(),
                        "artifact.write".to_string(),
                    ],
                    extension_scope: Vec::new(),
                    conversation_scope: None,
                    run_scope: None,
                    path_include: Vec::new(),
                    path_exclude: Vec::new(),
                    host_scope: Vec::new(),
                },
                AgentPermissionRule {
                    id: "builtin-agent-workdir".to_string(),
                    effect: "allow".to_string(),
                    actions: vec!["fs.read".to_string(), "fs.write".to_string()],
                    extension_scope: Vec::new(),
                    conversation_scope: None,
                    run_scope: None,
                    path_include: vec![GlobPattern::new(format!(
                        "~/.ennoia/agents/{agent_id}/work/**"
                    ))],
                    path_exclude: Vec::new(),
                    host_scope: Vec::new(),
                },
                AgentPermissionRule {
                    id: "builtin-agent-artifacts".to_string(),
                    effect: "allow".to_string(),
                    actions: vec![
                        "artifact.read".to_string(),
                        "artifact.write".to_string(),
                        "fs.read".to_string(),
                    ],
                    extension_scope: Vec::new(),
                    conversation_scope: None,
                    run_scope: None,
                    path_include: vec![GlobPattern::new(format!(
                        "~/.ennoia/agents/{agent_id}/artifacts/**"
                    ))],
                    path_exclude: Vec::new(),
                    host_scope: Vec::new(),
                },
                AgentPermissionRule {
                    id: "builtin-dangerous-ask".to_string(),
                    effect: "ask".to_string(),
                    actions: vec![
                        "net.fetch".to_string(),
                        "command.exec".to_string(),
                        "runtime.config.write".to_string(),
                        "extension.install".to_string(),
                        "extension.enable".to_string(),
                        "extension.disable".to_string(),
                    ],
                    extension_scope: Vec::new(),
                    conversation_scope: None,
                    run_scope: None,
                    path_include: Vec::new(),
                    path_exclude: Vec::new(),
                    host_scope: vec![GlobPattern::new("**")],
                },
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionApprovalRecord {
    pub approval_id: String,
    pub status: String,
    pub agent_id: String,
    pub action: String,
    pub target: PermissionTarget,
    pub scope: PermissionScope,
    pub trigger: PermissionTrigger,
    #[serde(default)]
    pub matched_rule_id: Option<String>,
    pub reason: String,
    pub created_at: String,
    #[serde(default)]
    pub resolved_at: Option<String>,
    #[serde(default)]
    pub resolution: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionEventRecord {
    pub event_id: String,
    pub agent_id: String,
    pub action: String,
    pub decision: String,
    pub target: PermissionTarget,
    pub scope: PermissionScope,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub matched_rule_id: Option<String>,
    #[serde(default)]
    pub approval_id: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
    pub created_at: String,
}

fn default_policy_mode() -> String {
    "default_deny".to_string()
}
