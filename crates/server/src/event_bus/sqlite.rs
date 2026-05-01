use std::path::PathBuf;

use chrono::{Duration, Utc};
use ennoia_extension_host::RegisteredHookContribution;
use ennoia_kernel::HookEventEnvelope;
use ennoia_observability::TraceContext;
use ennoia_paths::RuntimePaths;
use rusqlite::{params, Connection, OptionalExtension};
use uuid::Uuid;

const EVENT_BUS_SCHEMA_SQL: &str = r#"
CREATE TABLE IF NOT EXISTS hook_events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  event TEXT NOT NULL,
  resource_kind TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  conversation_id TEXT,
  lane_id TEXT,
  run_id TEXT,
  request_id TEXT,
  trace_id TEXT,
  span_id TEXT,
  parent_span_id TEXT,
  sampled INTEGER NOT NULL DEFAULT 1,
  source TEXT NOT NULL DEFAULT 'event_bus',
  envelope_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_hook_events_event_time
  ON hook_events(event, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_hook_events_resource
  ON hook_events(resource_kind, resource_id, created_at DESC);
CREATE INDEX IF NOT EXISTS idx_hook_events_trace_time
  ON hook_events(trace_id, created_at DESC);

CREATE TABLE IF NOT EXISTS hook_deliveries (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  event_id TEXT NOT NULL,
  extension_id TEXT NOT NULL,
  handler TEXT NOT NULL,
  status TEXT NOT NULL,
  attempt_count INTEGER NOT NULL DEFAULT 0,
  last_error TEXT,
  next_attempt_at TEXT NOT NULL,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_hook_deliveries_pending
  ON hook_deliveries(status, next_attempt_at, extension_id);
"#;

#[derive(Debug, Clone)]
pub struct HookEventWrite {
    pub envelope: HookEventEnvelope,
    pub hooks: Vec<RegisteredHookContribution>,
    pub trace: TraceContext,
}

#[derive(Debug, Clone)]
pub struct HookDeliveryRecord {
    pub id: String,
    pub event_id: String,
    pub extension_id: String,
    pub handler: String,
    pub attempt_count: u32,
    pub envelope: HookEventEnvelope,
    pub trace: TraceContext,
}

#[derive(Debug, Clone)]
pub struct EventBusStore {
    db_path: PathBuf,
}

impl EventBusStore {
    pub fn new(paths: &RuntimePaths) -> std::io::Result<Self> {
        if let Some(parent) = paths.system_events_db().parent() {
            std::fs::create_dir_all(parent)?;
        }
        let store = Self {
            db_path: paths.system_events_db(),
        };
        store.ensure_schema()?;
        Ok(store)
    }

    pub fn publish(&self, entry: HookEventWrite) -> std::io::Result<String> {
        let connection = self.open()?;
        let HookEventWrite {
            envelope,
            hooks,
            trace,
        } = entry;
        let event_id = format!("hev-{}", Uuid::new_v4());
        let created_at = envelope.occurred_at.clone();
        let envelope_json = serde_json::to_string(&envelope).map_err(std::io::Error::other)?;

        connection
            .execute(
                "INSERT INTO hook_events
                (id, event, resource_kind, resource_id, conversation_id, lane_id, run_id, request_id, trace_id, span_id, parent_span_id, sampled, source, envelope_json, created_at)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
                params![
                    event_id,
                    envelope.event,
                    envelope.resource.kind,
                    envelope.resource.id,
                    envelope.resource.conversation_id,
                    envelope.resource.lane_id,
                    envelope.resource.run_id,
                    trace.request_id,
                    trace.trace_id,
                    trace.span_id,
                    trace.parent_span_id,
                    if trace.sampled { 1 } else { 0 },
                    trace.source,
                    envelope_json,
                    created_at,
                ],
            )
            .map_err(std::io::Error::other)?;

        for hook in hooks {
            let handler = hook
                .hook
                .handler
                .clone()
                .unwrap_or_else(|| default_hook_handler_path(&hook.hook.event));
            let delivery_id = format!("hdl-{}", Uuid::new_v4());
            let now = now_iso();
            connection
                .execute(
                    "INSERT INTO hook_deliveries
                    (id, event_id, extension_id, handler, status, attempt_count, last_error, next_attempt_at, created_at, updated_at)
                    VALUES (?1, ?2, ?3, ?4, 'pending', 0, NULL, ?5, ?6, ?6)",
                    params![delivery_id, event_id, hook.extension_id, handler, now, now],
                )
                .map_err(std::io::Error::other)?;
        }

        Ok(event_id)
    }

    pub fn list_pending_deliveries(
        &self,
        limit: usize,
    ) -> std::io::Result<Vec<HookDeliveryRecord>> {
        let connection = self.open()?;
        let mut statement = connection
            .prepare(
                "SELECT d.id, d.event_id, d.extension_id, d.handler, d.attempt_count, e.envelope_json, e.request_id, e.trace_id, e.span_id, e.parent_span_id, e.sampled, e.source
                 FROM hook_deliveries d
                 JOIN hook_events e ON e.id = d.event_id
                 WHERE d.status = 'pending' AND d.next_attempt_at <= ?1
                 ORDER BY d.seq ASC
                 LIMIT ?2",
            )
            .map_err(std::io::Error::other)?;
        let rows = statement
            .query_map(params![now_iso(), limit.max(1) as i64], |row| {
                let envelope_json: String = row.get("envelope_json")?;
                let envelope =
                    serde_json::from_str::<HookEventEnvelope>(&envelope_json).map_err(|error| {
                        rusqlite::Error::FromSqlConversionFailure(
                            envelope_json.len(),
                            rusqlite::types::Type::Text,
                            Box::new(error),
                        )
                    })?;
                Ok(HookDeliveryRecord {
                    id: row.get("id")?,
                    event_id: row.get("event_id")?,
                    extension_id: row.get("extension_id")?,
                    handler: row.get("handler")?,
                    attempt_count: row.get::<_, i64>("attempt_count")? as u32,
                    envelope,
                    trace: TraceContext {
                        request_id: row.get("request_id")?,
                        trace_id: row.get("trace_id")?,
                        span_id: row.get("span_id")?,
                        parent_span_id: row.get("parent_span_id")?,
                        sampled: row.get::<_, i64>("sampled")? != 0,
                        source: row.get("source")?,
                    },
                })
            })
            .map_err(std::io::Error::other)?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(std::io::Error::other)
    }

    pub fn mark_delivery_succeeded(&self, delivery_id: &str) -> std::io::Result<()> {
        self.open()?
            .execute(
                "UPDATE hook_deliveries
                 SET status = 'succeeded', updated_at = ?2, last_error = NULL
                 WHERE id = ?1",
                params![delivery_id, now_iso()],
            )
            .map(|_| ())
            .map_err(std::io::Error::other)
    }

    pub fn mark_delivery_retry(
        &self,
        delivery_id: &str,
        error: &str,
        previous_attempts: u32,
    ) -> std::io::Result<bool> {
        let connection = self.open()?;
        let next_attempts = previous_attempts.saturating_add(1);
        let terminal = next_attempts >= 20;
        let updated_at = now_iso();
        let next_attempt_at = if terminal {
            updated_at.clone()
        } else {
            (Utc::now() + Duration::seconds(backoff_seconds(next_attempts))).to_rfc3339()
        };
        connection
            .execute(
                "UPDATE hook_deliveries
                 SET status = ?2,
                     attempt_count = ?3,
                     last_error = ?4,
                     next_attempt_at = ?5,
                     updated_at = ?6
                 WHERE id = ?1",
                params![
                    delivery_id,
                    if terminal { "failed" } else { "pending" },
                    next_attempts as i64,
                    error,
                    next_attempt_at,
                    updated_at,
                ],
            )
            .map_err(std::io::Error::other)?;
        Ok(terminal)
    }

    pub fn get_delivery(&self, delivery_id: &str) -> std::io::Result<Option<HookDeliveryRecord>> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT d.id, d.event_id, d.extension_id, d.handler, d.attempt_count, e.envelope_json, e.request_id, e.trace_id, e.span_id, e.parent_span_id, e.sampled, e.source
                 FROM hook_deliveries d
                 JOIN hook_events e ON e.id = d.event_id
                 WHERE d.id = ?1",
                params![delivery_id],
                |row| {
                    let envelope_json: String = row.get("envelope_json")?;
                    let envelope = serde_json::from_str::<HookEventEnvelope>(&envelope_json)
                        .map_err(|error| {
                            rusqlite::Error::FromSqlConversionFailure(
                                envelope_json.len(),
                                rusqlite::types::Type::Text,
                                Box::new(error),
                            )
                        })?;
                    Ok(HookDeliveryRecord {
                        id: row.get("id")?,
                        event_id: row.get("event_id")?,
                        extension_id: row.get("extension_id")?,
                        handler: row.get("handler")?,
                        attempt_count: row.get::<_, i64>("attempt_count")? as u32,
                        envelope,
                        trace: TraceContext {
                            request_id: row.get("request_id")?,
                            trace_id: row.get("trace_id")?,
                            span_id: row.get("span_id")?,
                            parent_span_id: row.get("parent_span_id")?,
                            sampled: row.get::<_, i64>("sampled")? != 0,
                            source: row.get("source")?,
                        },
                    })
                },
            )
            .optional()
            .map_err(std::io::Error::other)
    }

    pub fn latest_conversation_seq(&self, conversation_id: &str) -> std::io::Result<i64> {
        let connection = self.open()?;
        connection
            .query_row(
                "SELECT seq
                 FROM hook_events
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

    fn open(&self) -> std::io::Result<Connection> {
        let connection = Connection::open(&self.db_path).map_err(std::io::Error::other)?;
        connection
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(std::io::Error::other)?;
        Ok(connection)
    }

    fn ensure_schema(&self) -> std::io::Result<()> {
        let connection = self.open()?;
        rebuild_legacy_schema_if_needed(&connection)?;
        connection
            .execute_batch(EVENT_BUS_SCHEMA_SQL)
            .map_err(std::io::Error::other)
    }
}

fn rebuild_legacy_schema_if_needed(connection: &Connection) -> std::io::Result<()> {
    let hook_events_exists = table_exists(connection, "hook_events")?;
    let hook_deliveries_exists = table_exists(connection, "hook_deliveries")?;
    if !hook_events_exists && !hook_deliveries_exists {
        return Ok(());
    }

    let hook_events_valid = !hook_events_exists
        || table_has_columns(
            connection,
            "hook_events",
            &[
                "id",
                "event",
                "resource_kind",
                "resource_id",
                "request_id",
                "trace_id",
                "span_id",
                "parent_span_id",
                "sampled",
                "source",
                "envelope_json",
                "created_at",
            ],
        )?;
    let hook_deliveries_valid = !hook_deliveries_exists
        || table_has_columns(
            connection,
            "hook_deliveries",
            &[
                "id",
                "event_id",
                "extension_id",
                "handler",
                "status",
                "attempt_count",
                "last_error",
                "next_attempt_at",
                "created_at",
                "updated_at",
            ],
        )?;

    if hook_events_valid && hook_deliveries_valid {
        return Ok(());
    }

    connection
        .execute_batch(
            "
DROP TABLE IF EXISTS hook_deliveries;
DROP TABLE IF EXISTS hook_events;
",
        )
        .map_err(std::io::Error::other)
}

fn table_exists(connection: &Connection, table: &str) -> std::io::Result<bool> {
    connection
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1 LIMIT 1",
            params![table],
            |_| Ok(()),
        )
        .optional()
        .map(|row| row.is_some())
        .map_err(std::io::Error::other)
}

fn table_has_columns(
    connection: &Connection,
    table: &str,
    required_columns: &[&str],
) -> std::io::Result<bool> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(std::io::Error::other)?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(std::io::Error::other)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(std::io::Error::other)?;
    Ok(required_columns
        .iter()
        .all(|required| columns.iter().any(|column| column == required)))
}

fn backoff_seconds(attempt: u32) -> i64 {
    match attempt {
        0 | 1 => 1,
        2 => 2,
        3 => 4,
        4 => 8,
        5 => 16,
        _ => 30,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

fn default_hook_handler_path(event: &str) -> String {
    format!("hooks/{}", event.replace('.', "/"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recreates_legacy_hook_events_schema_when_trace_columns_are_missing() {
        let home = std::env::temp_dir().join(format!("ennoia-event-bus-test-{}", Uuid::new_v4()));
        let paths = RuntimePaths::new(&home);
        std::fs::create_dir_all(paths.system_sqlite_dir()).unwrap();
        {
            let connection = Connection::open(paths.system_events_db()).unwrap();
            connection
                .execute_batch(
                    "
CREATE TABLE hook_events (
  seq INTEGER PRIMARY KEY AUTOINCREMENT,
  id TEXT NOT NULL UNIQUE,
  event TEXT NOT NULL,
  resource_kind TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  envelope_json TEXT NOT NULL,
  created_at TEXT NOT NULL
);
",
                )
                .unwrap();
        }

        {
            let store = EventBusStore::new(&paths).unwrap();
            let connection = store.open().unwrap();
            let mut statement = connection
                .prepare("PRAGMA table_info(hook_events)")
                .unwrap();
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .unwrap()
                .collect::<Result<Vec<_>, _>>()
                .unwrap();

            assert!(columns.iter().any(|column| column == "trace_id"));
            assert!(columns.iter().any(|column| column == "request_id"));
            assert!(table_exists(&connection, "hook_deliveries").unwrap());
        }

        std::fs::remove_dir_all(home).unwrap();
    }
}
