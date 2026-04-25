use crate::observability::{
    ObservationLinkQuery, ObservationLogEntry, ObservationLogQuery, ObservationOverview,
    ObservationSpanLinkRecord, ObservationSpanQuery, ObservationSpanRecord,
};

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct ObservabilityLogsQuery {
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
pub(super) struct ObservabilityTracesQuery {
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
pub(super) struct ObservabilityTraceDetail {
    trace_id: String,
    spans: Vec<ObservationSpanRecord>,
    links: Vec<ObservationSpanLinkRecord>,
}

pub(super) async fn observability_overview(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<ObservationOverview> {
    state
        .observability
        .overview()
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn observability_logs(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ObservabilityLogsQuery>,
) -> ApiResult<Vec<ObservationLogEntry>> {
    state
        .observability
        .list_logs(&ObservationLogQuery {
            event: query.event,
            level: query.level,
            component: query.component,
            source_kind: query.source_kind,
            source_id: query.source_id,
            request_id: query.request_id,
            trace_id: query.trace_id,
            before_seq: query.cursor,
            limit: query.limit.unwrap_or(50),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn observability_log_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(log_id): Path<String>,
) -> ApiResult<ObservationLogEntry> {
    state
        .observability
        .get_log(&log_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("observability log not found"), &request))
}

pub(super) async fn observability_traces(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<ObservabilityTracesQuery>,
) -> ApiResult<Vec<ObservationSpanRecord>> {
    state
        .observability
        .list_spans(&ObservationSpanQuery {
            trace_id: None,
            request_id: query.request_id,
            component: query.component,
            kind: query.kind,
            source_kind: query.source_kind,
            source_id: query.source_id,
            limit: query.limit.unwrap_or(100),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn observability_trace_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(trace_id): Path<String>,
) -> ApiResult<ObservabilityTraceDetail> {
    let spans = state
        .observability
        .list_spans(&ObservationSpanQuery {
            trace_id: Some(trace_id.clone()),
            request_id: None,
            component: None,
            kind: None,
            source_kind: None,
            source_id: None,
            limit: 500,
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let links = state
        .observability
        .list_span_links(&ObservationLinkQuery {
            trace_id: Some(trace_id.clone()),
            span_id: None,
            limit: 500,
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(ObservabilityTraceDetail {
        trace_id,
        spans,
        links,
    }))
}
