use chrono::{DateTime, Duration as ChronoDuration, Utc};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ScheduleRecord {
    pub id: String,
    pub owner: JsonValue,
    pub trigger: ScheduleTrigger,
    pub target: ScheduleTarget,
    #[serde(default)]
    pub params: JsonValue,
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ScheduleTrigger {
    Once {
        at: String,
    },
    Interval {
        every_seconds: u64,
    },
    Cron {
        expression: String,
        next_run_at: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ScheduleTarget {
    pub extension_id: String,
    pub action_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SchedulePayload {
    #[serde(default)]
    pub owner: JsonValue,
    pub trigger: ScheduleTrigger,
    pub target: ScheduleTarget,
    #[serde(default)]
    pub params: JsonValue,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

pub(super) async fn schedule_actions(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredScheduleActionContribution>> {
    Json(state.extensions.snapshot().schedule_actions)
}

pub(super) async fn schedules_list(State(state): State<AppState>) -> Json<Vec<ScheduleRecord>> {
    let _guard = state.schedule_lock.lock().await;
    Json(read_schedules(&state).unwrap_or_default())
}

pub(super) async fn schedule_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
) -> ApiResult<ScheduleRecord> {
    let _guard = state.schedule_lock.lock().await;
    let schedule = read_schedules(&state)
        .unwrap_or_default()
        .into_iter()
        .find(|item| item.id == schedule_id)
        .ok_or_else(|| scoped(ApiError::not_found("schedule not found"), &request))?;
    Ok(Json(schedule))
}

pub(super) async fn schedule_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<ScheduleRecord> {
    ensure_schedule_action(&state, &payload.target, &request)?;
    let _guard = state.schedule_lock.lock().await;
    let now = now_iso();
    let mut schedules = read_schedules(&state).unwrap_or_default();
    let schedule = ScheduleRecord {
        id: format!("schedule-{}", Uuid::new_v4()),
        owner: payload.owner,
        next_run_at: next_run_at(&payload.trigger, Utc::now()).map(|item| item.to_rfc3339()),
        trigger: payload.trigger,
        target: payload.target,
        params: payload.params,
        enabled: payload.enabled,
        last_run_at: None,
        last_status: None,
        last_error: None,
        created_at: now.clone(),
        updated_at: now,
    };
    schedules.push(schedule.clone());
    write_schedules(&state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(schedule))
}

pub(super) async fn schedule_update(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
    Json(payload): Json<SchedulePayload>,
) -> ApiResult<ScheduleRecord> {
    ensure_schedule_action(&state, &payload.target, &request)?;
    let _guard = state.schedule_lock.lock().await;
    let mut schedules = read_schedules(&state).unwrap_or_default();
    let Some(schedule) = schedules.iter_mut().find(|item| item.id == schedule_id) else {
        return Err(scoped(ApiError::not_found("schedule not found"), &request));
    };
    schedule.owner = payload.owner;
    schedule.trigger = payload.trigger;
    schedule.target = payload.target;
    schedule.params = payload.params;
    schedule.enabled = payload.enabled;
    schedule.next_run_at = next_run_at(&schedule.trigger, Utc::now()).map(|item| item.to_rfc3339());
    schedule.updated_at = now_iso();
    let updated = schedule.clone();
    write_schedules(&state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(updated))
}

pub(super) async fn schedule_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
) -> ApiResult<JsonValue> {
    let _guard = state.schedule_lock.lock().await;
    let mut schedules = read_schedules(&state).unwrap_or_default();
    let before = schedules.len();
    schedules.retain(|item| item.id != schedule_id);
    if schedules.len() == before {
        return Err(scoped(ApiError::not_found("schedule not found"), &request));
    }
    write_schedules(&state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(serde_json::json!({ "deleted": true })))
}

pub(super) async fn schedule_run(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
) -> ApiResult<JsonValue> {
    let _guard = state.schedule_lock.lock().await;
    run_schedule_by_id(&state, &schedule_id, &request)
        .await
        .map(Json)
}

pub(super) async fn schedule_pause(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
) -> ApiResult<ScheduleRecord> {
    set_schedule_enabled(&state, &request, &schedule_id, false).await
}

pub(super) async fn schedule_resume(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(schedule_id): Path<String>,
) -> ApiResult<ScheduleRecord> {
    set_schedule_enabled(&state, &request, &schedule_id, true).await
}

pub(crate) async fn run_due_schedules_once(state: &AppState) {
    let Ok(_guard) = state.schedule_lock.try_lock() else {
        return;
    };
    let now = Utc::now();
    let schedules = read_schedules(state).unwrap_or_default();
    let due_ids = schedules
        .iter()
        .filter(|item| item.enabled)
        .filter(|item| {
            item.next_run_at
                .as_deref()
                .and_then(parse_time)
                .is_some_and(|at| at <= now)
        })
        .map(|item| item.id.clone())
        .collect::<Vec<_>>();
    drop(schedules);
    for schedule_id in due_ids {
        let request = RequestContext {
            request_id: format!("schedule-{}", Uuid::new_v4()),
        };
        let _ = run_schedule_by_id(state, &schedule_id, &request).await;
    }
}

async fn set_schedule_enabled(
    state: &AppState,
    request: &RequestContext,
    schedule_id: &str,
    enabled: bool,
) -> ApiResult<ScheduleRecord> {
    let _guard = state.schedule_lock.lock().await;
    let mut schedules = read_schedules(state).unwrap_or_default();
    let Some(schedule) = schedules.iter_mut().find(|item| item.id == schedule_id) else {
        return Err(scoped(ApiError::not_found("schedule not found"), request));
    };
    schedule.enabled = enabled;
    schedule.next_run_at = if enabled {
        next_run_at(&schedule.trigger, Utc::now()).map(|item| item.to_rfc3339())
    } else {
        None
    };
    schedule.updated_at = now_iso();
    let updated = schedule.clone();
    write_schedules(state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    Ok(Json(updated))
}

async fn run_schedule_by_id(
    state: &AppState,
    schedule_id: &str,
    request: &RequestContext,
) -> Result<JsonValue, ApiError> {
    let mut schedules = read_schedules(state).unwrap_or_default();
    let index = schedules
        .iter()
        .position(|item| item.id == schedule_id)
        .ok_or_else(|| scoped(ApiError::not_found("schedule not found"), request))?;
    let mut schedule = schedules[index].clone();
    let action = ensure_schedule_action(state, &schedule.target, request)?;
    let response = state
        .extensions
        .dispatch_rpc(
            &schedule.target.extension_id,
            &action.schedule_action.method,
            ennoia_kernel::ExtensionRpcRequest {
                params: schedule.params.clone(),
                context: serde_json::json!({
                    "schedule_id": schedule.id,
                    "action_id": schedule.target.action_id,
                    "owner": schedule.owner
                }),
            },
        )
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    schedule.last_run_at = Some(now_iso());
    if response.ok {
        schedule.last_status = Some("completed".to_string());
        schedule.last_error = None;
    } else {
        schedule.last_status = Some("failed".to_string());
        schedule.last_error = response
            .error
            .map(|error| format!("{}: {}", error.code, error.message));
    }
    advance_schedule(&mut schedule, Utc::now());
    schedule.updated_at = now_iso();
    schedules[index] = schedule;
    write_schedules(state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        Ok(response.data)
    } else {
        Err(scoped(
            ApiError::bad_request("schedule action failed"),
            request,
        ))
    }
}

fn ensure_schedule_action(
    state: &AppState,
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<RegisteredScheduleActionContribution, ApiError> {
    state
        .extensions
        .snapshot()
        .schedule_actions
        .into_iter()
        .find(|item| {
            item.extension_id == target.extension_id && item.schedule_action.id == target.action_id
        })
        .ok_or_else(|| scoped(ApiError::not_found("schedule action not found"), request))
}

fn read_schedules(state: &AppState) -> std::io::Result<Vec<ScheduleRecord>> {
    let path = state.runtime_paths.schedules_file();
    if !path.exists() {
        return Ok(Vec::new());
    }
    let contents = fs::read_to_string(path)?;
    serde_json::from_str(&contents).map_err(std::io::Error::other)
}

fn write_schedules(state: &AppState, schedules: &[ScheduleRecord]) -> std::io::Result<()> {
    let path = state.runtime_paths.schedules_file();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = serde_json::to_string_pretty(schedules).map_err(std::io::Error::other)?;
    fs::write(path, contents)
}

fn next_run_at(trigger: &ScheduleTrigger, now: DateTime<Utc>) -> Option<DateTime<Utc>> {
    match trigger {
        ScheduleTrigger::Once { at } => parse_time(at),
        ScheduleTrigger::Interval { every_seconds } => {
            ChronoDuration::try_seconds(*every_seconds as i64).map(|duration| now + duration)
        }
        ScheduleTrigger::Cron { next_run_at, .. } => parse_time(next_run_at),
    }
}

fn advance_schedule(schedule: &mut ScheduleRecord, now: DateTime<Utc>) {
    match &schedule.trigger {
        ScheduleTrigger::Once { .. } => {
            schedule.enabled = false;
            schedule.next_run_at = None;
        }
        ScheduleTrigger::Interval { .. } | ScheduleTrigger::Cron { .. } => {
            schedule.next_run_at =
                next_run_at(&schedule.trigger, now).map(|item| item.to_rfc3339());
        }
    }
}

fn parse_time(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|item| item.with_timezone(&Utc))
}

fn default_enabled() -> bool {
    true
}
