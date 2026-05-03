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
    pub message_id: Option<String>,
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
    pub target_scope: Vec<GlobPattern>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentPermissionProfile {
    #[serde(default = "default_permission_profile_mode")]
    pub mode: String,
    #[serde(default)]
    pub command_rules: Vec<GlobPattern>,
    #[serde(default)]
    pub path_rules: Vec<GlobPattern>,
}

impl Default for AgentPermissionPolicy {
    fn default() -> Self {
        Self {
            mode: default_policy_mode(),
            rules: Vec::new(),
        }
    }
}

impl Default for AgentPermissionProfile {
    fn default() -> Self {
        Self::builtin_worker()
    }
}

impl AgentPermissionProfile {
    pub fn builtin_worker() -> Self {
        Self {
            mode: default_permission_profile_mode(),
            command_rules: Vec::new(),
            path_rules: Vec::new(),
        }
    }

    pub fn compile_policy(&self, _agent_id: &str) -> AgentPermissionPolicy {
        let mode = normalize_permission_profile_mode(&self.mode);
        let normalized_command_rules = self
            .command_rules
            .iter()
            .map(|pattern| GlobPattern::new(pattern.as_str().trim().replace('\\', "/")))
            .collect::<Vec<_>>();
        let normalized_path_rules = self
            .path_rules
            .iter()
            .map(|pattern| GlobPattern::new(pattern.as_str().trim().replace('\\', "/")))
            .collect::<Vec<_>>();
        let mut rules = vec![allow_rule(
            "builtin-core-chat",
            &[
                "provider.generate",
                "conversation.read",
                "conversation.write",
                "conversation.branch.create",
                "conversation.branch.switch",
                "memory.read",
                "memory.write",
                "memory.review",
                "run.create",
                "run.read",
                "artifact.read",
                "artifact.write",
            ],
        )];

        if normalized_path_rules.is_empty() {
            if mode == "blacklist" {
                rules.push(allow_rule("builtin-files-allow", &["fs.read", "fs.write"]));
            } else {
                rules.push(ask_rule("builtin-path-ask-fs", &["fs.read", "fs.write"]));
            }
        } else {
            rules.push(allow_path_rule(
                "builtin-path-rules-fs",
                &["fs.read", "fs.write"],
                normalized_path_rules.clone(),
            ));
            rules.push(ask_rule("builtin-path-ask-fs", &["fs.read", "fs.write"]));
        }

        match mode.as_str() {
            "blacklist" => {
                if !normalized_command_rules.is_empty() {
                    rules.push(ask_target_rule(
                        "builtin-command-blacklist-ask",
                        &["command.exec"],
                        normalized_command_rules,
                    ));
                }
                if !normalized_path_rules.is_empty() {
                    rules.push(allow_path_rule(
                        "builtin-command-path-allow",
                        &["command.exec"],
                        normalized_path_rules,
                    ));
                    rules.push(ask_rule("builtin-command-path-ask", &["command.exec"]));
                }
                AgentPermissionPolicy {
                    mode: "default_allow".to_string(),
                    rules,
                }
            }
            _ => {
                if !normalized_command_rules.is_empty() {
                    if normalized_path_rules.is_empty() {
                        rules.push(allow_target_rule(
                            "builtin-command-whitelist-allow",
                            &["command.exec"],
                            normalized_command_rules,
                        ));
                    } else {
                        rules.push(allow_target_path_rule(
                            "builtin-command-whitelist-allow",
                            &["command.exec"],
                            normalized_command_rules,
                            normalized_path_rules,
                        ));
                    }
                }
                AgentPermissionPolicy {
                    mode: "default_ask".to_string(),
                    rules,
                }
            }
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
    pub expires_at: Option<String>,
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
    "default_ask".to_string()
}

fn default_permission_profile_mode() -> String {
    "whitelist".to_string()
}

fn default_execution_environment_sandbox_enabled() -> bool {
    false
}

fn normalize_permission_profile_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "blacklist" => "blacklist".to_string(),
        _ => "whitelist".to_string(),
    }
}

fn allow_rule(id: &str, actions: &[&str]) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: Vec::new(),
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: Vec::new(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn ask_rule(id: &str, actions: &[&str]) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "ask".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: Vec::new(),
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: Vec::new(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn allow_path_rule(id: &str, actions: &[&str], paths: Vec<GlobPattern>) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: Vec::new(),
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: paths,
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn allow_target_rule(id: &str, actions: &[&str], targets: Vec<GlobPattern>) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: targets,
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: Vec::new(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn ask_target_rule(id: &str, actions: &[&str], targets: Vec<GlobPattern>) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "ask".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: targets,
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: Vec::new(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn allow_target_path_rule(
    id: &str,
    actions: &[&str],
    targets: Vec<GlobPattern>,
    paths: Vec<GlobPattern>,
) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        target_scope: targets,
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: paths,
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentExecutionEnvironment {
    #[serde(default = "default_execution_environment_sandbox_enabled")]
    pub sandbox_enabled: bool,
}

impl Default for AgentExecutionEnvironment {
    fn default() -> Self {
        Self {
            sandbox_enabled: default_execution_environment_sandbox_enabled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn whitelist_mode_compiles_to_default_ask() {
        let profile = AgentPermissionProfile {
            mode: "whitelist".to_string(),
            command_rules: vec![GlobPattern::new("git")],
            path_rules: vec![GlobPattern::new("/workspace/project/**")],
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_ask");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "builtin-command-whitelist-allow"));
    }

    #[test]
    fn whitelist_mode_without_path_rules_asks_for_fs() {
        let profile = AgentPermissionProfile {
            mode: "whitelist".to_string(),
            command_rules: Vec::new(),
            path_rules: Vec::new(),
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_ask");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "builtin-path-ask-fs" && rule.effect == "ask"));
    }

    #[test]
    fn blacklist_mode_compiles_to_default_allow() {
        let profile = AgentPermissionProfile {
            mode: "blacklist".to_string(),
            command_rules: vec![GlobPattern::new("powershell")],
            path_rules: vec![GlobPattern::new("D:/data/code/**")],
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_allow");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "builtin-command-blacklist-ask"));
    }

    #[test]
    fn sandbox_enabled_is_the_runtime_source_of_truth() {
        let environment = AgentExecutionEnvironment {
            sandbox_enabled: true,
        };
        assert!(environment.sandbox_enabled);
    }
}
