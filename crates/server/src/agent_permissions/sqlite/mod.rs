mod query;
mod schema;

use std::fs;
use std::path::PathBuf;

use chrono::{Duration, Utc};
use ennoia_kernel::{
    AgentDocument, AgentPermissionPolicy, AgentPermissionRule, PermissionApprovalRecord,
    PermissionDecision, PermissionEventRecord, PermissionRequest,
};
use ennoia_observability::RequestContext;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

use self::query::{FilterOperator, SelectQuery, UpdateQuery};
use self::schema::{
    ColumnDef, PermissionApprovalsSchema, PermissionEventsSchema, PermissionGrantsSchema,
    TableSchema,
};
use super::{
    grant_matches, is_expired_iso, now_iso, rule_matches, PermissionApprovalsQuery,
    PermissionEventsQuery, PermissionGrantRecord, PermissionPolicySummary,
};

const APPROVAL_TTL_MINUTES: i64 = 15;
const GRANT_ONCE_TTL_MINUTES: i64 = 10;
const SQLITE_PRAGMAS: &[&str] = &["PRAGMA journal_mode=WAL;", "PRAGMA synchronous=NORMAL;"];

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
        let store = Self {
            db_path: paths.permissions_db(),
            runtime_paths: paths.clone(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn load_policy(&self, agent_id: &str) -> std::io::Result<AgentPermissionPolicy> {
        let path = self.runtime_paths.agent_config_file(agent_id);
        if !path.exists() {
            return Ok(AgentPermissionPolicy::builtin_worker(agent_id));
        }
        let contents = fs::read_to_string(path)?;
        let document = toml::from_str::<AgentDocument>(&contents).map_err(std::io::Error::other)?;
        Ok(document.permission_policy)
    }

    pub fn save_policy(
        &self,
        agent_id: &str,
        policy: &AgentPermissionPolicy,
    ) -> std::io::Result<()> {
        let path = self.runtime_paths.agent_config_file(agent_id);
        if !path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("agent '{agent_id}' not found"),
            ));
        }
        let mut document = toml::from_str::<AgentDocument>(&fs::read_to_string(&path)?)
            .map_err(std::io::Error::other)?;
        document.permission_policy = policy.clone();
        fs::write(
            path,
            toml::to_string_pretty(&document).map_err(std::io::Error::other)?,
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
        let connection = self.open()?;
        let mut builder = UpdateQuery::new(PermissionGrantsSchema::NAME);
        builder.push_assignment(PermissionGrantsSchema::CONSUMED_AT, now_iso().into());
        builder.push_filter(
            PermissionGrantsSchema::GRANT_ID,
            FilterOperator::Eq,
            grant_id.to_string().into(),
        );
        builder.push_null_filter(PermissionGrantsSchema::CONSUMED_AT);
        builder.build().execute(&connection).map(|_| ())
    }

    pub fn list_events(
        &self,
        query: &PermissionEventsQuery,
    ) -> std::io::Result<Vec<PermissionEventRecord>> {
        let connection = self.open()?;
        let prepared = build_permission_events_query(query).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(prepared.params),
                map_permission_event,
            )
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn list_approvals(
        &self,
        query: &PermissionApprovalsQuery,
    ) -> std::io::Result<Vec<PermissionApprovalRecord>> {
        self.expire_pending_approvals()?;
        let connection = self.open()?;
        let prepared = build_permission_approvals_query(query).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(prepared.params),
                map_permission_approval,
            )
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn resolve_approval(
        &self,
        approval_id: &str,
        resolution: &str,
    ) -> std::io::Result<Option<PermissionApprovalRecord>> {
        self.expire_pending_approvals()?;
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
        let mut builder = UpdateQuery::new(PermissionApprovalsSchema::NAME);
        builder.push_assignment(PermissionApprovalsSchema::STATUS, status.to_string().into());
        builder.push_assignment(
            PermissionApprovalsSchema::RESOLUTION,
            normalized_resolution.clone().into(),
        );
        builder.push_assignment(
            PermissionApprovalsSchema::RESOLVED_AT,
            resolved_at.clone().into(),
        );
        builder.push_filter(
            PermissionApprovalsSchema::APPROVAL_ID,
            FilterOperator::Eq,
            approval_id.to_string().into(),
        );
        builder.build().execute(&connection)?;

        approval.status = status.to_string();
        approval.resolution = Some(normalized_resolution.clone());
        approval.resolved_at = Some(resolved_at);

        if status == "approved" {
            self.apply_approval_resolution(&approval, &normalized_resolution)?;
        }

        Ok(Some(approval))
    }

    pub fn latest_conversation_approval_seq(&self, conversation_id: &str) -> std::io::Result<i64> {
        self.expire_pending_approvals()?;
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT seq
                 FROM permission_approvals
                 WHERE conversation_id = ?1
                 ORDER BY seq DESC
                 LIMIT 1",
                params![conversation_id],
                |row| row.get::<_, i64>(0),
            )
            .optional()
            .map(|value| value.unwrap_or(0))
            .map_err(std::io::Error::other)
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
        let request = PermissionRequest {
            agent_id: approval.agent_id.clone(),
            action: approval.action.clone(),
            target: approval.target.clone(),
            scope: approval.scope.clone(),
            trigger: approval.trigger.clone(),
        };
        let connection = self.open()?;
        connection
            .execute(
                &PermissionGrantsSchema::insert_statement(),
                params![
                    format!("grant-{}", Uuid::new_v4()),
                    approval.approval_id,
                    approval.agent_id,
                    mode,
                    serde_json::to_string(&request).map_err(std::io::Error::other)?,
                    Option::<String>::None,
                    if mode == "once" {
                        Some((Utc::now() + Duration::minutes(GRANT_ONCE_TTL_MINUTES)).to_rfc3339())
                    } else {
                        None
                    },
                    now_iso(),
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
            expires_at: Some((Utc::now() + Duration::minutes(APPROVAL_TTL_MINUTES)).to_rfc3339()),
            resolved_at: None,
            resolution: None,
        };

        let connection = self.open()?;
        connection
            .execute(
                &PermissionApprovalsSchema::insert_statement(),
                params![
                    approval.approval_id,
                    approval.status,
                    approval.agent_id,
                    approval.action,
                    approval.scope.conversation_id.clone(),
                    approval.scope.run_id.clone(),
                    approval.scope.message_id.clone(),
                    serde_json::to_string(&approval.target).map_err(std::io::Error::other)?,
                    serde_json::to_string(&approval.scope).map_err(std::io::Error::other)?,
                    serde_json::to_string(&approval.trigger).map_err(std::io::Error::other)?,
                    approval.matched_rule_id,
                    approval.reason,
                    approval.created_at,
                    approval.expires_at,
                    approval.resolved_at,
                    approval.resolution,
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

        let connection = self.open()?;
        connection
            .execute(
                &PermissionEventsSchema::insert_statement(),
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
        let prepared = build_permission_grants_query(request).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(prepared.params),
                map_permission_grant,
            )
            .map_err(std::io::Error::other)?;

        for row in rows {
            let grant = row.map_err(std::io::Error::other)?;
            if grant.consumed_at.is_some() {
                continue;
            }
            if let Some(expires_at) = &grant.expires_at {
                if is_expired_iso(expires_at) {
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
        let connection = self.open()?;
        let mut builder = SelectQuery::new(
            PermissionApprovalsSchema::NAME,
            PermissionApprovalsSchema::SELECT_COLUMNS,
        );
        builder.push_filter(
            PermissionApprovalsSchema::APPROVAL_ID,
            FilterOperator::Eq,
            approval_id.to_string().into(),
        );
        let prepared = builder.build();
        let mut statement = prepared.prepare(&connection)?;
        statement
            .query_row(
                rusqlite::params_from_iter(prepared.params),
                map_permission_approval,
            )
            .optional()
            .map_err(std::io::Error::other)
    }

    fn expire_pending_approvals(&self) -> std::io::Result<()> {
        let now = now_iso();
        let connection = self.open()?;
        let mut builder = UpdateQuery::new(PermissionApprovalsSchema::NAME);
        builder.push_assignment(
            PermissionApprovalsSchema::STATUS,
            "expired".to_string().into(),
        );
        builder.push_assignment(
            PermissionApprovalsSchema::RESOLUTION,
            "expired".to_string().into(),
        );
        builder.push_assignment(PermissionApprovalsSchema::RESOLVED_AT, now.clone().into());
        builder.push_filter(
            PermissionApprovalsSchema::STATUS,
            FilterOperator::Eq,
            "pending".to_string().into(),
        );
        builder.push_filter(
            PermissionApprovalsSchema::EXPIRES_AT,
            FilterOperator::Lte,
            now.into(),
        );
        builder.build().execute(&connection).map(|_| ())
    }

    fn open(&self) -> std::io::Result<Connection> {
        let connection = Connection::open(&self.db_path).map_err(std::io::Error::other)?;
        for pragma in SQLITE_PRAGMAS {
            connection
                .execute_batch(pragma)
                .map_err(std::io::Error::other)?;
        }
        Ok(connection)
    }

    fn ensure_schema(&self) -> std::io::Result<()> {
        let connection = self.open()?;
        for statement in schema::table_statements() {
            connection
                .execute_batch(&statement)
                .map_err(std::io::Error::other)?;
        }
        ensure_columns(
            &connection,
            PermissionApprovalsSchema::NAME,
            PermissionApprovalsSchema::LEGACY_COLUMNS,
        )?;
        for statement in schema::index_statements() {
            connection
                .execute_batch(&statement)
                .map_err(std::io::Error::other)?;
        }
        Ok(())
    }
}

fn build_permission_events_query(query: &PermissionEventsQuery) -> SelectQuery {
    let mut builder = SelectQuery::new(
        PermissionEventsSchema::NAME,
        PermissionEventsSchema::SELECT_COLUMNS,
    );
    if let Some(agent_id) = &query.agent_id {
        builder.push_filter(
            PermissionEventsSchema::AGENT_ID,
            FilterOperator::Eq,
            agent_id.clone().into(),
        );
    }
    if let Some(action) = &query.action {
        builder.push_filter(
            PermissionEventsSchema::ACTION,
            FilterOperator::Eq,
            action.clone().into(),
        );
    }
    if let Some(decision) = &query.decision {
        builder.push_filter(
            PermissionEventsSchema::DECISION,
            FilterOperator::Eq,
            decision.clone().into(),
        );
    }
    builder
        .order_by(PermissionEventsSchema::SEQ, true)
        .limit(query.limit.max(1) as i64)
}

fn build_permission_approvals_query(query: &PermissionApprovalsQuery) -> SelectQuery {
    let mut builder = SelectQuery::new(
        PermissionApprovalsSchema::NAME,
        PermissionApprovalsSchema::SELECT_COLUMNS,
    );
    if let Some(agent_id) = &query.agent_id {
        builder.push_filter(
            PermissionApprovalsSchema::AGENT_ID,
            FilterOperator::Eq,
            agent_id.clone().into(),
        );
    }
    if let Some(conversation_id) = &query.conversation_id {
        builder.push_filter(
            PermissionApprovalsSchema::CONVERSATION_ID,
            FilterOperator::Eq,
            conversation_id.clone().into(),
        );
    }
    if let Some(status) = &query.status {
        builder.push_filter(
            PermissionApprovalsSchema::STATUS,
            FilterOperator::Eq,
            status.clone().into(),
        );
    }
    builder
        .order_by(PermissionApprovalsSchema::SEQ, true)
        .limit(query.limit.max(1) as i64)
}

fn build_permission_grants_query(request: &PermissionRequest) -> SelectQuery {
    let mut builder = SelectQuery::new(
        PermissionGrantsSchema::NAME,
        PermissionGrantsSchema::SELECT_COLUMNS,
    );
    builder.push_filter(
        PermissionGrantsSchema::AGENT_ID,
        FilterOperator::Eq,
        request.agent_id.clone().into(),
    );
    builder.order_by(PermissionGrantsSchema::SEQ, true)
}

fn map_permission_event(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionEventRecord> {
    let target_json: String = row.get(PermissionEventsSchema::TARGET_JSON)?;
    let scope_json: String = row.get(PermissionEventsSchema::SCOPE_JSON)?;
    Ok(PermissionEventRecord {
        event_id: row.get(PermissionEventsSchema::EVENT_ID)?,
        agent_id: row.get(PermissionEventsSchema::AGENT_ID)?,
        action: row.get(PermissionEventsSchema::ACTION)?,
        decision: row.get(PermissionEventsSchema::DECISION)?,
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
        extension_id: row.get(PermissionEventsSchema::EXTENSION_ID)?,
        matched_rule_id: row.get(PermissionEventsSchema::MATCHED_RULE_ID)?,
        approval_id: row.get(PermissionEventsSchema::APPROVAL_ID)?,
        trace_id: row.get(PermissionEventsSchema::TRACE_ID)?,
        created_at: row.get(PermissionEventsSchema::CREATED_AT)?,
    })
}

fn map_permission_grant(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionGrantRecord> {
    let request_json: String = row.get(PermissionGrantsSchema::REQUEST_JSON)?;
    let request = serde_json::from_str::<PermissionRequest>(&request_json).map_err(|error| {
        rusqlite::Error::FromSqlConversionFailure(
            request_json.len(),
            rusqlite::types::Type::Text,
            Box::new(error),
        )
    })?;
    Ok(PermissionGrantRecord {
        grant_id: row.get(PermissionGrantsSchema::GRANT_ID)?,
        approval_id: row.get(PermissionGrantsSchema::APPROVAL_ID)?,
        agent_id: row.get(PermissionGrantsSchema::AGENT_ID)?,
        mode: row.get(PermissionGrantsSchema::MODE)?,
        request,
        consumed_at: row.get(PermissionGrantsSchema::CONSUMED_AT)?,
        expires_at: row.get(PermissionGrantsSchema::EXPIRES_AT)?,
    })
}

fn map_permission_approval(row: &rusqlite::Row<'_>) -> rusqlite::Result<PermissionApprovalRecord> {
    let target_json: String = row.get(PermissionApprovalsSchema::TARGET_JSON)?;
    let scope_json: String = row.get(PermissionApprovalsSchema::SCOPE_JSON)?;
    let trigger_json: String = row.get(PermissionApprovalsSchema::TRIGGER_JSON)?;
    Ok(PermissionApprovalRecord {
        approval_id: row.get(PermissionApprovalsSchema::APPROVAL_ID)?,
        status: row.get(PermissionApprovalsSchema::STATUS)?,
        agent_id: row.get(PermissionApprovalsSchema::AGENT_ID)?,
        action: row.get(PermissionApprovalsSchema::ACTION)?,
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
        matched_rule_id: row.get(PermissionApprovalsSchema::MATCHED_RULE_ID)?,
        reason: row.get(PermissionApprovalsSchema::REASON)?,
        created_at: row.get(PermissionApprovalsSchema::CREATED_AT)?,
        expires_at: row.get(PermissionApprovalsSchema::EXPIRES_AT)?,
        resolved_at: row.get(PermissionApprovalsSchema::RESOLVED_AT)?,
        resolution: row.get(PermissionApprovalsSchema::RESOLUTION)?,
    })
}

fn ensure_columns(
    connection: &Connection,
    table: &str,
    columns: &[ColumnDef],
) -> std::io::Result<()> {
    let pragma = format!("PRAGMA table_info({table})");
    let mut statement = connection.prepare(&pragma).map_err(std::io::Error::other)?;
    let rows = statement
        .query_map([], |row| row.get::<_, String>("name"))
        .map_err(std::io::Error::other)?;
    let existing = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(std::io::Error::other)?;

    for column in columns {
        if existing.iter().any(|name| name == column.name) {
            continue;
        }
        let statement = format!(
            "ALTER TABLE {} ADD COLUMN {}",
            table,
            column.render_definition()
        );
        connection
            .execute_batch(&statement)
            .map_err(std::io::Error::other)?;
    }
    Ok(())
}
