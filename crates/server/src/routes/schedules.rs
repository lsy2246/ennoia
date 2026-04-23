use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tokio::process::Command;
use tokio::time::timeout;

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
    #[serde(default = "default_schedule_target_kind")]
    pub kind: ScheduleTargetKind,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub action_id: Option<String>,
    #[serde(default)]
    pub command: Option<CommandScheduleTarget>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ScheduleTargetKind {
    Extension,
    Command,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CommandScheduleTarget {
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
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
    ensure_schedule_target(&state, &payload.target, &request)?;
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
    ensure_schedule_target(&state, &payload.target, &request)?;
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
    let execution = run_schedule_target(state, &schedule, request).await?;

    schedule.last_run_at = Some(now_iso());
    if execution.ok {
        schedule.last_status = Some("completed".to_string());
        schedule.last_error = None;
    } else {
        schedule.last_status = Some("failed".to_string());
        schedule.last_error = execution.error;
    }
    advance_schedule(&mut schedule, Utc::now());
    schedule.updated_at = now_iso();
    schedules[index] = schedule;
    write_schedules(state, &schedules)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if execution.ok {
        Ok(execution.data)
    } else {
        Err(scoped(
            ApiError::bad_request("schedule target failed"),
            request,
        ))
    }
}

struct ScheduleExecution {
    ok: bool,
    data: JsonValue,
    error: Option<String>,
}

async fn run_schedule_target(
    state: &AppState,
    schedule: &ScheduleRecord,
    request: &RequestContext,
) -> Result<ScheduleExecution, ApiError> {
    match schedule.target.kind {
        ScheduleTargetKind::Extension => run_extension_schedule_target(state, schedule, request),
        ScheduleTargetKind::Command => run_command_schedule_target(schedule, request).await,
    }
}

fn run_extension_schedule_target(
    state: &AppState,
    schedule: &ScheduleRecord,
    request: &RequestContext,
) -> Result<ScheduleExecution, ApiError> {
    let action = ensure_schedule_action(state, &schedule.target, request)?;
    let extension_id = schedule_extension_id(&schedule.target, request)?;
    let action_id = schedule_action_id(&schedule.target, request)?;
    let response = state
        .extensions
        .dispatch_rpc(
            &extension_id,
            &action.schedule_action.method,
            ennoia_kernel::ExtensionRpcRequest {
                params: schedule.params.clone(),
                context: serde_json::json!({
                    "schedule_id": schedule.id,
                    "action_id": action_id,
                    "owner": schedule.owner
                }),
            },
        )
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    Ok(ScheduleExecution {
        ok: response.ok,
        data: response.data,
        error: response
            .error
            .map(|error| format!("{}: {}", error.code, error.message)),
    })
}

async fn run_command_schedule_target(
    schedule: &ScheduleRecord,
    request: &RequestContext,
) -> Result<ScheduleExecution, ApiError> {
    let command = schedule_command_target(&schedule.target, request)?;
    let timeout_ms = command
        .timeout_ms
        .unwrap_or(120_000)
        .clamp(1_000, 3_600_000);
    let mut process = shell_command(&command.command);
    process.stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(cwd) = command
        .cwd
        .as_deref()
        .filter(|item| !item.trim().is_empty())
    {
        process.current_dir(cwd);
    }

    let output = timeout(StdDuration::from_millis(timeout_ms), process.output())
        .await
        .map_err(|_| scoped(ApiError::bad_request("command schedule timed out"), request))?
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    let stdout = truncate_output(&String::from_utf8_lossy(&output.stdout));
    let stderr = truncate_output(&String::from_utf8_lossy(&output.stderr));
    let status_code = output.status.code();
    let data = serde_json::json!({
        "kind": "command",
        "command": command.command,
        "cwd": command.cwd,
        "timeout_ms": timeout_ms,
        "status_code": status_code,
        "stdout": stdout,
        "stderr": stderr
    });

    Ok(ScheduleExecution {
        ok: output.status.success(),
        data,
        error: if output.status.success() {
            None
        } else {
            Some(format!(
                "command exited with status {}",
                status_code
                    .map(|code| code.to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ))
        },
    })
}

#[cfg(windows)]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("powershell");
    process.args([
        "-NoProfile",
        "-ExecutionPolicy",
        "Bypass",
        "-Command",
        command,
    ]);
    process
}

#[cfg(not(windows))]
fn shell_command(command: &str) -> Command {
    let mut process = Command::new("sh");
    process.args(["-lc", command]);
    process
}

fn truncate_output(value: &str) -> String {
    const MAX_OUTPUT_CHARS: usize = 8192;
    value.chars().take(MAX_OUTPUT_CHARS).collect()
}

fn ensure_schedule_target(
    state: &AppState,
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<(), ApiError> {
    match target.kind {
        ScheduleTargetKind::Extension => {
            ensure_schedule_action(state, target, request)?;
        }
        ScheduleTargetKind::Command => {
            let command = schedule_command_target(target, request)?;
            if command.command.trim().is_empty() {
                return Err(scoped(
                    ApiError::bad_request("command is required"),
                    request,
                ));
            }
        }
    }
    Ok(())
}

fn ensure_schedule_action(
    state: &AppState,
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<RegisteredScheduleActionContribution, ApiError> {
    let extension_id = schedule_extension_id(target, request)?;
    let action_id = schedule_action_id(target, request)?;
    state
        .extensions
        .snapshot()
        .schedule_actions
        .into_iter()
        .find(|item| item.extension_id == extension_id && item.schedule_action.id == action_id)
        .ok_or_else(|| scoped(ApiError::not_found("schedule action not found"), request))
}

fn schedule_extension_id(
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<String, ApiError> {
    target
        .extension_id
        .clone()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| scoped(ApiError::bad_request("extension_id is required"), request))
}

fn schedule_action_id(
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<String, ApiError> {
    target
        .action_id
        .clone()
        .filter(|item| !item.trim().is_empty())
        .ok_or_else(|| scoped(ApiError::bad_request("action_id is required"), request))
}

fn schedule_command_target(
    target: &ScheduleTarget,
    request: &RequestContext,
) -> Result<CommandScheduleTarget, ApiError> {
    target
        .command
        .clone()
        .ok_or_else(|| scoped(ApiError::bad_request("command target is required"), request))
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

fn default_schedule_target_kind() -> ScheduleTargetKind {
    ScheduleTargetKind::Extension
}
