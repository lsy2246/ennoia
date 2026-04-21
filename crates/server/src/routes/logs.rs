use super::*;

pub(super) async fn logs_list(
    State(state): State<AppState>,
    Query(query): Query<LogsQuery>,
) -> Json<Vec<LogRecordRow>> {
    Json(
        db::list_recent_logs(
            &state.pool,
            query.limit.unwrap_or(50),
            query.q.as_deref(),
            query.level.as_deref(),
            query.source.as_deref(),
        )
        .await
        .unwrap_or_default(),
    )
}

pub(super) async fn frontend_log_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<FrontendLogPayload>,
) -> Result<StatusCode, ApiError> {
    let id = format!("flog-{}", Uuid::new_v4());
    let at = payload.at.unwrap_or_else(now_iso);
    let source = payload
        .source
        .unwrap_or_else(|| "frontend".to_string())
        .trim()
        .to_string();
    let source = if source.is_empty() {
        "frontend".to_string()
    } else {
        source
    };

    db::insert_frontend_log(
        &state.pool,
        &id,
        &payload.level,
        &source,
        &payload.title,
        &payload.summary,
        payload.details.as_deref(),
        &at,
    )
    .await
    .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    Ok(StatusCode::NO_CONTENT)
}
