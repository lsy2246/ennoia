use std::path::PathBuf;

use chrono::Utc;
pub use ennoia_kernel::{
    ExtensionRecordAppend, ExtensionRecordEntry, ExtensionRecordListQuery, ExtensionRecordUpdate,
    ExtensionStateEntry, ExtensionStateGetQuery, ExtensionStateListQuery, ExtensionStatePut,
};
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

const EXTENSION_RUNTIME_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS extension_state_entries (
  extension_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  scope_type TEXT NOT NULL,
  scope_id TEXT NOT NULL,
  state_key TEXT NOT NULL,
  value_json TEXT NOT NULL,
  version INTEGER NOT NULL,
  updated_at TEXT NOT NULL,
  expires_at TEXT,
  PRIMARY KEY (extension_id, namespace, scope_type, scope_id, state_key)
);
CREATE INDEX IF NOT EXISTS idx_extension_state_scope
  ON extension_state_entries(extension_id, namespace, scope_type, scope_id, updated_at DESC);

CREATE TABLE IF NOT EXISTS extension_record_entries (
  id TEXT PRIMARY KEY,
  extension_id TEXT NOT NULL,
  namespace TEXT NOT NULL,
  scope_type TEXT NOT NULL,
  scope_id TEXT NOT NULL,
  kind TEXT NOT NULL,
  status TEXT,
  title TEXT,
  summary TEXT,
  payload_json TEXT NOT NULL,
  related_message_id TEXT,
  parent_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  closed_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_extension_records_scope
  ON extension_record_entries(extension_id, namespace, scope_type, scope_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_extension_records_related_message
  ON extension_record_entries(related_message_id, created_at DESC);
"#;

#[derive(Debug, Clone)]
pub struct ExtensionRuntimeStore {
    db_path: PathBuf,
}

impl ExtensionRuntimeStore {
    pub fn new(paths: &RuntimePaths) -> std::io::Result<Self> {
        if let Some(parent) = paths.extensions_runtime_db().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: paths.extensions_runtime_db(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn get_state(
        &self,
        query: &ExtensionStateGetQuery,
    ) -> std::io::Result<Option<ExtensionStateEntry>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT extension_id, namespace, scope_type, scope_id, state_key, value_json, version, updated_at, expires_at
             FROM extension_state_entries
             WHERE extension_id = ?1 AND namespace = ?2 AND scope_type = ?3 AND scope_id = ?4 AND state_key = ?5",
        )
        .map_err(sql_err)?;
        let row = statement
            .query_row(
                params![
                    query.extension_id,
                    query.namespace,
                    query.scope_type,
                    query.scope_id,
                    query.key
                ],
                map_state_entry,
            )
            .optional()
            .map_err(sql_err)?;
        Ok(row)
    }

    pub fn put_state(&self, payload: &ExtensionStatePut) -> std::io::Result<ExtensionStateEntry> {
        let connection = self.open()?;
        let current = self.get_state(&ExtensionStateGetQuery {
            extension_id: payload.extension_id.clone(),
            namespace: payload.namespace.clone(),
            scope_type: payload.scope_type.clone(),
            scope_id: payload.scope_id.clone(),
            key: payload.key.clone(),
        })?;
        let version = current.map(|item| item.version + 1).unwrap_or(1);
        let updated_at = now_iso();
        let value_json = serde_json::to_string(&payload.value).map_err(std::io::Error::other)?;
        connection.execute(
            "INSERT INTO extension_state_entries
             (extension_id, namespace, scope_type, scope_id, state_key, value_json, version, updated_at, expires_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
             ON CONFLICT(extension_id, namespace, scope_type, scope_id, state_key) DO UPDATE SET
               value_json = excluded.value_json,
               version = excluded.version,
               updated_at = excluded.updated_at,
               expires_at = excluded.expires_at",
            params![
                payload.extension_id,
                payload.namespace,
                payload.scope_type,
                payload.scope_id,
                payload.key,
                value_json,
                version,
                updated_at,
                payload.expires_at,
            ],
        )
        .map_err(sql_err)?;
        self.get_state(&ExtensionStateGetQuery {
            extension_id: payload.extension_id.clone(),
            namespace: payload.namespace.clone(),
            scope_type: payload.scope_type.clone(),
            scope_id: payload.scope_id.clone(),
            key: payload.key.clone(),
        })?
        .ok_or_else(|| std::io::Error::other("failed to reload extension state"))
    }

    pub fn delete_state(&self, query: &ExtensionStateGetQuery) -> std::io::Result<bool> {
        let connection = self.open()?;
        let affected = connection.execute(
            "DELETE FROM extension_state_entries
             WHERE extension_id = ?1 AND namespace = ?2 AND scope_type = ?3 AND scope_id = ?4 AND state_key = ?5",
            params![query.extension_id, query.namespace, query.scope_type, query.scope_id, query.key],
        )
        .map_err(sql_err)?;
        Ok(affected > 0)
    }

    pub fn list_state(
        &self,
        query: &ExtensionStateListQuery,
    ) -> std::io::Result<Vec<ExtensionStateEntry>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT extension_id, namespace, scope_type, scope_id, state_key, value_json, version, updated_at, expires_at
             FROM extension_state_entries WHERE 1 = 1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(value) = &query.extension_id {
            sql.push_str(" AND extension_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.namespace {
            sql.push_str(" AND namespace = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.scope_type {
            sql.push_str(" AND scope_type = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.scope_id {
            sql.push_str(" AND scope_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.key {
            sql.push_str(" AND state_key = ?");
            params.push(Box::new(value.clone()));
        }
        sql.push_str(" ORDER BY updated_at DESC");
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit.max(1) as i64));
        }

        let mut statement = connection.prepare(&sql).map_err(sql_err)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|value| &**value)),
                map_state_entry,
            )
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn append_record(
        &self,
        payload: &ExtensionRecordAppend,
    ) -> std::io::Result<ExtensionRecordEntry> {
        let connection = self.open()?;
        let id = format!("extrec-{}", Uuid::new_v4());
        let now = now_iso();
        let payload_json =
            serde_json::to_string(&payload.payload).map_err(std::io::Error::other)?;
        connection.execute(
            "INSERT INTO extension_record_entries
             (id, extension_id, namespace, scope_type, scope_id, kind, status, title, summary, payload_json, related_message_id, parent_id, created_at, updated_at, closed_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, NULL)",
            params![
                id,
                payload.extension_id,
                payload.namespace,
                payload.scope_type,
                payload.scope_id,
                payload.kind,
                payload.status,
                payload.title,
                payload.summary,
                payload_json,
                payload.related_message_id,
                payload.parent_id,
                now.clone(),
                now,
            ],
        )
        .map_err(sql_err)?;
        self.get_record(&id)?
            .ok_or_else(|| std::io::Error::other("failed to reload extension record"))
    }

    pub fn update_record(
        &self,
        payload: &ExtensionRecordUpdate,
    ) -> std::io::Result<Option<ExtensionRecordEntry>> {
        let Some(mut current) = self.get_record(&payload.id)? else {
            return Ok(None);
        };
        let connection = self.open()?;
        let now = now_iso();
        let next_payload = payload
            .payload
            .clone()
            .unwrap_or_else(|| current.payload.clone());
        let payload_json = serde_json::to_string(&next_payload).map_err(std::io::Error::other)?;
        let related_message_id = payload.related_message_id.clone().flatten();
        let parent_id = payload.parent_id.clone().flatten();
        connection
            .execute(
                "UPDATE extension_record_entries
             SET status = COALESCE(?2, status),
                 title = COALESCE(?3, title),
                 summary = COALESCE(?4, summary),
                 payload_json = ?5,
                 related_message_id = COALESCE(?6, related_message_id),
                 parent_id = COALESCE(?7, parent_id),
                 updated_at = ?8,
                 closed_at = COALESCE(closed_at, ?9)
             WHERE id = ?1",
                params![
                    payload.id,
                    payload.status,
                    payload.title,
                    payload.summary,
                    payload_json,
                    related_message_id,
                    parent_id,
                    now.clone(),
                    current.closed_at.clone(),
                ],
            )
            .map_err(sql_err)?;
        if payload
            .status
            .as_deref()
            .is_some_and(|status| status == "closed")
            && current.closed_at.is_none()
        {
            connection
                .execute(
                    "UPDATE extension_record_entries SET closed_at = ?2 WHERE id = ?1",
                    params![payload.id, now.clone()],
                )
                .map_err(sql_err)?;
        }
        current = self.get_record(&payload.id)?.ok_or_else(|| {
            std::io::Error::other("failed to reload extension record after update")
        })?;
        Ok(Some(current))
    }

    pub fn close_record(&self, id: &str) -> std::io::Result<Option<ExtensionRecordEntry>> {
        let Some(_) = self.get_record(id)? else {
            return Ok(None);
        };
        let connection = self.open()?;
        let now = now_iso();
        connection.execute(
            "UPDATE extension_record_entries SET status = COALESCE(status, 'closed'), closed_at = ?2, updated_at = ?2 WHERE id = ?1",
            params![id, now],
        )
        .map_err(sql_err)?;
        let current = self.get_record(id)?.ok_or_else(|| {
            std::io::Error::other("failed to reload extension record after close")
        })?;
        Ok(Some(current))
    }

    pub fn get_record(&self, id: &str) -> std::io::Result<Option<ExtensionRecordEntry>> {
        let connection = self.open()?;
        let mut statement = connection.prepare(
            "SELECT id, extension_id, namespace, scope_type, scope_id, kind, status, title, summary, payload_json, related_message_id, parent_id, created_at, updated_at, closed_at
             FROM extension_record_entries WHERE id = ?1",
        )
        .map_err(sql_err)?;
        let row = statement
            .query_row(params![id], map_record_entry)
            .optional()
            .map_err(sql_err)?;
        Ok(row)
    }

    pub fn list_records(
        &self,
        query: &ExtensionRecordListQuery,
    ) -> std::io::Result<Vec<ExtensionRecordEntry>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT id, extension_id, namespace, scope_type, scope_id, kind, status, title, summary, payload_json, related_message_id, parent_id, created_at, updated_at, closed_at
             FROM extension_record_entries WHERE 1 = 1",
        );
        let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();
        if let Some(value) = &query.extension_id {
            sql.push_str(" AND extension_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.namespace {
            sql.push_str(" AND namespace = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.scope_type {
            sql.push_str(" AND scope_type = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.scope_id {
            sql.push_str(" AND scope_id = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.kind {
            sql.push_str(" AND kind = ?");
            params.push(Box::new(value.clone()));
        }
        if let Some(value) = &query.related_message_id {
            sql.push_str(" AND related_message_id = ?");
            params.push(Box::new(value.clone()));
        }
        if query.open_only.unwrap_or(false) {
            sql.push_str(" AND closed_at IS NULL");
        }
        sql.push_str(" ORDER BY created_at DESC");
        if let Some(limit) = query.limit {
            sql.push_str(" LIMIT ?");
            params.push(Box::new(limit.max(1) as i64));
        }

        let mut statement = connection.prepare(&sql).map_err(sql_err)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(params.iter().map(|value| &**value)),
                map_record_entry,
            )
            .map_err(sql_err)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn latest_conversation_record_updated_at(
        &self,
        conversation_id: &str,
    ) -> std::io::Result<Option<String>> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT updated_at
                 FROM extension_record_entries
                 WHERE scope_type = 'conversation' AND scope_id = ?1
                 ORDER BY updated_at DESC
                 LIMIT 1",
                params![conversation_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_err)
    }

    fn open(&self) -> std::io::Result<Connection> {
        let connection = Connection::open(&self.db_path).map_err(sql_err)?;
        for statement in SQLITE_PRAGMAS {
            connection.execute_batch(statement).map_err(sql_err)?;
        }
        Ok(connection)
    }

    fn ensure_schema(&self) -> std::io::Result<()> {
        let connection = self.open()?;
        connection
            .execute_batch(EXTENSION_RUNTIME_SCHEMA_SQL)
            .map_err(sql_err)?;
        Ok(())
    }
}

const SQLITE_PRAGMAS: &[&str] = &["PRAGMA journal_mode=WAL;", "PRAGMA synchronous=NORMAL;"];

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn sql_err(error: rusqlite::Error) -> std::io::Error {
    std::io::Error::other(error)
}

fn map_state_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExtensionStateEntry> {
    let value_json: String = row.get("value_json")?;
    Ok(ExtensionStateEntry {
        extension_id: row.get("extension_id")?,
        namespace: row.get("namespace")?,
        scope_type: row.get("scope_type")?,
        scope_id: row.get("scope_id")?,
        key: row.get("state_key")?,
        value: serde_json::from_str(&value_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                value_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        version: row.get("version")?,
        updated_at: row.get("updated_at")?,
        expires_at: row.get("expires_at")?,
    })
}

fn map_record_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ExtensionRecordEntry> {
    let payload_json: String = row.get("payload_json")?;
    Ok(ExtensionRecordEntry {
        id: row.get("id")?,
        extension_id: row.get("extension_id")?,
        namespace: row.get("namespace")?,
        scope_type: row.get("scope_type")?,
        scope_id: row.get("scope_id")?,
        kind: row.get("kind")?,
        status: row.get("status")?,
        title: row.get("title")?,
        summary: row.get("summary")?,
        payload: serde_json::from_str(&payload_json).map_err(|error| {
            rusqlite::Error::FromSqlConversionFailure(
                payload_json.len(),
                rusqlite::types::Type::Text,
                Box::new(error),
            )
        })?,
        related_message_id: row.get("related_message_id")?,
        parent_id: row.get("parent_id")?,
        created_at: row.get("created_at")?,
        updated_at: row.get("updated_at")?,
        closed_at: row.get("closed_at")?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    use tempfile::tempdir;

    #[test]
    fn latest_conversation_record_updated_at_tracks_new_records() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path());
        let store = ExtensionRuntimeStore::new(&paths).expect("extension runtime store");
        assert_eq!(
            store
                .latest_conversation_record_updated_at("conv-1")
                .expect("latest record timestamp"),
            None
        );

        let record = store
            .append_record(&ExtensionRecordAppend {
                extension_id: "artifact-runner".to_string(),
                namespace: "artifact-runner/conversation/conv-1".to_string(),
                scope_type: "conversation".to_string(),
                scope_id: "conv-1".to_string(),
                kind: "artifact-runner.artifact".to_string(),
                status: Some("ready".to_string()),
                title: Some("Canvas".to_string()),
                summary: Some("Canvas".to_string()),
                payload: serde_json::json!({ "type": "html-preview" }),
                related_message_id: Some("msg-1".to_string()),
                parent_id: None,
            })
            .expect("append record");

        assert_eq!(
            store
                .latest_conversation_record_updated_at("conv-1")
                .expect("latest record timestamp"),
            Some(record.updated_at)
        );
        assert_eq!(
            store
                .latest_conversation_record_updated_at("conv-2")
                .expect("latest other timestamp"),
            None
        );
    }
}
