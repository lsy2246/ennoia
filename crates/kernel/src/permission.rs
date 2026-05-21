use serde::{Deserialize, Serialize};

use crate::policy::GlobPattern;

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
    pub target_scope: Vec<AgentPermissionCommandEntry>,
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
pub struct AgentPermissionCommandEntry {
    #[serde(rename = "match", default = "default_command_entry_match")]
    pub match_type: String,
    pub value: String,
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
    pub entries: Vec<AgentPermissionCommandEntry>,
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
        Self::default_profile()
    }
}

impl AgentPermissionProfile {
    pub fn default_profile() -> Self {
        Self {
            mode: default_permission_profile_mode(),
            entries: Vec::new(),
        }
    }

    pub fn compile_policy(&self, _agent_id: &str) -> AgentPermissionPolicy {
        let mode = normalize_permission_profile_mode(&self.mode);
        let normalized_entries = self
            .entries
            .iter()
            .map(|entry| {
                let match_type = normalize_command_entry_match_type(&entry.match_type);
                let value = if match_type == "regex" {
                    entry.value.trim().to_string()
                } else {
                    entry.value.trim().replace('\\', "/")
                };
                AgentPermissionCommandEntry { match_type, value }
            })
            .filter(|entry| !entry.value.is_empty())
            .collect::<Vec<_>>();
        let mut rules = Vec::new();

        match mode.as_str() {
            "blacklist" => {
                if !normalized_entries.is_empty() {
                    rules.push(ask_target_rule(
                        "runtime-command-blacklist-ask",
                        &["command.exec"],
                        normalized_entries,
                    ));
                }
                rules.push(allow_rule(
                    "runtime-command-default-allow",
                    &["command.exec"],
                ));
            }
            _ => {
                if !normalized_entries.is_empty() {
                    rules.push(allow_target_rule(
                        "runtime-command-whitelist-allow",
                        &["command.exec"],
                        normalized_entries,
                    ));
                }
                rules.push(ask_rule("runtime-command-default-ask", &["command.exec"]));
            }
        }
        rules.push(allow_rule("runtime-default-allow", &["*"]));
        AgentPermissionPolicy {
            mode: "default_ask".to_string(),
            rules,
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

fn default_command_entry_match() -> String {
    "prefix".to_string()
}

fn normalize_permission_profile_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "blacklist" => "blacklist".to_string(),
        _ => "whitelist".to_string(),
    }
}

fn normalize_command_entry_match_type(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "exact" => "exact".to_string(),
        "regex" => "regex".to_string(),
        _ => "prefix".to_string(),
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

fn allow_target_rule(
    id: &str,
    actions: &[&str],
    targets: Vec<AgentPermissionCommandEntry>,
) -> AgentPermissionRule {
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

fn ask_target_rule(
    id: &str,
    actions: &[&str],
    targets: Vec<AgentPermissionCommandEntry>,
) -> AgentPermissionRule {
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
            entries: vec![AgentPermissionCommandEntry {
                match_type: "prefix".to_string(),
                value: "git status".to_string(),
            }],
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_ask");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "runtime-command-whitelist-allow"));
    }

    #[test]
    fn whitelist_mode_without_entries_asks_for_command_exec() {
        let profile = AgentPermissionProfile {
            mode: "whitelist".to_string(),
            entries: Vec::new(),
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_ask");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "runtime-command-default-ask" && rule.effect == "ask"));
    }

    #[test]
    fn blacklist_mode_compiles_to_default_allow() {
        let profile = AgentPermissionProfile {
            mode: "blacklist".to_string(),
            entries: vec![AgentPermissionCommandEntry {
                match_type: "regex".to_string(),
                value: "^git\\s+(push|pull)(\\s|$)".to_string(),
            }],
        };
        let policy = profile.compile_policy("coder");
        assert_eq!(policy.mode, "default_ask");
        assert!(policy
            .rules
            .iter()
            .any(|rule| rule.id == "runtime-command-blacklist-ask"));
    }

    #[test]
    fn regex_entries_preserve_regex_escape_sequences() {
        let profile = AgentPermissionProfile {
            mode: "blacklist".to_string(),
            entries: vec![AgentPermissionCommandEntry {
                match_type: "regex".to_string(),
                value: String::from(r"^git\s+status$"),
            }],
        };
        let policy = profile.compile_policy("coder");
        let rule = policy
            .rules
            .iter()
            .find(|rule| rule.id == "runtime-command-blacklist-ask")
            .expect("compiled blacklist rule");
        assert_eq!(rule.target_scope[0].value, r"^git\s+status$");
    }

    #[test]
    fn sandbox_enabled_is_the_runtime_source_of_truth() {
        let environment = AgentExecutionEnvironment {
            sandbox_enabled: true,
        };
        assert!(environment.sandbox_enabled);
    }
}
