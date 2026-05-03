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
    pub path_whitelist: Vec<GlobPattern>,
    #[serde(default)]
    pub allow_command_exec: bool,
    #[serde(default)]
    pub allow_external_network: bool,
    #[serde(default)]
    pub allow_runtime_config_write: bool,
    #[serde(default)]
    pub allow_extension_manage: bool,
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
            path_whitelist: Vec::new(),
            allow_command_exec: false,
            allow_external_network: false,
            allow_runtime_config_write: false,
            allow_extension_manage: false,
        }
    }

    pub fn compile_policy(&self, agent_id: &str) -> AgentPermissionPolicy {
        let mode = normalize_permission_profile_mode(&self.mode);
        let mut rules = vec![
            allow_rule(
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
            ),
            allow_path_rule(
                "builtin-agent-workdir",
                &["fs.read", "fs.write"],
                &[format!("**/.ennoia/agents/{agent_id}/work/**")],
            ),
            allow_path_rule(
                "builtin-agent-artifacts",
                &["artifact.read", "artifact.write", "fs.read", "fs.write"],
                &[format!("**/.ennoia/agents/{agent_id}/artifacts/**")],
            ),
        ];

        if self.path_whitelist.is_empty() {
            rules.push(allow_rule("builtin-files-global", &["fs.read", "fs.write"]));
        } else {
            rules.push(AgentPermissionRule {
                id: "builtin-path-whitelist".to_string(),
                effect: "allow".to_string(),
                actions: vec!["fs.read".to_string(), "fs.write".to_string()],
                extension_scope: Vec::new(),
                conversation_scope: None,
                run_scope: None,
                path_include: self
                    .path_whitelist
                    .iter()
                    .map(|pattern| GlobPattern::new(pattern.as_str().replace('\\', "/")))
                    .collect(),
                path_exclude: Vec::new(),
                host_scope: Vec::new(),
            });
        }

        match mode.as_str() {
            "trusted" => {
                if !self.path_whitelist.is_empty() {
                    rules.push(ask_rule("builtin-files-ask", &["fs.read", "fs.write"]));
                }
                if !self.allow_external_network {
                    rules.push(ask_rule("builtin-network-ask", &["net.fetch"]));
                }
                if !self.allow_command_exec {
                    rules.push(ask_rule("builtin-command-ask", &["command.exec"]));
                }
                if !self.allow_runtime_config_write {
                    rules.push(ask_rule(
                        "builtin-runtime-config-ask",
                        &["runtime.config.write"],
                    ));
                }
                if !self.allow_extension_manage {
                    rules.push(ask_rule(
                        "builtin-extension-manage-ask",
                        &["extension.install", "extension.enable", "extension.disable"],
                    ));
                }
                AgentPermissionPolicy {
                    mode: "default_allow".to_string(),
                    rules,
                }
            }
            _ => {
                rules.push(if self.allow_external_network {
                    allow_rule("builtin-network-allow", &["net.fetch"])
                } else {
                    ask_rule("builtin-network-ask", &["net.fetch"])
                });
                rules.push(if self.allow_command_exec {
                    allow_rule("builtin-command-allow", &["command.exec"])
                } else {
                    ask_rule("builtin-command-ask", &["command.exec"])
                });
                rules.push(if self.allow_runtime_config_write {
                    allow_rule("builtin-runtime-config-allow", &["runtime.config.write"])
                } else {
                    ask_rule("builtin-runtime-config-ask", &["runtime.config.write"])
                });
                rules.push(if self.allow_extension_manage {
                    allow_rule(
                        "builtin-extension-manage-allow",
                        &["extension.install", "extension.enable", "extension.disable"],
                    )
                } else {
                    ask_rule(
                        "builtin-extension-manage-ask",
                        &["extension.install", "extension.enable", "extension.disable"],
                    )
                });
                rules.push(ask_rule("builtin-catch-all-ask", &["*"]));
                AgentPermissionPolicy {
                    mode: "default_deny".to_string(),
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
    "default_deny".to_string()
}

fn default_permission_profile_mode() -> String {
    "restricted".to_string()
}

fn default_execution_environment_mode() -> String {
    "host".to_string()
}

fn normalize_permission_profile_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "trusted" => "trusted".to_string(),
        _ => "restricted".to_string(),
    }
}

fn normalize_execution_environment_mode(value: &str) -> String {
    match value.trim().to_ascii_lowercase().as_str() {
        "native" => "native".to_string(),
        _ => "host".to_string(),
    }
}

fn allow_rule(id: &str, actions: &[&str]) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
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
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: Vec::new(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

fn allow_path_rule(id: &str, actions: &[&str], paths: &[String]) -> AgentPermissionRule {
    AgentPermissionRule {
        id: id.to_string(),
        effect: "allow".to_string(),
        actions: actions.iter().map(|item| (*item).to_string()).collect(),
        extension_scope: Vec::new(),
        conversation_scope: None,
        run_scope: None,
        path_include: paths.iter().cloned().map(GlobPattern::new).collect(),
        path_exclude: Vec::new(),
        host_scope: Vec::new(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentExecutionEnvironment {
    #[serde(default = "default_execution_environment_mode")]
    pub mode: String,
}

impl Default for AgentExecutionEnvironment {
    fn default() -> Self {
        Self {
            mode: default_execution_environment_mode(),
        }
    }
}

impl AgentExecutionEnvironment {
    pub fn normalized_mode(&self) -> String {
        normalize_execution_environment_mode(&self.mode)
    }

    pub fn is_native(&self) -> bool {
        self.normalized_mode() == "native"
    }
}
