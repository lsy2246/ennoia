use std::fs;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use ennoia_kernel::{
    AgentPermissionPolicy, AgentPermissionRule, PermissionApprovalRecord, PermissionDecision,
    PermissionEventRecord, PermissionRequest,
};
use ennoia_observability::RequestContext;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

const PERMISSIONS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS permission_events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  event_id TEXT NOT NULL UNIQUE,
  agent_id TEXT NOT NULL,
  action TEXT NOT NULL,
  decision TEXT NOT NULL,
  target_json TEXT NOT NULL,
  scope_json TEXT NOT NULL,
  extension_id TEXT,
  matched_rule_id TEXT,
  approval_id TEXT,
  trace_id TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_permission_events_agent_time
  ON permission_events(agent_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_permission_events_decision_time
  ON permission_events(decision, created_at DESC);

CREATE TABLE IF NOT EXISTS permission_approvals (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  approval_id TEXT NOT NULL UNIQUE,
  status TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  action TEXT NOT NULL,
  target_json TEXT NOT NULL,
  scope_json TEXT NOT NULL,
  trigger_json TEXT NOT NULL,
  matched_rule_id TEXT,
  reason TEXT NOT NULL,
  created_at TEXT NOT NULL,
  resolved_at TEXT,
  resolution TEXT
);
CREATE INDEX IF NOT EXISTS idx_permission_approvals_status_time
  ON permission_approvals(status, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_permission_approvals_agent_time
  ON permission_approvals(agent_id, created_at DESC);

CREATE TABLE IF NOT EXISTS permission_grants (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  grant_id TEXT NOT NULL UNIQUE,
  approval_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  mode TEXT NOT NULL,
  request_json TEXT NOT NULL,
  consumed_at TEXT,
  expires_at TEXT,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_permission_grants_agent_time
  ON permission_grants(agent_id, created_at DESC);
"#;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionGrantRecord {
    grant_id: String,
    approval_id: String,
    agent_id: String,
    mode: String,
    request: PermissionRequest,
    consumed_at: Option<String>,
    expires_at: Option<String>,
    created_at: String,
}

#[derive(Debug, Clone)]
pub struct AgentPermissionStore {
    db_path: PathBuf,
    runtime_paths: RuntimePaths,
}

impl AgentPermissionStore {
    pub fn new(paths: &RuntimePaths) -> std::io::Result<Self> {
        if let Some(parent) = paths.permissions_db().parent() {
            fs::create_dir_all(parent)?;
        }
        fs::create_dir_all(paths.agent_policies_dir())?;
        let store = Self {
            db_path: paths.permissions_db(),
            runtime_paths: paths.clone(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn load_policy(&self, agent_id: &str) -> std::io::Result<AgentPermissionPolicy> {
        let path = self.runtime_paths.agent_policy_file(agent_id);
        if !path.exists() {
            return Ok(AgentPermissionPolicy::builtin_worker(agent_id));
        }
        let contents = fs::read_to_string(path)?;
        toml::from_str(&contents).map_err(std::io::Error::other)
    }

    pub fn save_policy(
        &self,
        agent_id: &str,
        policy: &AgentPermissionPolicy,
    ) -> std::io::Result<()> {
        if let Some(parent) = self.runtime_paths.agent_policy_file(agent_id).parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            self.runtime_paths.agent_policy_file(agent_id),
            toml::to_string_pretty(policy).map_err(std::io::Error::other)?,
        )
    }

    pub fn policy_summary(&self, agent_id: &str) -> std::io::Result<PermissionPolicySummary> {
        let policy = self.load_policy(agent_id)?;
        let mut allow_count = 0;
        let mut ask_count = 0;
        let mut deny_count = 0;
        for rule in &policy.rules {
            match rule.effect.trim().to_ascii_lowercase().as_str() {
                "allow" => allow_count += 1,
                "ask" => ask_count += 1,
                "deny" => deny_count += 1,
                _ => {}
            }
        }
        Ok(PermissionPolicySummary {
            agent_id: agent_id.to_string(),
            mode: policy.mode,
            allow_count,
            ask_count,
            deny_count,
        })
    }

    pub fn evaluate_request(
        &self,
        request: &PermissionRequest,
        trace: Option<&RequestContext>,
    ) -> std::io::Result<PermissionDecision> {
        if let Some(grant) = self.find_matching_grant(request)? {
            return Ok(PermissionDecision {
                decision: "allow".to_string(),
                matched_rule_id: Some(format!("grant:{}", grant.mode)),
                reason: format!("matched approval grant {}", grant.grant_id),
                approval_id: Some(grant.approval_id),
                grant_id: Some(grant.grant_id),
            });
        }

        let policy = self.load_policy(&request.agent_id)?;
        if let Some(rule) = policy
            .rules
            .iter()
            .find(|rule| rule_matches(rule, request))
            .cloned()
        {
            return self.decision_from_rule(request, rule, trace);
        }

        match policy.mode.trim().to_ascii_lowercase().as_str() {
            "default_allow" => {
                let decision = PermissionDecision {
                    decision: "allow".to_string(),
                    matched_rule_id: None,
                    reason: "policy mode default_allow".to_string(),
                    approval_id: None,
                    grant_id: None,
                };
                self.append_event(request, &decision, trace)?;
                Ok(decision)
            }
            _ => {
                let decision = PermissionDecision {
                    decision: "deny".to_string(),
                    matched_rule_id: None,
                    reason: "policy mode default_deny".to_string(),
                    approval_id: None,
                    grant_id: None,
                };
                self.append_event(request, &decision, trace)?;
                Ok(decision)
            }
        }
    }

    pub fn consume_grant(&self, grant_id: &str) -> std::io::Result<()> {
        self.open()?
            .execute(
                "UPDATE permission_grants SET consumed_at = ?2 WHERE grant_id = ?1 AND consumed_at IS NULL",
                params![grant_id, now_iso()],
            )
            .map(|_| ())
            .map_err(std::io::Error::other)
    }

    pub fn list_events(
        &self,
        query: &PermissionEventsQuery,
    ) -> std::io::Result<Vec<PermissionEventRecord>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT event_id, agent_id, action, decision, target_json, scope_json, extension_id, matched_rule_id, approval_id, trace_id, created_at
             FROM permission_events",
        );
        let mut filters = Vec::new();
        let mut params = Vec::<rusqlite::types::Value>::new();
        if let Some(agent_id) = &query.agent_id {
            filters.push("agent_id = ?");
            params.push(agent_id.clone().into());
        }
        if let Some(action) = &query.action {
            filters.push("action = ?");
            params.push(action.clone().into());
        }
        if let Some(decision) = &query.decision {
            filters.push("decision = ?");
            params.push(decision.clone().into());
        }
        if !filters.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&filters.join(" AND "));
        }
        sql.push_str(" ORDER BY seq DESC LIMIT ?");
        params.push((query.limit.max(1) as i64).into());
        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params), map_permission_event)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn list_approvals(
        &self,
        query: &PermissionApprovalsQuery,
    ) -> std::io::Result<Vec<PermissionApprovalRecord>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT approval_id, status, agent_id, action, target_json, scope_json, trigger_json, matched_rule_id, reason, created_at, resolved_at, resolution
             FROM permission_approvals",
        );
        let mut filters = Vec::new();
        let mut params = Vec::<rusqlite::types::Value>::new();
        if let Some(agent_id) = &query.agent_id {
            filters.push("agent_id = ?");
            params.push(agent_id.clone().into());
        }
        if let Some(status) = &query.status {
            filters.push("status = ?");
            params.push(status.clone().into());
        }
        if !filters.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&filters.join(" AND "));
        }
        sql.push_str(" ORDER BY seq DESC LIMIT ?");
        params.push((query.limit.max(1) as i64).into());
        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params), map_permission_approval)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn resolve_approval(
        &self,
        approval_id: &str,
        resolution: &str,
    ) -> std::io::Result<Option<PermissionApprovalRecord>> {
        let Some(mut approval) = self.get_approval(approval_id)? else {
            return Ok(None);
        };
        if approval.status != "pending" {
            return Ok(Some(approval));
        }

        let normalized_resolution = resolution.trim().to_ascii_lowercase();
        let resolved_at = now_iso();
        let status = if normalized_resolution == "deny" {
            "rejected"
        } else {
            "approved"
        };
        let connection = self.open()?;
        connection
            .execute(
                "UPDATE permission_approvals
                 SET status = ?2, resolution = ?3, resolved_at = ?4
                 WHERE approval_id = ?1",
                params![approval_id, status, normalized_resolution, resolved_at],
            )
            .map_err(std::io::Error::other)?;
        approval.status = status.to_string();
        approval.resolution = Some(normalized_resolution.clone());
        approval.resolved_at = Some(resolved_at);

        if status == "approved" {
            self.apply_approval_resolution(&approval, &normalized_resolution)?;
        }

        Ok(Some(approval))
    }

    fn decision_from_rule(
        &self,
        request: &PermissionRequest,
        rule: AgentPermissionRule,
        trace: Option<&RequestContext>,
    ) -> std::io::Result<PermissionDecision> {
        let effect = rule.effect.trim().to_ascii_lowercase();
        match effect.as_str() {
            "allow" => {
                let decision = PermissionDecision {
                    decision: "allow".to_string(),
                    matched_rule_id: Some(rule.id),
                    reason: "matched allow rule".to_string(),
                    approval_id: None,
                    grant_id: None,
                };
                self.append_event(request, &decision, trace)?;
                Ok(decision)
            }
            "ask" => {
                let approval = self.create_approval(request, Some(rule.id.clone()), trace)?;
                let decision = PermissionDecision {
                    decision: "ask".to_string(),
                    matched_rule_id: Some(rule.id),
                    reason: "matched ask rule".to_string(),
                    approval_id: Some(approval.approval_id.clone()),
                    grant_id: None,
                };
                self.append_event(request, &decision, trace)?;
                Ok(decision)
            }
            _ => {
                let decision = PermissionDecision {
                    decision: "deny".to_string(),
                    matched_rule_id: Some(rule.id),
                    reason: "matched deny rule".to_string(),
                    approval_id: None,
                    grant_id: None,
                };
                self.append_event(request, &decision, trace)?;
                Ok(decision)
            }
        }
    }

    fn apply_approval_resolution(
        &self,
        approval: &PermissionApprovalRecord,
        resolution: &str,
    ) -> std::io::Result<()> {
        match resolution {
            "allow_once" | "allow_conversation" | "allow_run" => {
                let mode = resolution.trim_start_matches("allow_");
                self.insert_grant(approval, mode)
            }
            "allow_policy" => self.append_policy_rule_from_approval(approval),
            _ => Ok(()),
        }
    }

    fn append_policy_rule_from_approval(
        &self,
        approval: &PermissionApprovalRecord,
    ) -> std::io::Result<()> {
        let mut policy = self.load_policy(&approval.agent_id)?;
        let mut rule = AgentPermissionRule {
            id: format!("approval-{}", Uuid::new_v4().simple()),
            effect: "allow".to_string(),
            actions: vec![approval.action.clone()],
            extension_scope: approval
                .scope
                .extension_id
                .clone()
                .into_iter()
                .collect::<Vec<_>>(),
            conversation_scope: approval
                .scope
                .conversation_id
                .as_ref()
                .map(|_| "current".to_string()),
            run_scope: approval
                .scope
                .run_id
                .as_ref()
                .map(|_| "current".to_string()),
            path_include: Vec::new(),
            path_exclude: Vec::new(),
            host_scope: Vec::new(),
        };
        if let Some(path) = approval
            .target
            .path
            .clone()
            .or_else(|| approval.scope.path.clone())
        {
            rule.path_include
                .push(ennoia_kernel::GlobPattern::new(path.replace('\\', "/")));
        }
        if let Some(host) = approval
            .target
            .host
            .clone()
            .or_else(|| approval.scope.host.clone())
        {
            rule.host_scope.push(ennoia_kernel::GlobPattern::new(host));
        }
        policy.rules.push(rule);
        self.save_policy(&approval.agent_id, &policy)
    }

    fn insert_grant(&self, approval: &PermissionApprovalRecord, mode: &str) -> std::io::Result<()> {
        let grant_id = format!("grant-{}", Uuid::new_v4());
        let request = PermissionRequest {
            agent_id: approval.agent_id.clone(),
            action: approval.action.clone(),
            target: approval.target.clone(),
            scope: approval.scope.clone(),
            trigger: approval.trigger.clone(),
        };
        let request_json = serde_json::to_string(&request).map_err(std::io::Error::other)?;
        let created_at = now_iso();
        let expires_at = if mode == "once" {
            Some((Utc::now() + Duration::minutes(10)).to_rfc3339())
        } else {
            None
        };
        self.open()?
            .execute(
                "INSERT INTO permission_grants
                 (grant_id, approval_id, agent_id, mode, request_json, consumed_at, expires_at, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7)",
                params![
                    grant_id,
                    approval.approval_id,
                    approval.agent_id,
                    mode,
                    request_json,
                    expires_at,
                    created_at,
                ],
            )
            .map(|_| ())
            .map_err(std::io::Error::other)
    }

    fn create_approval(
        &self,
        request: &PermissionRequest,
        matched_rule_id: Option<String>,
        _trace: Option<&RequestContext>,
    ) -> std::io::Result<PermissionApprovalRecord> {
        let approval = PermissionApprovalRecord {
            approval_id: format!("apr-{}", Uuid::new_v4()),
            status: "pending".to_string(),
            agent_id: request.agent_id.clone(),
            action: request.action.clone(),
            target: request.target.clone(),
            scope: request.scope.clone(),
            trigger: request.trigger.clone(),
            matched_rule_id,
            reason: format!("approval required for {}", request.action),
            created_at: now_iso(),
            resolved_at: None,
            resolution: None,
        };
        self.open()?
            .execute(
                "INSERT INTO permission_approvals
                 (approval_id, status, agent_id, action, target_json, scope_json, trigger_json, matched_rule_id, reason, created_at, resolved_at, resolution)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, NULL, NULL)",
                params![
                    approval.approval_id,
                    approval.status,
                    approval.agent_id,
                    approval.action,
                    serde_json::to_string(&approval.target).map_err(std::io::Error::other)?,
                    serde_json::to_string(&approval.scope).map_err(std::io::Error::other)?,
                    serde_json::to_string(&approval.trigger).map_err(std::io::Error::other)?,
                    approval.matched_rule_id,
                    approval.reason,
                    approval.created_at,
                ],
            )
            .map_err(std::io::Error::other)?;
        Ok(approval)
    }

    fn append_event(
        &self,
        request: &PermissionRequest,
        decision: &PermissionDecision,
        trace: Option<&RequestContext>,
    ) -> std::io::Result<()> {
        let event = PermissionEventRecord {
            event_id: format!("pev-{}", Uuid::new_v4()),
            agent_id: request.agent_id.clone(),
            action: request.action.clone(),
            decision: decision.decision.clone(),
            target: request.target.clone(),
            scope: request.scope.clone(),
            extension_id: request.scope.extension_id.clone(),
            matched_rule_id: decision.matched_rule_id.clone(),
            approval_id: decision.approval_id.clone(),
            trace_id: trace.map(|item| item.trace_id.clone()),
            created_at: now_iso(),
        };
        self.open()?
            .execute(
                "INSERT INTO permission_events
                 (event_id, agent_id, action, decision, target_json, scope_json, extension_id, matched_rule_id, approval_id, trace_id, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    event.event_id,
                    event.agent_id,
                    event.action,
                    event.decision,
                    serde_json::to_string(&event.target).map_err(std::io::Error::other)?,
                    serde_json::to_string(&event.scope).map_err(std::io::Error::other)?,
                    event.extension_id,
                    event.matched_rule_id,
                    event.approval_id,
                    event.trace_id,
                    event.created_at,
                ],
            )
            .map(|_| ())
            .map_err(std::io::Error::other)
    }

    fn find_matching_grant(
        &self,
        request: &PermissionRequest,
    ) -> std::io::Result<Option<PermissionGrantRecord>> {
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT grant_id, approval_id, agent_id, mode, request_json, consumed_at, expires_at, created_at
                 FROM permission_grants
                 WHERE agent_id = ?1
                 ORDER BY seq DESC",
            )
            .map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(params![request.agent_id], |row| {
                let request_json: String = row.get("request_json")?;
                let parsed =
                    serde_json::from_str::<PermissionRequest>(&request_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            request_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(PermissionGrantRecord {
                    grant_id: row.get("grant_id")?,
                    approval_id: row.get("approval_id")?,
                    agent_id: row.get("agent_id")?,
                    mode: row.get("mode")?,
                    request: parsed,
                    consumed_at: row.get("consumed_at")?,
                    expires_at: row.get("expires_at")?,
                    created_at: row.get("created_at")?,
                })
            })
            .map_err(std::io::Error::other)?;
        for row in rows {
            let grant = row.map_err(std::io::Error::other)?;
            if grant.consumed_at.is_some() {
                continue;
            }
            if let Some(expires_at) = &grant.expires_at {
                if expires_at.as_str() < now_iso().as_str() {
                    continue;
                }
            }
            if grant_matches(&grant, request) {
                return Ok(Some(grant));
            }
        }
        Ok(None)
    }

    fn get_approval(&self, approval_id: &str) -> std::io::Result<Option<PermissionApprovalRecord>> {
        self.open()?
            .query_row(
                "SELECT approval_id, status, agent_id, action, target_json, scope_json, trigger_json, matched_rule_id, reason, created_at, resolved_at, resolution
                 FROM permission_approvals
                 WHERE approval_id = ?1",
                params![approval_id],
                map_permission_approval,
            )
            .optional()
            .map_err(std::io::Error::other)
    }

    fn open(&self) -> std::io::Result<Connection> {
        let connection = Connection::open(&self.db_path).map_err(std::io::Error::other)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(std::io::Error::other)?;
        Ok(connection)
    }

    fn ensure_schema(&self) -> std::io::Result<()> {
        self.open()?
            .execute_batch(PERMISSIONS_SCHEMA_SQL)
            .map(|_| ())
            .map_err(std::io::Error::other)
    }
}

fn rule_matches(rule: &AgentPermissionRule, request: &PermissionRequest) -> bool {
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

fn grant_matches(grant: &PermissionGrantRecord, request: &PermissionRequest) -> bool {
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

fn map_permission_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionEventRecord> {
    let target_json: String = row.get("target_json")?;
    let scope_json: String = row.get("scope_json")?;
    Ok(PermissionEventRecord {
        event_id: row.get("event_id")?,
        agent_id: row.get("agent_id")?,
        action: row.get("action")?,
        decision: row.get("decision")?,
        target: serde_json::from_str(&target_json).unwrap_or_else(|_| {
            ennoia_kernel::PermissionTarget {
                kind: "unknown".to_string(),
                id: "unknown".to_string(),
                conversation_id: None,
                run_id: None,
                path: None,
                host: None,
            }
        }),
        scope: serde_json::from_str(&scope_json).unwrap_or_default(),
        extension_id: row.get("extension_id")?,
        matched_rule_id: row.get("matched_rule_id")?,
        approval_id: row.get("approval_id")?,
        trace_id: row.get("trace_id")?,
        created_at: row.get("created_at")?,
    })
}

fn map_permission_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionApprovalRecord> {
    let target_json: String = row.get("target_json")?;
    let scope_json: String = row.get("scope_json")?;
    let trigger_json: String = row.get("trigger_json")?;
    Ok(PermissionApprovalRecord {
        approval_id: row.get("approval_id")?,
        status: row.get("status")?,
        agent_id: row.get("agent_id")?,
        action: row.get("action")?,
        target: serde_json::from_str(&target_json).unwrap_or_else(|_| {
            ennoia_kernel::PermissionTarget {
                kind: "unknown".to_string(),
                id: "unknown".to_string(),
                conversation_id: None,
                run_id: None,
                path: None,
                host: None,
            }
        }),
        scope: serde_json::from_str(&scope_json).unwrap_or_default(),
        trigger: serde_json::from_str(&trigger_json).unwrap_or(ennoia_kernel::PermissionTrigger {
            kind: "unknown".to_string(),
            user_initiated: false,
        }),
        matched_rule_id: row.get("matched_rule_id")?,
        reason: row.get("reason")?,
        created_at: row.get("created_at")?,
        resolved_at: row.get("resolved_at")?,
        resolution: row.get("resolution")?,
    })
}

fn namespace(action: &str) -> &str {
    action.split('.').next().unwrap_or(action)
}

fn normalize_path(value: &str) -> String {
    value.replace('\\', "/")
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
