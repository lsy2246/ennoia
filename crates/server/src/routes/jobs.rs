use super::*;

pub(super) async fn jobs_list(State(state): State<AppState>) -> Json<Vec<JobRow>> {
    Json(db::list_jobs(&state.pool).await.unwrap_or_default())
}

pub(super) async fn jobs_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<CreateJobRequest>,
) -> ApiResult<JobRecord> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let enqueue = ennoia_scheduler::EnqueueRequest {
        owner,
        job_kind: JobKind::from_str(payload.job_kind.as_deref().unwrap_or("maintenance")),
        schedule_kind: ScheduleKind::from_str(&payload.schedule_kind),
        schedule_value: payload.schedule_value,
        payload: payload.payload.unwrap_or(JsonValue::Null),
        max_retries: payload.max_retries,
        run_at: payload.run_at,
    };
    state
        .scheduler_store
        .enqueue(enqueue)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

pub(super) async fn job_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
) -> ApiResult<JobDetailRow> {
    db::get_job(&state.pool, &job_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))
}

pub(super) async fn job_update(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
    Json(payload): Json<UpdateJobRequest>,
) -> ApiResult<JobDetailRow> {
    let current = db::get_job(&state.pool, &job_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))?;
    let payload_json = payload
        .payload
        .map(|value| serde_json::to_string(&value).unwrap_or_else(|_| "null".to_string()))
        .unwrap_or(current.payload_json.clone());
    let updated = db::update_job(
        &state.pool,
        &job_id,
        payload.job_kind.as_deref().unwrap_or(&current.job_kind),
        payload
            .schedule_kind
            .as_deref()
            .unwrap_or(&current.schedule_kind),
        payload
            .schedule_value
            .as_deref()
            .unwrap_or(&current.schedule_value),
        &payload_json,
        payload.run_at.or(current.next_run_at),
        payload.max_retries.unwrap_or(current.max_retries),
    )
    .await
    .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    updated
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))
}

pub(super) async fn job_run_now(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
) -> ApiResult<JobDetailRow> {
    db::run_job_now(&state.pool, &job_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))
}

pub(super) async fn job_enable(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
) -> ApiResult<JobDetailRow> {
    db::set_job_status(&state.pool, &job_id, "pending")
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))
}

pub(super) async fn job_disable(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
) -> ApiResult<JobDetailRow> {
    db::set_job_status(&state.pool, &job_id, "disabled")
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("job not found"), &request))
}

pub(super) async fn job_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(job_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = db::delete_job(&state.pool, &job_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    if !deleted {
        return Err(scoped(ApiError::not_found("job not found"), &request));
    }
    Ok(StatusCode::NO_CONTENT)
}
