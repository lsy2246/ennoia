use crate::logs_store::{
    LogEntry, LogEntryQuery, LogTraceLinkQuery, LogTraceLinkRecord, LogTraceQuery, LogTraceRecord,
    LogsOverview,
};

use super::*;
use crate::app::live_server_config;

#[derive(Debug, Serialize)]
struct LogStreamPayload {
    overview: LogsOverview,
    logs: Vec<LogEntry>,
    traces: Vec<LogTraceRecord>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LogEntriesQuery {
    #[serde(default)]
    event: Option<String>,
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    source_id: Option<String>,
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    trace_id: Option<String>,
    #[serde(default)]
    cursor: Option<i64>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct LogTracesQuery {
    #[serde(default)]
    request_id: Option<String>,
    #[serde(default)]
    component: Option<String>,
    #[serde(default)]
    kind: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    source_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Serialize)]
pub(super) struct LogTraceDetail {
    trace_id: String,
    spans: Vec<LogTraceRecord>,
    links: Vec<LogTraceLinkRecord>,
}

pub(super) async fn logs_overview(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<LogsOverview> {
    state
        .logs
        .overview()
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn log_entries(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<LogEntriesQuery>,
) -> ApiResult<Vec<LogEntry>> {
    state
        .logs
        .list_logs(&LogEntryQuery {
            event: query.event,
            level: query.level,
            component: query.component,
            source_kind: query.source_kind,
            source_id: query.source_id,
            request_id: query.request_id,
            trace_id: query.trace_id,
            before_seq: query.cursor,
            after_seq: None,
            limit: query.limit.unwrap_or(50),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn log_entry_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(log_id): Path<String>,
) -> ApiResult<LogEntry> {
    state
        .logs
        .get_log(&log_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("log entry not found"), &request))
}

pub(super) async fn log_traces(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<LogTracesQuery>,
) -> ApiResult<Vec<LogTraceRecord>> {
    state
        .logs
        .list_spans(&LogTraceQuery {
            trace_id: None,
            request_id: query.request_id,
            component: query.component,
            kind: query.kind,
            source_kind: query.source_kind,
            source_id: query.source_id,
            after_seq: None,
            limit: query.limit.unwrap_or(100),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn log_trace_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(trace_id): Path<String>,
) -> ApiResult<LogTraceDetail> {
    let spans = state
        .logs
        .list_spans(&LogTraceQuery {
            trace_id: Some(trace_id.clone()),
            request_id: None,
            component: None,
            kind: None,
            source_kind: None,
            source_id: None,
            after_seq: None,
            limit: 500,
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let links = state
        .logs
        .list_span_links(&LogTraceLinkQuery {
            trace_id: Some(trace_id.clone()),
            span_id: None,
            limit: 500,
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(LogTraceDetail {
        trace_id,
        spans,
        links,
    }))
}

pub(super) async fn logs_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let logs_store = state.logs.clone();
    let stream = async_stream::stream! {
        let mut last_log_seq = latest_log_seq(&logs_store).unwrap_or(0);
        let mut last_span_seq = latest_span_seq(&logs_store).unwrap_or(0);
        loop {
            tokio::time::sleep(Duration::from_millis(
                live_server_config(&state).streams.logs_poll_ms,
            )).await;

            let next_logs = logs_store.list_logs(&LogEntryQuery {
                after_seq: Some(last_log_seq),
                limit: 200,
                ..LogEntryQuery::default()
            });
            let next_traces = logs_store.list_spans(&LogTraceQuery {
                after_seq: Some(last_span_seq),
                limit: 200,
                ..LogTraceQuery::default()
            });

            match (next_logs, next_traces) {
                (Ok(logs), Ok(traces)) => {
                    if logs.is_empty() && traces.is_empty() {
                        continue;
                    }
                    if let Some(last) = logs.last() {
                        last_log_seq = last.seq;
                    }
                    if let Some(last) = traces.last() {
                        last_span_seq = last.seq;
                    }
                    match logs_store.overview() {
                        Ok(overview) => {
                            let payload = LogStreamPayload { overview, logs, traces };
                            yield Ok(Event::default().event("logs.delta").data(
                                serde_json::to_string(&payload).unwrap_or_else(|_| "{}".to_string())
                            ));
                        }
                        Err(error) => {
                            yield Ok(Event::default().event("logs.error").data(error.to_string()));
                        }
                    }
                }
                (Err(error), _) | (_, Err(error)) => {
                    yield Ok(Event::default().event("logs.error").data(error.to_string()));
                }
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn latest_log_seq(store: &crate::logs_store::LogsStore) -> std::io::Result<i64> {
    Ok(store
        .list_logs(&LogEntryQuery {
            limit: 1,
            ..LogEntryQuery::default()
        })?
        .first()
        .map(|item| item.seq)
        .unwrap_or(0))
}

fn latest_span_seq(store: &crate::logs_store::LogsStore) -> std::io::Result<i64> {
    Ok(store
        .list_spans(&LogTraceQuery {
            limit: 1,
            ..LogTraceQuery::default()
        })?
        .first()
        .map(|item| item.seq)
        .unwrap_or(0))
}
