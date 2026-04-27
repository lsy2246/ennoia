mod sqlite;

use chrono::Utc;
use ennoia_kernel::{AgentPermissionRule, PermissionRequest};
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

#[derive(Debug, Clone)]
pub(super) struct PermissionGrantRecord {
    pub grant_id: String,
    pub approval_id: String,
    pub agent_id: String,
    pub mode: String,
    pub request: PermissionRequest,
    pub consumed_at: Option<String>,
    pub expires_at: Option<String>,
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
    if grant.agent_id != request.agent_id || grant.request.action != request.action {
        return false;
    }
    match grant.mode.as_str() {
        "once" => {
            grant.request.target == request.target
                && grant.request.scope == request.scope
                && grant.request.trigger == request.trigger
        }
        "conversation" => {
            grant.request.scope.conversation_id.is_some()
                && grant.request.scope.conversation_id == request.scope.conversation_id
        }
        "run" => {
            grant.request.scope.run_id.is_some()
                && grant.request.scope.run_id == request.scope.run_id
        }
        _ => false,
    }
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
