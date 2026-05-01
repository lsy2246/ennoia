mod query;
mod schema;

use std::path::PathBuf;

use chrono::Utc;
use ennoia_observability::TraceContext;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use self::query::{ColumnCount, FilterOperator, SelectQuery};
use self::schema::{LogsSchema, SpanLinksSchema, SpansSchema, TableSchema};

pub const OBSERVABILITY_COMPONENT_HOST: &str = "host";
pub const OBSERVABILITY_COMPONENT_EXTENSION_HOST: &str = "extension_host";
pub const OBSERVABILITY_COMPONENT_BEHAVIOR: &str = "behavior_router";
pub const OBSERVABILITY_COMPONENT_PROXY: &str = "extension_proxy";
pub const OBSERVABILITY_COMPONENT_EVENT_BUS: &str = "event_bus";

const SQLITE_PRAGMAS: &[&str] = &["PRAGMA journal_mode=WAL;", "PRAGMA synchronous=NORMAL;"];

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
    pub after_seq: Option<i64>,
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
    pub after_seq: Option<i64>,
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
                &LogsSchema::insert_statement(),
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
        let prepared = build_logs_query(query).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(prepared.params), map_log_entry)
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn get_log(&self, id: &str) -> std::io::Result<Option<ObservationLogEntry>> {
        let connection = self.open()?;
        connection
            .query_row(
                &LogsSchema::select_by_id_statement(),
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
                &SpansSchema::insert_statement(),
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
        let prepared = build_spans_query(query).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(rusqlite::params_from_iter(prepared.params), map_span_record)
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
                &SpanLinksSchema::insert_statement(),
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
        let prepared = build_span_links_query(query).build();
        let mut statement = prepared.prepare(&connection)?;
        let rows = statement
            .query_map(
                rusqlite::params_from_iter(prepared.params),
                map_span_link_record,
            )
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn overview(&self) -> std::io::Result<ObservationOverview> {
        let connection = self.open()?;
        let log_count = query_count(&connection, &LogsSchema::count_statement(ColumnCount::All))?;
        let span_count = query_count(&connection, &SpansSchema::count_statement(ColumnCount::All))?;
        let trace_count = query_count(
            &connection,
            &SpansSchema::count_statement(ColumnCount::Distinct(SpansSchema::TRACE_ID)),
        )?;
        Ok(ObservationOverview {
            log_count,
            span_count,
            trace_count,
        })
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
        for statement in schema::schema_statements() {
            connection
                .execute_batch(&statement)
                .map_err(std::io::Error::other)?;
        }
        Ok(())
    }
}

fn build_logs_query(query: &ObservationLogQuery) -> SelectQuery {
    let mut builder = SelectQuery::new(LogsSchema::NAME, LogsSchema::SELECT_COLUMNS);
    if let Some(event) = &query.event {
        builder.push_filter(LogsSchema::EVENT, FilterOperator::Eq, event.clone().into());
    }
    if let Some(level) = &query.level {
        builder.push_filter(
            LogsSchema::LEVEL,
            FilterOperator::EqIgnoreCase,
            level.clone().into(),
        );
    }
    if let Some(component) = &query.component {
        builder.push_filter(
            LogsSchema::COMPONENT,
            FilterOperator::Eq,
            component.clone().into(),
        );
    }
    if let Some(source_kind) = &query.source_kind {
        builder.push_filter(
            LogsSchema::SOURCE_KIND,
            FilterOperator::Eq,
            source_kind.clone().into(),
        );
    }
    if let Some(source_id) = &query.source_id {
        builder.push_filter(
            LogsSchema::SOURCE_ID,
            FilterOperator::Eq,
            source_id.clone().into(),
        );
    }
    if let Some(request_id) = &query.request_id {
        builder.push_filter(
            LogsSchema::REQUEST_ID,
            FilterOperator::Eq,
            request_id.clone().into(),
        );
    }
    if let Some(trace_id) = &query.trace_id {
        builder.push_filter(
            LogsSchema::TRACE_ID,
            FilterOperator::Eq,
            trace_id.clone().into(),
        );
    }
    if let Some(before_seq) = query.before_seq {
        builder.push_filter(LogsSchema::SEQ, FilterOperator::Lt, before_seq.into());
    }
    if let Some(after_seq) = query.after_seq {
        builder.push_filter(LogsSchema::SEQ, FilterOperator::Gt, after_seq.into());
    }
    let descending = query.after_seq.is_none();
    builder
        .order_by(LogsSchema::SEQ, descending)
        .limit(query.limit.max(1) as i64)
}

fn build_spans_query(query: &ObservationSpanQuery) -> SelectQuery {
    let mut builder = SelectQuery::new(SpansSchema::NAME, SpansSchema::SELECT_COLUMNS);
    if let Some(trace_id) = &query.trace_id {
        builder.push_filter(
            SpansSchema::TRACE_ID,
            FilterOperator::Eq,
            trace_id.clone().into(),
        );
    }
    if let Some(request_id) = &query.request_id {
        builder.push_filter(
            SpansSchema::REQUEST_ID,
            FilterOperator::Eq,
            request_id.clone().into(),
        );
    }
    if let Some(component) = &query.component {
        builder.push_filter(
            SpansSchema::COMPONENT,
            FilterOperator::Eq,
            component.clone().into(),
        );
    }
    if let Some(kind) = &query.kind {
        builder.push_filter(SpansSchema::KIND, FilterOperator::Eq, kind.clone().into());
    }
    if let Some(source_kind) = &query.source_kind {
        builder.push_filter(
            SpansSchema::SOURCE_KIND,
            FilterOperator::Eq,
            source_kind.clone().into(),
        );
    }
    if let Some(source_id) = &query.source_id {
        builder.push_filter(
            SpansSchema::SOURCE_ID,
            FilterOperator::Eq,
            source_id.clone().into(),
        );
    }
    if let Some(after_seq) = query.after_seq {
        builder.push_filter(SpansSchema::SEQ, FilterOperator::Gt, after_seq.into());
    }
    let descending =
        query.after_seq.is_none() && query.trace_id.is_none() && query.request_id.is_none();
    builder
        .order_by(SpansSchema::SEQ, descending)
        .limit(query.limit.max(1) as i64)
}

fn build_span_links_query(query: &ObservationLinkQuery) -> SelectQuery {
    let mut builder = SelectQuery::new(SpanLinksSchema::NAME, SpanLinksSchema::SELECT_COLUMNS);
    if let Some(trace_id) = &query.trace_id {
        builder.push_filter(
            SpanLinksSchema::TRACE_ID,
            FilterOperator::Eq,
            trace_id.clone().into(),
        );
    }
    if let Some(span_id) = &query.span_id {
        builder.push_filter(
            SpanLinksSchema::SPAN_ID,
            FilterOperator::Eq,
            span_id.clone().into(),
        );
    }
    builder
        .order_by(SpanLinksSchema::SEQ, false)
        .limit(query.limit.max(1) as i64)
}

fn map_log_entry(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationLogEntry> {
    let attributes_json: String = row.get(LogsSchema::ATTRIBUTES_JSON)?;
    Ok(ObservationLogEntry {
        seq: row.get(LogsSchema::SEQ)?,
        id: row.get(LogsSchema::ID)?,
        event: row.get(LogsSchema::EVENT)?,
        level: row.get(LogsSchema::LEVEL)?,
        component: row.get(LogsSchema::COMPONENT)?,
        source_kind: row.get(LogsSchema::SOURCE_KIND)?,
        source_id: row.get(LogsSchema::SOURCE_ID)?,
        request_id: row.get(LogsSchema::REQUEST_ID)?,
        trace_id: row.get(LogsSchema::TRACE_ID)?,
        span_id: row.get(LogsSchema::SPAN_ID)?,
        parent_span_id: row.get(LogsSchema::PARENT_SPAN_ID)?,
        message: row.get(LogsSchema::MESSAGE)?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        created_at: row.get(LogsSchema::CREATED_AT)?,
    })
}

fn map_span_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationSpanRecord> {
    let attributes_json: String = row.get(SpansSchema::ATTRIBUTES_JSON)?;
    Ok(ObservationSpanRecord {
        seq: row.get(SpansSchema::SEQ)?,
        id: row.get(SpansSchema::ID)?,
        trace_id: row.get(SpansSchema::TRACE_ID)?,
        span_id: row.get(SpansSchema::SPAN_ID)?,
        parent_span_id: row.get(SpansSchema::PARENT_SPAN_ID)?,
        request_id: row.get(SpansSchema::REQUEST_ID)?,
        sampled: row.get::<_, i64>(SpansSchema::SAMPLED)? != 0,
        source: row.get(SpansSchema::SOURCE)?,
        kind: row.get(SpansSchema::KIND)?,
        name: row.get(SpansSchema::NAME_COL)?,
        component: row.get(SpansSchema::COMPONENT)?,
        source_kind: row.get(SpansSchema::SOURCE_KIND)?,
        source_id: row.get(SpansSchema::SOURCE_ID)?,
        status: row.get(SpansSchema::STATUS)?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        started_at: row.get(SpansSchema::STARTED_AT)?,
        ended_at: row.get(SpansSchema::ENDED_AT)?,
        duration_ms: row.get(SpansSchema::DURATION_MS)?,
    })
}

fn map_span_link_record(row: &rusqlite::Row<'_>) -> rusqlite::Result<ObservationSpanLinkRecord> {
    let attributes_json: String = row.get(SpanLinksSchema::ATTRIBUTES_JSON)?;
    Ok(ObservationSpanLinkRecord {
        seq: row.get(SpanLinksSchema::SEQ)?,
        id: row.get(SpanLinksSchema::ID)?,
        trace_id: row.get(SpanLinksSchema::TRACE_ID)?,
        span_id: row.get(SpanLinksSchema::SPAN_ID)?,
        linked_trace_id: row.get(SpanLinksSchema::LINKED_TRACE_ID)?,
        linked_span_id: row.get(SpanLinksSchema::LINKED_SPAN_ID)?,
        link_type: row.get(SpanLinksSchema::LINK_TYPE)?,
        attributes: serde_json::from_str(&attributes_json).unwrap_or(JsonValue::Null),
        created_at: row.get(SpanLinksSchema::CREATED_AT)?,
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
