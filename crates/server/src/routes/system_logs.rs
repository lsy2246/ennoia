use crate::system_log::{SystemLogEntry, SystemLogQuery};

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct SystemLogsQuery {
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
    cursor: Option<i64>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) async fn system_logs(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<SystemLogsQuery>,
) -> ApiResult<Vec<SystemLogEntry>> {
    state
        .system_log
        .list(&SystemLogQuery {
            event: query.event,
            level: query.level,
            component: query.component,
            source_kind: query.source_kind,
            source_id: query.source_id,
            before_seq: query.cursor,
            limit: query.limit.unwrap_or(50),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn system_log_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(log_id): Path<String>,
) -> ApiResult<SystemLogEntry> {
    state
        .system_log
        .get(&log_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("system log not found"), &request))
}
