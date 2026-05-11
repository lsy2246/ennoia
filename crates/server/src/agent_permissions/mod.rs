mod sqlite;

use chrono::Utc;
use ennoia_kernel::{AgentPermissionCommandEntry, AgentPermissionRule, PermissionRequest};
use regex::Regex;
use serde::{Deserialize, Serialize};

pub use sqlite::AgentPermissionStore;

#[derive(Debug, Clone, Default)]
pub struct PermissionEventsQuery {
    pub agent_id: Option<String>,
    pub action: Option<String>,
    pub decision: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionApprovalsQuery {
    pub agent_id: Option<String>,
    pub conversation_id: Option<String>,
    pub status: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Default)]
pub struct PermissionGrantsQuery {
    pub agent_id: Option<String>,
    pub conversation_id: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionPolicySummary {
    pub agent_id: String,
    pub mode: String,
    pub allow_count: usize,
    pub ask_count: usize,
    pub deny_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApprovalResolutionPayload {
    pub resolution: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PermissionGrantRecord {
    pub grant_id: String,
    pub approval_id: String,
    pub agent_id: String,
    pub mode: String,
    pub request: PermissionRequest,
    pub consumed_at: Option<String>,
    pub expires_at: Option<String>,
    pub revoked_at: Option<String>,
}

pub(super) fn rule_matches(rule: &AgentPermissionRule, request: &PermissionRequest) -> bool {
    if !rule.actions.is_empty()
        && !rule.actions.iter().any(|action| {
            action == &request.action
                || action == "*"
                || action.as_str() == format!("{}.*", namespace(&request.action))
        })
    {
        return false;
    }
    if !rule.extension_scope.is_empty() {
        let Some(extension_id) = &request.scope.extension_id else {
            return false;
        };
        if !rule.extension_scope.iter().any(|item| item == extension_id) {
            return false;
        }
    }
    if !matches_target_scope(rule, request) {
        return false;
    }
    if !matches_conversation_scope(rule.conversation_scope.as_deref(), request) {
        return false;
    }
    if !matches_run_scope(rule.run_scope.as_deref(), request) {
        return false;
    }
    if !matches_path_scope(rule, request) {
        return false;
    }
    if !matches_host_scope(rule, request) {
        return false;
    }
    true
}

fn matches_target_scope(rule: &AgentPermissionRule, request: &PermissionRequest) -> bool {
    if rule.target_scope.is_empty() {
        return true;
    }
    rule.target_scope
        .iter()
        .any(|entry| matches_command_entry(entry, request.target.id.as_str()))
}

fn matches_command_entry(entry: &AgentPermissionCommandEntry, candidate: &str) -> bool {
    match entry.match_type.trim().to_ascii_lowercase().as_str() {
        "exact" => {
            normalize_command_match_value(candidate) == normalize_command_match_value(&entry.value)
        }
        "regex" => Regex::new(entry.value.trim())
            .map(|pattern| pattern.is_match(&normalize_command_match_value(candidate)))
            .unwrap_or(false),
        _ => normalize_command_match_value(candidate)
            .starts_with(&normalize_command_match_value(&entry.value)),
    }
}

fn normalize_command_match_value(value: &str) -> String {
    value.trim().replace('\\', "/")
}

fn matches_conversation_scope(scope: Option<&str>, request: &PermissionRequest) -> bool {
    match scope.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "" | "any" => true,
        "current" | "same_conversation" => {
            let Some(current) = &request.scope.conversation_id else {
                return false;
            };
            request
                .target
                .conversation_id
                .as_ref()
                .or(Some(&request.target.id))
                .map(|target| target == current)
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn matches_run_scope(scope: Option<&str>, request: &PermissionRequest) -> bool {
    match scope.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "" | "any" => true,
        "current" | "same_run" => {
            let Some(current) = &request.scope.run_id else {
                return false;
            };
            request
                .target
                .run_id
                .as_ref()
                .or(Some(&request.target.id))
                .map(|target| target == current)
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn matches_path_scope(rule: &AgentPermissionRule, request: &PermissionRequest) -> bool {
    let candidate = request
        .target
        .path
        .as_deref()
        .or(request.scope.path.as_deref())
        .map(normalize_path);
    if !rule.path_include.is_empty() {
        let Some(candidate) = candidate.as_deref() else {
            return false;
        };
        if !rule
            .path_include
            .iter()
            .any(|pattern| pattern.matches(candidate))
        {
            return false;
        }
    }
    if !rule.path_exclude.is_empty() {
        let Some(candidate) = candidate.as_deref() else {
            return true;
        };
        if rule
            .path_exclude
            .iter()
            .any(|pattern| pattern.matches(candidate))
        {
            return false;
        }
    }
    true
}

fn matches_host_scope(rule: &AgentPermissionRule, request: &PermissionRequest) -> bool {
    if rule.host_scope.is_empty() {
        return true;
    }
    let candidate = request
        .target
        .host
        .as_deref()
        .or(request.scope.host.as_deref())
        .unwrap_or_default();
    rule.host_scope
        .iter()
        .any(|pattern| pattern.matches(candidate))
}

pub(super) fn grant_matches(grant: &PermissionGrantRecord, request: &PermissionRequest) -> bool {
    if grant.agent_id != request.agent_id {
        return false;
    }
    match grant.mode.as_str() {
        "once" => {
            if grant.request.action != request.action {
                return false;
            }
            same_target_without_run(&grant.request.target, &request.target)
                && same_scope_without_run(&grant.request.scope, &request.scope)
        }
        "reply_action" => {
            if grant.request.action != request.action {
                return false;
            }
            grant.request.scope.conversation_id.is_some()
                && grant.request.scope.conversation_id == request.scope.conversation_id
                && grant.request.scope.message_id.is_some()
                && grant.request.scope.message_id == request.scope.message_id
                && grant.request.target.kind == request.target.kind
                && grant.request.target.id == request.target.id
        }
        "conversation_all" => {
            grant.request.scope.conversation_id.is_some()
                && grant.request.scope.conversation_id == request.scope.conversation_id
        }
        _ => false,
    }
}

fn same_target_without_run(
    left: &ennoia_kernel::PermissionTarget,
    right: &ennoia_kernel::PermissionTarget,
) -> bool {
    left.kind == right.kind
        && left.id == right.id
        && left.conversation_id == right.conversation_id
        && left.path == right.path
        && left.host == right.host
}

fn same_scope_without_run(
    left: &ennoia_kernel::PermissionScope,
    right: &ennoia_kernel::PermissionScope,
) -> bool {
    left.conversation_id == right.conversation_id
        && left.message_id == right.message_id
        && left.extension_id == right.extension_id
        && left.path == right.path
        && left.host == right.host
}

pub(super) fn namespace(action: &str) -> &str {
    action.split('.').next().unwrap_or(action)
}

pub(super) fn normalize_path(value: &str) -> String {
    value.replace('\\', "/")
}

pub(super) fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

pub(super) fn is_expired_iso(value: &str) -> bool {
    chrono::DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc) <= Utc::now())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::{grant_matches, matches_command_entry, PermissionGrantRecord};
    use ennoia_kernel::{
        AgentPermissionCommandEntry, PermissionRequest, PermissionScope, PermissionTarget,
        PermissionTrigger,
    };

    fn request(run_id: &str) -> PermissionRequest {
        PermissionRequest {
            agent_id: "a".to_string(),
            action: "fs.read".to_string(),
            target: PermissionTarget {
                kind: "file".to_string(),
                id: "/workspace/missing.txt".to_string(),
                conversation_id: Some("conv-1".to_string()),
                run_id: Some(run_id.to_string()),
                path: Some("/workspace/missing.txt".to_string()),
                host: None,
            },
            scope: PermissionScope {
                conversation_id: Some("conv-1".to_string()),
                run_id: Some(run_id.to_string()),
                message_id: Some("msg-1".to_string()),
                extension_id: Some("builtin".to_string()),
                path: Some("/workspace/missing.txt".to_string()),
                host: None,
            },
            trigger: PermissionTrigger {
                kind: "pipeline.workflow_to_conversation".to_string(),
                user_initiated: true,
            },
        }
    }

    fn grant(mode: &str, request: PermissionRequest) -> PermissionGrantRecord {
        PermissionGrantRecord {
            grant_id: "grant-1".to_string(),
            approval_id: "apr-1".to_string(),
            agent_id: "a".to_string(),
            mode: mode.to_string(),
            request,
            consumed_at: None,
            expires_at: None,
            revoked_at: None,
        }
    }

    #[test]
    fn once_grant_matches_resumed_request_with_new_run_id() {
        let original = request("run-1");
        let resumed = request("run-2");
        let approval_grant = grant("once", original);
        assert!(grant_matches(&approval_grant, &resumed));
    }

    #[test]
    fn once_grant_does_not_match_different_message() {
        let original = request("run-1");
        let mut resumed = request("run-2");
        resumed.scope.message_id = Some("msg-2".to_string());
        let approval_grant = grant("once", original);
        assert!(!grant_matches(&approval_grant, &resumed));
    }

    #[test]
    fn command_entry_exact_match_uses_normalized_invocation() {
        let entry = AgentPermissionCommandEntry {
            match_type: "exact".to_string(),
            value: r#"node C:\tools\runner.mjs"#.to_string(),
        };
        assert!(matches_command_entry(&entry, "node C:/tools/runner.mjs"));
        assert!(!matches_command_entry(
            &entry,
            "node C:/tools/runner.mjs --watch"
        ));
    }

    #[test]
    fn command_entry_prefix_match_supports_global_commands() {
        let entry = AgentPermissionCommandEntry {
            match_type: "prefix".to_string(),
            value: "git".to_string(),
        };
        assert!(matches_command_entry(&entry, "git status"));
        assert!(matches_command_entry(&entry, "git diff --cached"));
        assert!(!matches_command_entry(&entry, "node git-status.js"));
    }

    #[test]
    fn command_entry_regex_match_supports_advanced_patterns() {
        let entry = AgentPermissionCommandEntry {
            match_type: "regex".to_string(),
            value: String::from(r"^git\s+(status|diff)(\s|$)"),
        };
        assert!(matches_command_entry(&entry, "git status"));
        assert!(matches_command_entry(&entry, "git diff --cached"));
        assert!(!matches_command_entry(&entry, "git push origin main"));
    }

    #[test]
    fn invalid_regex_entry_does_not_match() {
        let entry = AgentPermissionCommandEntry {
            match_type: "regex".to_string(),
            value: "(".to_string(),
        };
        assert!(!matches_command_entry(&entry, "git status"));
    }
}
