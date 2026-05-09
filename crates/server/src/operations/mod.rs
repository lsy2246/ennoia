use std::io;

use chrono::Utc;
use ennoia_kernel::{
    OperationApprovalLink, OperationListQuery, OperationPerformRequest, OperationRecord,
    OperationStatus,
};
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::Value as JsonValue;
use uuid::Uuid;

const OPERATIONS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS operations (
  id TEXT PRIMARY KEY,
  extension_id TEXT NOT NULL,
  agent_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  branch_id TEXT,
  lane_id TEXT,
  run_id TEXT NOT NULL,
  message_id TEXT,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  status TEXT NOT NULL,
  input_json TEXT NOT NULL,
  output_json TEXT,
  error_json TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_operations_conversation
  ON operations(conversation_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_operations_run
  ON operations(run_id, updated_at DESC);
CREATE INDEX IF NOT EXISTS idx_operations_message
  ON operations(message_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS operation_approvals (
  operation_id TEXT NOT NULL,
  approval_id TEXT NOT NULL,
  created_at TEXT NOT NULL,
  PRIMARY KEY (operation_id, approval_id)
);
CREATE INDEX IF NOT EXISTS idx_operation_approvals_approval
  ON operation_approvals(approval_id, created_at DESC);

CREATE TABLE IF NOT EXISTS operation_events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  operation_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  event TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_operation_events_conversation
  ON operation_events(conversation_id, seq DESC);
"#;

#[derive(Debug, Clone)]
pub struct OperationStore {
    db_path: std::path::PathBuf,
}

#[derive(Debug, Clone)]
pub struct OperationResumeTarget {
    pub operation: OperationRecord,
    pub approval: Option<OperationApprovalLink>,
}

impl OperationStore {
    pub fn new(paths: &RuntimePaths) -> io::Result<Self> {
        if let Some(parent) = paths.operations_db().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: paths.operations_db(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn create_operation(
        &self,
        extension_id: &str,
        payload: &OperationPerformRequest,
    ) -> io::Result<OperationRecord> {
        let connection = self.open()?;
        let now = now_iso();
        let id = format!("op-{}", Uuid::new_v4());
        let input_json = serde_json::to_string(&payload.input).map_err(io::Error::other)?;
        connection
            .execute(
                "INSERT INTO operations
                 (id, extension_id, agent_id, conversation_id, branch_id, lane_id, run_id, message_id, kind, name, status, input_json, output_json, error_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, NULL, NULL, ?13, ?13)",
                params![
                    id,
                    extension_id,
                    payload.agent_id,
                    payload.conversation_id,
                    payload.branch_id,
                    payload.lane_id,
                    payload.run_id,
                    payload.message_id,
                    payload.kind,
                    payload.name,
                    OperationStatus::Queued.as_str(),
                    input_json,
                    now,
                ],
            )
            .map_err(sql_err)?;
        let record = self
            .get_operation_with_connection(&connection, &id)?
            .ok_or_else(|| io::Error::other("failed to reload operation"))?;
        self.cancel_superseded_active_operations_with_connection(&connection, &record)?;
        self.append_event_with_connection(&connection, &record, "queued")?;
        Ok(record)
    }

    pub fn update_operation(
        &self,
        operation_id: &str,
        status: OperationStatus,
        output: Option<JsonValue>,
        error: Option<JsonValue>,
    ) -> io::Result<OperationRecord> {
        let connection = self.open()?;
        let now = now_iso();
        let output_json = output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(io::Error::other)?;
        let error_json = error
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(io::Error::other)?;
        connection
            .execute(
                "UPDATE operations
                 SET status = ?2,
                     output_json = ?3,
                     error_json = ?4,
                     updated_at = ?5
                 WHERE id = ?1",
                params![operation_id, status.as_str(), output_json, error_json, now,],
            )
            .map_err(sql_err)?;
        let record = self
            .get_operation(operation_id)?
            .ok_or_else(|| io::Error::other("operation not found after update"))?;
        self.append_event_with_connection(&connection, &record, status.as_str())?;
        Ok(record)
    }

    pub fn link_approval(&self, operation_id: &str, approval_id: &str) -> io::Result<()> {
        let connection = self.open()?;
        let now = now_iso();
        connection
            .execute(
                "INSERT OR IGNORE INTO operation_approvals (operation_id, approval_id, created_at)
                 VALUES (?1, ?2, ?3)",
                params![operation_id, approval_id, now],
            )
            .map_err(sql_err)?;
        Ok(())
    }

    pub fn list_operations(&self, query: &OperationListQuery) -> io::Result<Vec<OperationRecord>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT id, extension_id, agent_id, conversation_id, branch_id, lane_id, run_id, message_id, kind, name, status, input_json, output_json, error_json, created_at, updated_at
             FROM operations WHERE 1 = 1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(value) = &query.conversation_id {
            sql.push_str(" AND conversation_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.run_id {
            sql.push_str(" AND run_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.message_id {
            sql.push_str(" AND message_id = ?");
            params.push(Box::new(value.clone()));
        }
        sql.push_str(" ORDER BY updated_at ASC");
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit.max(1) as i64));
        }
        let mut statement = connection.prepare(&sql).map_err(sql_err)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|value| &**value)),
                map_operation_record,
            )
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(io::Error::other)
    }

    pub fn get_operation(&self, operation_id: &str) -> io::Result<Option<OperationRecord>> {
        let connection = self.open()?;
        self.get_operation_with_connection(&connection, operation_id)
    }

    pub fn cancel_abandoned_operations(&self) -> io::Result<Vec<OperationRecord>> {
        let connection = self.open()?;
        let stale =
            self.list_operations_by_statuses_with_connection(&connection, &["queued", "running"])?;
        let mut cancelled = Vec::with_capacity(stale.len());
        for operation in stale {
            let reason = serde_json::json!({
                "reason": "abandoned",
                "message": "operation was cancelled because the server restarted before it completed",
            });
            let updated = self.update_operation_with_connection(
                &connection,
                &operation.id,
                OperationStatus::Cancelled,
                None,
                Some(reason),
            )?;
            cancelled.push(updated);
        }
        Ok(cancelled)
    }

    fn get_operation_with_connection(
        &self,
        connection: &Connection,
        operation_id: &str,
    ) -> io::Result<Option<OperationRecord>> {
        let mut statement = connection
            .prepare(
                "SELECT id, extension_id, agent_id, conversation_id, branch_id, lane_id, run_id, message_id, kind, name, status, input_json, output_json, error_json, created_at, updated_at
                 FROM operations
                 WHERE id = ?1",
            )
            .map_err(sql_err)?;
        statement
            .query_row(params![operation_id], map_operation_record)
            .optional()
            .map_err(sql_err)
    }

    pub fn latest_conversation_seq(&self, conversation_id: &str) -> io::Result<i64> {
        let connection = self.open()?;
        let seq = connection
            .query_row(
                "SELECT COALESCE(MAX(seq), 0) FROM operation_events WHERE conversation_id = ?1",
                params![conversation_id],
                |row| row.get::<usize, i64>(0),
            )
            .map_err(sql_err)?;
        Ok(seq)
    }

    pub fn find_resume_target_by_approval(
        &self,
        approval_id: &str,
    ) -> io::Result<Option<OperationResumeTarget>> {
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT o.id, o.extension_id, o.agent_id, o.conversation_id, o.branch_id, o.lane_id, o.run_id, o.message_id, o.kind, o.name, o.status, o.input_json, o.output_json, o.error_json, o.created_at, o.updated_at,
                        oa.operation_id, oa.approval_id, oa.created_at
                 FROM operation_approvals oa
                 JOIN operations o ON o.id = oa.operation_id
                 WHERE oa.approval_id = ?1
                 ORDER BY oa.created_at DESC
                 LIMIT 1",
            )
            .map_err(sql_err)?;
        statement
            .query_row(params![approval_id], |row| {
                let operation = map_operation_record(row)?;
                let approval = OperationApprovalLink {
                    operation_id: row.get(16)?,
                    approval_id: row.get(17)?,
                    created_at: row.get(18)?,
                };
                Ok(OperationResumeTarget {
                    operation,
                    approval: Some(approval),
                })
            })
            .optional()
            .map_err(sql_err)
    }

    fn open(&self) -> io::Result<Connection> {
        Connection::open(&self.db_path).map_err(sql_err)
    }

    fn ensure_schema(&self) -> io::Result<()> {
        let connection = self.open()?;
        connection
            .execute_batch(OPERATIONS_SCHEMA_SQL)
            .map_err(sql_err)
    }

    fn append_event_with_connection(
        &self,
        connection: &Connection,
        record: &OperationRecord,
        event: &str,
    ) -> io::Result<()> {
        let payload_json = serde_json::to_string(record).map_err(io::Error::other)?;
        connection
            .execute(
                "INSERT INTO operation_events (id, operation_id, conversation_id, event, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("opevt-{}", Uuid::new_v4()),
                    record.id,
                    record.conversation_id,
                    event,
                    payload_json,
                    now_iso(),
                ],
            )
            .map(|_| ())
            .map_err(sql_err)
    }

    fn update_operation_with_connection(
        &self,
        connection: &Connection,
        operation_id: &str,
        status: OperationStatus,
        output: Option<JsonValue>,
        error: Option<JsonValue>,
    ) -> io::Result<OperationRecord> {
        let now = now_iso();
        let output_json = output
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(io::Error::other)?;
        let error_json = error
            .as_ref()
            .map(serde_json::to_string)
            .transpose()
            .map_err(io::Error::other)?;
        connection
            .execute(
                "UPDATE operations
                 SET status = ?2,
                     output_json = ?3,
                     error_json = ?4,
                     updated_at = ?5
                 WHERE id = ?1",
                params![operation_id, status.as_str(), output_json, error_json, now,],
            )
            .map_err(sql_err)?;
        let record = self
            .get_operation_with_connection(connection, operation_id)?
            .ok_or_else(|| io::Error::other("operation not found after update"))?;
        self.append_event_with_connection(connection, &record, status.as_str())?;
        Ok(record)
    }

    fn cancel_superseded_active_operations_with_connection(
        &self,
        connection: &Connection,
        current: &OperationRecord,
    ) -> io::Result<Vec<OperationRecord>> {
        let mut statement = connection
            .prepare(
                "SELECT id, extension_id, agent_id, conversation_id, branch_id, lane_id, run_id, message_id, kind, name, status, input_json, output_json, error_json, created_at, updated_at
                 FROM operations
                 WHERE id <> ?1
                   AND extension_id = ?2
                   AND agent_id = ?3
                   AND conversation_id = ?4
                   AND COALESCE(branch_id, '') = COALESCE(?5, '')
                   AND COALESCE(lane_id, '') = COALESCE(?6, '')
                   AND run_id = ?7
                   AND COALESCE(message_id, '') = COALESCE(?8, '')
                   AND kind = ?9
                   AND name = ?10
                   AND status IN ('queued', 'running', 'blocked')
                 ORDER BY updated_at ASC",
            )
            .map_err(sql_err)?;
        let rows = statement
            .query_map(
                params![
                    current.id,
                    current.extension_id,
                    current.agent_id,
                    current.conversation_id,
                    current.branch_id,
                    current.lane_id,
                    current.run_id,
                    current.message_id,
                    current.kind,
                    current.name,
                ],
                map_operation_record,
            )
            .map_err(sql_err)?;
        let mut cancelled = Vec::new();
        for row in rows {
            let operation = row.map_err(io::Error::other)?;
            let reason = serde_json::json!({
                "reason": "superseded",
                "message": "operation was cancelled because a newer operation took over the same execution slot",
                "superseded_by_operation_id": current.id,
            });
            let updated = self.update_operation_with_connection(
                connection,
                &operation.id,
                OperationStatus::Cancelled,
                None,
                Some(reason),
            )?;
            cancelled.push(updated);
        }
        Ok(cancelled)
    }

    fn list_operations_by_statuses_with_connection(
        &self,
        connection: &Connection,
        statuses: &[&str],
    ) -> io::Result<Vec<OperationRecord>> {
        if statuses.is_empty() {
            return Ok(Vec::new());
        }
        let placeholders = std::iter::repeat_n("?", statuses.len())
            .collect::<Vec<_>>()
            .join(", ");
        let sql = format!(
            "SELECT id, extension_id, agent_id, conversation_id, branch_id, lane_id, run_id, message_id, kind, name, status, input_json, output_json, error_json, created_at, updated_at
             FROM operations
             WHERE status IN ({placeholders})
             ORDER BY updated_at ASC"
        );
        let mut statement = connection.prepare(&sql).map_err(sql_err)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(statuses.iter().copied()),
                map_operation_record,
            )
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(io::Error::other)
    }
}

fn map_operation_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<OperationRecord> {
    let input_json: String = row.get(11)?;
    let output_json: Option<String> = row.get(12)?;
    let error_json: Option<String> = row.get(13)?;
    let status = match row.get::<usize, String>(10)?.as_str() {
        "queued" => OperationStatus::Queued,
        "running" => OperationStatus::Running,
        "blocked" => OperationStatus::Blocked,
        "succeeded" => OperationStatus::Succeeded,
        "failed" => OperationStatus::Failed,
        "cancelled" => OperationStatus::Cancelled,
        _ => OperationStatus::Failed,
    };
    Ok(OperationRecord {
        id: row.get(0)?,
        extension_id: row.get(1)?,
        agent_id: row.get(2)?,
        conversation_id: row.get(3)?,
        branch_id: row.get(4)?,
        lane_id: row.get(5)?,
        run_id: row.get(6)?,
        message_id: row.get(7)?,
        kind: row.get(8)?,
        name: row.get(9)?,
        status,
        input: serde_json::from_str(&input_json).unwrap_or(JsonValue::Null),
        output: output_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<JsonValue>(value).ok()),
        error: error_json
            .as_deref()
            .and_then(|value| serde_json::from_str::<JsonValue>(value).ok()),
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn sql_err(error: rusqlite::Error) -> io::Error {
    io::Error::other(error)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_paths::RuntimePaths;
    use tempfile::tempdir;

    fn sample_request() -> OperationPerformRequest {
        OperationPerformRequest {
            agent_id: "agent-a".to_string(),
            conversation_id: "conv-1".to_string(),
            run_id: "run-1".to_string(),
            branch_id: Some("branch-main".to_string()),
            lane_id: None,
            message_id: Some("msg-1".to_string()),
            kind: "provider".to_string(),
            name: "generate".to_string(),
            deferred: false,
            input: serde_json::json!({ "prompt": "hello" }),
        }
    }

    #[test]
    fn create_operation_cancels_previous_active_peer_in_same_slot() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path());
        let store = OperationStore::new(&paths).expect("store");
        let payload = sample_request();

        let first = store
            .create_operation("workflow", &payload)
            .expect("first op");
        let first_running = store
            .update_operation(&first.id, OperationStatus::Running, None, None)
            .expect("mark running");
        assert_eq!(first_running.status, OperationStatus::Running);

        let second = store
            .create_operation("workflow", &payload)
            .expect("second op");
        let reloaded_first = store
            .get_operation(&first.id)
            .expect("reload first")
            .expect("first exists");

        assert_eq!(second.status, OperationStatus::Queued);
        assert_eq!(reloaded_first.status, OperationStatus::Cancelled);
        assert_eq!(
            reloaded_first
                .error
                .as_ref()
                .and_then(|value| value.get("reason"))
                .and_then(JsonValue::as_str),
            Some("superseded")
        );
    }

    #[test]
    fn cancel_abandoned_operations_preserves_blocked_entries() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path());
        let store = OperationStore::new(&paths).expect("store");

        let running = store
            .create_operation("workflow", &sample_request())
            .expect("queued");
        store
            .update_operation(&running.id, OperationStatus::Running, None, None)
            .expect("running");

        let mut blocked_payload = sample_request();
        blocked_payload.message_id = Some("msg-2".to_string());
        let blocked = store
            .create_operation("workflow", &blocked_payload)
            .expect("blocked queued");
        store
            .update_operation(
                &blocked.id,
                OperationStatus::Blocked,
                None,
                Some(serde_json::json!({ "approval_id": "apr-1" })),
            )
            .expect("blocked");

        let cancelled = store
            .cancel_abandoned_operations()
            .expect("cancel abandoned");
        let reloaded_running = store
            .get_operation(&running.id)
            .expect("reload running")
            .expect("running exists");
        let reloaded_blocked = store
            .get_operation(&blocked.id)
            .expect("reload blocked")
            .expect("blocked exists");

        assert_eq!(cancelled.len(), 1);
        assert_eq!(reloaded_running.status, OperationStatus::Cancelled);
        assert_eq!(reloaded_blocked.status, OperationStatus::Blocked);
        assert_eq!(
            reloaded_running
                .error
                .as_ref()
                .and_then(|value| value.get("reason"))
                .and_then(JsonValue::as_str),
            Some("abandoned")
        );
    }
}
