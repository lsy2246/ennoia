use std::path::PathBuf;

use chrono::Utc;
use ennoia_observability::TraceContext;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

pub const OBSERVABILITY_COMPONENT_HOST: &str = "host";
pub const OBSERVABILITY_COMPONENT_EXTENSION_HOST: &str = "extension_host";
pub const OBSERVABILITY_COMPONENT_BEHAVIOR: &str = "behavior_router";
pub const OBSERVABILITY_COMPONENT_PROXY: &str = "extension_proxy";
pub const OBSERVABILITY_COMPONENT_EVENT_BUS: &str = "event_bus";

const OBSERVABILITY_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS logs (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  event TEXT NOT NULL,
  level TEXT NOT NULL,
  component TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_id TEXT,
  request_id TEXT,
  trace_id TEXT,
  span_id TEXT,
  parent_span_id TEXT,
  message TEXT NOT NULL,
  attributes_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_logs_event_time
  ON logs(event, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_logs_component_time
  ON logs(component, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_logs_source_time
  ON logs(source_kind, source_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_logs_trace_time
  ON logs(trace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS spans (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  trace_id TEXT NOT NULL,
  span_id TEXT NOT NULL,
  parent_span_id TEXT,
  request_id TEXT NOT NULL,
  sampled INTEGER NOT NULL DEFAULT 1,
  source TEXT NOT NULL,
  kind TEXT NOT NULL,
  name TEXT NOT NULL,
  component TEXT NOT NULL,
  source_kind TEXT NOT NULL,
  source_id TEXT,
  status TEXT NOT NULL,
  attributes_json TEXT NOT NULL,
  started_at TEXT NOT NULL,
  ended_at TEXT NOT NULL,
  duration_ms INTEGER NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_spans_trace_seq
  ON spans(trace_id, seq ASC);
CREATE INDEX IF NOT EXISTS idx_spans_request_seq
  ON spans(request_id, seq ASC);
CREATE INDEX IF NOT EXISTS idx_spans_component_time
  ON spans(component, ended_at DESC);

CREATE TABLE IF NOT EXISTS span_links (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  trace_id TEXT NOT NULL,
  span_id TEXT NOT NULL,
  linked_trace_id TEXT NOT NULL,
  linked_span_id TEXT NOT NULL,
  link_type TEXT NOT NULL,
  attributes_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_span_links_trace_seq
  ON span_links(trace_id, seq ASC);
CREATE INDEX IF NOT EXISTS idx_span_links_span
  ON span_links(span_id, linked_span_id, link_type);
"#;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationLogEntry {
    pub id: String,
    pub seq: i64,
    pub event: String,
    pub level: String,
    pub component: String,
    pub source_kind: String,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub request_id: Option<String>,
    #[serde(default)]
    pub trace_id: Option<String>,
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    pub message: String,
    #[serde(default)]
    pub attributes: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObservationLogQuery {
    pub event: Option<String>,
    pub level: Option<String>,
    pub component: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub request_id: Option<String>,
    pub trace_id: Option<String>,
    pub before_seq: Option<i64>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct ObservationLogWrite {
    pub event: String,
    pub level: String,
    pub component: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub message: String,
    pub attributes: JsonValue,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationSpanRecord {
    pub id: String,
    pub seq: i64,
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    pub request_id: String,
    pub sampled: bool,
    pub source: String,
    pub kind: String,
    pub name: String,
    pub component: String,
    pub source_kind: String,
    #[serde(default)]
    pub source_id: Option<String>,
    pub status: String,
    #[serde(default)]
    pub attributes: JsonValue,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Default)]
pub struct ObservationSpanQuery {
    pub trace_id: Option<String>,
    pub request_id: Option<String>,
    pub component: Option<String>,
    pub kind: Option<String>,
    pub source_kind: Option<String>,
    pub source_id: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct ObservationSpanWrite {
    pub trace: TraceContext,
    pub kind: String,
    pub name: String,
    pub component: String,
    pub source_kind: String,
    pub source_id: Option<String>,
    pub status: String,
    pub attributes: JsonValue,
    pub started_at: String,
    pub ended_at: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ObservationSpanLinkRecord {
    pub id: String,
    pub seq: i64,
    pub trace_id: String,
    pub span_id: String,
    pub linked_trace_id: String,
    pub linked_span_id: String,
    pub link_type: String,
    #[serde(default)]
    pub attributes: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct ObservationLinkQuery {
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub limit: usize,
}

#[derive(Debug, Clone)]
pub struct ObservationSpanLinkWrite {
    pub trace_id: String,
    pub span_id: String,
    pub linked_trace_id: String,
    pub linked_span_id: String,
    pub link_type: String,
    pub attributes: JsonValue,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ObservationOverview {
    pub log_count: i64,
    pub span_count: i64,
    pub trace_count: i64,
}

#[derive(Debug, Clone)]
pub struct ObservabilityStore {
    db_path: PathBuf,
}

impl ObservabilityStore {
    pub fn new(paths: &RuntimePaths) -> std::io::Result<Self> {
        if let Some(parent) = paths.observability_db().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: paths.observability_db(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn append_log(&self, entry: ObservationLogWrite) -> std::io::Result<ObservationLogEntry> {
        self.append_log_scoped(entry, None)
    }

    pub fn append_log_scoped(
        &self,
        entry: ObservationLogWrite,
        trace: Option<&TraceContext>,
    ) -> std::io::Result<ObservationLogEntry> {
        let connection = self.open()?;
        let created_at = entry.created_at.unwrap_or_else(now_iso);
        let id = format!("log-{}", Uuid::new_v4());
        let attributes_json =
            serde_json::to_string(&entry.attributes).map_err(std::io::Error::other)?;
        connection
            .execute(
                "INSERT INTO logs
                (id, event, level, component, source_kind, source_id, request_id, trace_id, span_id, parent_span_id, message, attributes_json, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
                params![
                    id,
                    entry.event,
                    entry.level,
                    entry.component,
                    entry.source_kind,
                    entry.source_id,
                    trace.map(|item| item.request_id.clone()),
                    trace.map(|item| item.trace_id.clone()),
                    trace.map(|item| item.span_id.clone()),
                    trace.and_then(|item| item.parent_span_id.clone()),
                    entry.message,
                    attributes_json,
                    created_at,
                ],
            )
            .map_err(std::io::Error::other)?;
        let seq = connection.last_insert_rowid();
        Ok(ObservationLogEntry {
            id,
            seq,
            event: entry.event,
            level: entry.level,
            component: entry.component,
            source_kind: entry.source_kind,
            source_id: entry.source_id,
            request_id: trace.map(|item| item.request_id.clone()),
            trace_id: trace.map(|item| item.trace_id.clone()),
            span_id: trace.map(|item| item.span_id.clone()),
            parent_span_id: trace.and_then(|item| item.parent_span_id.clone()),
            message: entry.message,
            attributes: entry.attributes,
            created_at,
        })
    }

    pub fn list_logs(
        &self,
        query: &ObservationLogQuery,
    ) -> std::io::Result<Vec<ObservationLogEntry>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT seq, id, event, level, component, source_kind, source_id, request_id, trace_id, span_id, parent_span_id, message, attributes_json, created_at
             FROM logs WHERE 1=1",
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
        if let Some(request_id) = &query.request_id {
            sql.push_str(" AND request_id = ?");
            params_vec.push(request_id.clone().into());
        }
        if let Some(trace_id) = &query.trace_id {
            sql.push_str(" AND trace_id = ?");
            params_vec.push(trace_id.clone().into());
        }
        if let Some(before_seq) = query.before_seq {
            sql.push_str(" AND seq < ?");
            params_vec.push(before_seq.into());
        }
        sql.push_str(" ORDER BY seq DESC LIMIT ?");
        params_vec.push((query.limit.max(1) as i64).into());

        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_vec), map_log_entry)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn get_log(&self, id: &str) -> std::io::Result<Option<ObservationLogEntry>> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT seq, id, event, level, component, source_kind, source_id, request_id, trace_id, span_id, parent_span_id, message, attributes_json, created_at
                 FROM logs WHERE id = ?1",
                params![id],
                map_log_entry,
            )
            .optional()
            .map_err(std::io::Error::other)
    }

    pub fn append_span(
        &self,
        entry: ObservationSpanWrite,
    ) -> std::io::Result<ObservationSpanRecord> {
        let connection = self.open()?;
        let id = format!("span-{}", Uuid::new_v4());
        let attributes_json =
            serde_json::to_string(&entry.attributes).map_err(std::io::Error::other)?;
        connection
            .execute(
                "INSERT INTO spans
                (id, trace_id, span_id, parent_span_id, request_id, sampled, source, kind, name, component, source_kind, source_id, status, attributes_json, started_at, ended_at, duration_ms)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
                params![
                    id,
                    entry.trace.trace_id,
                    entry.trace.span_id,
                    entry.trace.parent_span_id,
                    entry.trace.request_id,
                    if entry.trace.sampled { 1 } else { 0 },
                    entry.trace.source,
                    entry.kind,
                    entry.name,
                    entry.component,
                    entry.source_kind,
                    entry.source_id,
                    entry.status,
                    attributes_json,
                    entry.started_at,
                    entry.ended_at,
                    entry.duration_ms,
                ],
            )
            .map_err(std::io::Error::other)?;
        let seq = connection.last_insert_rowid();
        Ok(ObservationSpanRecord {
            id,
            seq,
            trace_id: entry.trace.trace_id,
            span_id: entry.trace.span_id,
            parent_span_id: entry.trace.parent_span_id,
            request_id: entry.trace.request_id,
            sampled: entry.trace.sampled,
            source: entry.trace.source,
            kind: entry.kind,
            name: entry.name,
            component: entry.component,
            source_kind: entry.source_kind,
            source_id: entry.source_id,
            status: entry.status,
            attributes: entry.attributes,
            started_at: entry.started_at,
            ended_at: entry.ended_at,
            duration_ms: entry.duration_ms,
        })
    }

    pub fn list_spans(
        &self,
        query: &ObservationSpanQuery,
    ) -> std::io::Result<Vec<ObservationSpanRecord>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT seq, id, trace_id, span_id, parent_span_id, request_id, sampled, source, kind, name, component, source_kind, source_id, status, attributes_json, started_at, ended_at, duration_ms
             FROM spans WHERE 1=1",
        );
        let mut params_vec = Vec::<rusqlite::types::Value>::new();

        if let Some(trace_id) = &query.trace_id {
            sql.push_str(" AND trace_id = ?");
            params_vec.push(trace_id.clone().into());
        }
        if let Some(request_id) = &query.request_id {
            sql.push_str(" AND request_id = ?");
            params_vec.push(request_id.clone().into());
        }
        if let Some(component) = &query.component {
            sql.push_str(" AND component = ?");
            params_vec.push(component.clone().into());
        }
        if let Some(kind) = &query.kind {
            sql.push_str(" AND kind = ?");
            params_vec.push(kind.clone().into());
        }
        if let Some(source_kind) = &query.source_kind {
            sql.push_str(" AND source_kind = ?");
            params_vec.push(source_kind.clone().into());
        }
        if let Some(source_id) = &query.source_id {
            sql.push_str(" AND source_id = ?");
            params_vec.push(source_id.clone().into());
        }
        if query.trace_id.is_some() || query.request_id.is_some() {
            sql.push_str(" ORDER BY seq ASC LIMIT ?");
        } else {
            sql.push_str(" ORDER BY seq DESC LIMIT ?");
        }
        params_vec.push((query.limit.max(1) as i64).into());

        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_vec), map_span_record)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn append_span_link(
        &self,
        entry: ObservationSpanLinkWrite,
    ) -> std::io::Result<ObservationSpanLinkRecord> {
        let connection = self.open()?;
        let id = format!("link-{}", Uuid::new_v4());
        let created_at = entry.created_at.unwrap_or_else(now_iso);
        let attributes_json =
            serde_json::to_string(&entry.attributes).map_err(std::io::Error::other)?;
        connection
            .execute(
                "INSERT INTO span_links
                (id, trace_id, span_id, linked_trace_id, linked_span_id, link_type, attributes_json, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![
                    id,
                    entry.trace_id,
                    entry.span_id,
                    entry.linked_trace_id,
                    entry.linked_span_id,
                    entry.link_type,
                    attributes_json,
                    created_at,
                ],
            )
            .map_err(std::io::Error::other)?;
        let seq = connection.last_insert_rowid();
        Ok(ObservationSpanLinkRecord {
            id,
            seq,
            trace_id: entry.trace_id,
            span_id: entry.span_id,
            linked_trace_id: entry.linked_trace_id,
            linked_span_id: entry.linked_span_id,
            link_type: entry.link_type,
            attributes: entry.attributes,
            created_at,
        })
    }

    pub fn list_span_links(
        &self,
        query: &ObservationLinkQuery,
    ) -> std::io::Result<Vec<ObservationSpanLinkRecord>> {
        let connection = self.open()?;
        let mut sql = String::from(
            "SELECT seq, id, trace_id, span_id, linked_trace_id, linked_span_id, link_type, attributes_json, created_at
             FROM span_links WHERE 1=1",
        );
        let mut params_vec = Vec::<rusqlite::types::Value>::new();

        if let Some(trace_id) = &query.trace_id {
            sql.push_str(" AND trace_id = ?");
            params_vec.push(trace_id.clone().into());
        }
        if let Some(span_id) = &query.span_id {
            sql.push_str(" AND span_id = ?");
            params_vec.push(span_id.clone().into());
        }
        sql.push_str(" ORDER BY seq ASC LIMIT ?");
        params_vec.push((query.limit.max(1) as i64).into());

        let mut statement = connection.prepare(&sql).map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(params_vec), map_span_link_record)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn overview(&self) -> std::io::Result<ObservationOverview> {
        let connection = self.open()?;
        let log_count = query_count(&connection, "SELECT COUNT(*) FROM logs")?;
        let span_count = query_count(&connection, "SELECT COUNT(*) FROM spans")?;
        let trace_count = query_count(&connection, "SELECT COUNT(DISTINCT trace_id) FROM spans")?;
        Ok(ObservationOverview {
            log_count,
            span_count,
            trace_count,
        })
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
            .execute_batch(OBSERVABILITY_SCHEMA_SQL)
            .map_err(std::io::Error::other)
    }
}

fn map_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationLogEntry> {
    let attributes_json: String = row.get("attributes_json")?;
    Ok(ObservationLogEntry {
        seq: row.get("seq")?,
        id: row.get("id")?,
        event: row.get("event")?,
        level: row.get("level")?,
        component: row.get("component")?,
        source_kind: row.get("source_kind")?,
        source_id: row.get("source_id")?,
        request_id: row.get("request_id")?,
        trace_id: row.get("trace_id")?,
        span_id: row.get("span_id")?,
        parent_span_id: row.get("parent_span_id")?,
        message: row.get("message")?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        created_at: row.get("created_at")?,
    })
}

fn map_span_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationSpanRecord> {
    let attributes_json: String = row.get("attributes_json")?;
    Ok(ObservationSpanRecord {
        seq: row.get("seq")?,
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        span_id: row.get("span_id")?,
        parent_span_id: row.get("parent_span_id")?,
        request_id: row.get("request_id")?,
        sampled: row.get::<_, i64>("sampled")? != 0,
        source: row.get("source")?,
        kind: row.get("kind")?,
        name: row.get("name")?,
        component: row.get("component")?,
        source_kind: row.get("source_kind")?,
        source_id: row.get("source_id")?,
        status: row.get("status")?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        started_at: row.get("started_at")?,
        ended_at: row.get("ended_at")?,
        duration_ms: row.get("duration_ms")?,
    })
}

fn map_span_link_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationSpanLinkRecord> {
    let attributes_json: String = row.get("attributes_json")?;
    Ok(ObservationSpanLinkRecord {
        seq: row.get("seq")?,
        id: row.get("id")?,
        trace_id: row.get("trace_id")?,
        span_id: row.get("span_id")?,
        linked_trace_id: row.get("linked_trace_id")?,
        linked_span_id: row.get("linked_span_id")?,
        link_type: row.get("link_type")?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        created_at: row.get("created_at")?,
    })
}

fn query_count(connection: &Connection, sql: &str) -> std::io::Result<i64> {
    connection
        .query_row(sql, [], |row| row.get::<_, i64>(0))
        .map_err(std::io::Error::other)
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
