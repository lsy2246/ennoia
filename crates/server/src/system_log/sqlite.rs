use std::path::PathBuf;

use chrono::Utc;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub const SYSTEM_LOG_COMPONENT_HOST: &str = "host";
pub const SYSTEM_LOG_COMPONENT_EXTENSION_HOST: &str = "extension_host";
pub const SYSTEM_LOG_COMPONENT_BEHAVIOR: &str = "behavior_router";
pub const SYSTEM_LOG_COMPONENT_MEMORY: &str = "memory_router";
pub const SYSTEM_LOG_COMPONENT_PROXY: &str = "extension_proxy";

const SYSTEM_LOG_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS system_log (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  event TEXT NOT NULL,
  level TEXT NOT NULL,
  component TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_id TEXT,
  summary TEXT NOT NULL,
  payload_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_system_log_event_time
  ON system_log(event, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_system_log_component_time
  ON system_log(component, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_system_log_source_time
  ON system_log(source_kind, source_id, created_at DESC);
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SystemLogEntry {
    pub id: String,
    pub seq: i64,
    pub event: String,
    pub level: String,
    pub component: String,
    pub source_kind: String,
    #[serde(default)]
    pub source_id: Option<String>,
    pub summary: String,
    #[serde(default)]
    pub payload: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct SystemLogQuery {
    pub event: Option<String>,
    pub level: Option<String>,
    pub component: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub before_seq: Option<i64>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct SystemLogWrite {
    pub event: String,
    pub level: String,
    pub component: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub summary: String,
    pub payload: JsonValue,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SystemLogStore {
    db_path: PathBuf,
}

impl SystemLogStore {
    pub fn new(paths: &RuntimePaths) -> std::io::Result<Self> {
        if let Some(parent) = paths.system_log_db().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: paths.system_log_db(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn append(&self, entry: SystemLogWrite) -> std::io::Result<SystemLogEntry> {
        let connection = self.open()?;
        let created_at = entry.created_at.unwrap_or_else(now_iso);
        let id = format!("slog-{}", Uuid::new_v4());
        let payload_json = serde_json::to_string(&entry.payload).map_err(std::io::Error::other)?;
        connection
            .execute(
                "INSERT INTO system_log
                (id, event, level, component, source_kind, source_id, summary, payload_json, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                params![
                    id,
                    entry.event,
                    entry.level,
                    entry.component,
                    entry.source_kind,
                    entry.source_id,
                    entry.summary,
                    payload_json,
                    created_at,
                ],
            )
            .map_err(std::io::Error::other)?;
        let seq = connection.last_insert_rowid();
        Ok(SystemLogEntry {
            id,
            seq,
            event: entry.event,
            level: entry.level,
            component: entry.component,
            source_kind: entry.source_kind,
            source_id: entry.source_id,
            summary: entry.summary,
            payload: entry.payload,
            created_at,
        })
    }

    pub fn list(&self, query: &SystemLogQuery) -> std::io::Result<Vec<SystemLogEntry>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT seq, id, event, level, component, source_kind, source_id, summary, payload_json, created_at
             FROM system_log WHERE 1=1",
        );
        let mut params_vec = Vec::<rusqlite::types::Value>::new();

        if let Some(event) = &query.event {
            sql.push_str(" AND event = ?");
            params_vec.push(event.clone().into());
        }
        if let Some(level) = &query.level {
            sql.push_str(" AND lower(level) = lower(?)");
            params_vec.push(level.clone().into());
        }
        if let Some(component) = &query.component {
            sql.push_str(" AND component = ?");
            params_vec.push(component.clone().into());
        }
        if let Some(source_kind) = &query.source_kind {
            sql.push_str(" AND source_kind = ?");
            params_vec.push(source_kind.clone().into());
        }
        if let Some(source_id) = &query.source_id {
            sql.push_str(" AND source_id = ?");
            params_vec.push(source_id.clone().into());
        }
        if let Some(before_seq) = query.before_seq {
            sql.push_str(" AND seq < ?");
            params_vec.push(before_seq.into());
        }
        sql.push_str(" ORDER BY seq DESC LIMIT ?");
        params_vec.push((query.limit.max(1) as i64).into());

        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_vec), map_system_log_entry)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn get(&self, id: &str) -> std::io::Result<Option<SystemLogEntry>> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT seq, id, event, level, component, source_kind, source_id, summary, payload_json, created_at
                 FROM system_log WHERE id = ?1",
                params![id],
                map_system_log_entry,
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
        let connection = self.open()?;
        connection
            .execute_batch(SYSTEM_LOG_SCHEMA_SQL)
            .map_err(std::io::Error::other)
    }
}

fn map_system_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<SystemLogEntry> {
    let payload_json: String = row.get("payload_json")?;
    Ok(SystemLogEntry {
        seq: row.get("seq")?,
        id: row.get("id")?,
        event: row.get("event")?,
        level: row.get("level")?,
        component: row.get("component")?,
        source_kind: row.get("source_kind")?,
        source_id: row.get("source_id")?,
        summary: row.get("summary")?,
        payload: serde_json::from_str(&payload_json).unwrap_or(JsonValue::Null),
        created_at: row.get("created_at")?,
    })
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
