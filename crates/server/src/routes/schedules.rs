use std::process::Stdio;
use std::time::Duration as StdDuration;

use chrono::{DateTime, Duration as ChronoDuration, Utc};
use tokio::process::Command;
use tokio::time::{sleep, timeout};

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ScheduleRecord {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub owner: JsonValue,
    pub trigger: ScheduleTrigger,
    pub executor: ScheduleExecutor,
    #[serde(default)]
    pub delivery: ScheduleDelivery,
    #[serde(default)]
    pub retry: ScheduleRetryPolicy,
    pub enabled: bool,
    #[serde(default)]
    pub next_run_at: Option<String>,
    #[serde(default)]
    pub last_run_at: Option<String>,
    #[serde(default)]
    pub last_status: Option<String>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub last_output: Option<JsonValue>,
    #[serde(default)]
    pub history: Vec<ScheduleRunRecord>,
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
#[serde(tag = "kind", rename_all = "snake_case")]
pub(super) enum ScheduleExecutor {
    Command { command: CommandScheduleExecutor },
    Agent { agent: AgentScheduleExecutor },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct CommandScheduleExecutor {
    pub command: String,
    #[serde(default)]
    pub cwd: Option<String>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct AgentScheduleExecutor {
    pub agent_id: String,
    pub prompt: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub max_turns: Option<u32>,
    #[serde(default)]
    pub context: ScheduleAgentContext,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ScheduleAgentContext {
    #[serde(default)]
    pub conversation_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ScheduleConversationTarget {
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub lane_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ScheduleDelivery {
    #[serde(flatten)]
    pub target: ScheduleConversationTarget,
    #[serde(default)]
    pub content_mode: Option<ScheduleDeliveryContentMode>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub(super) enum ScheduleDeliveryContentMode {
    Full,
    Summary,
    Conclusion,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(super) struct ScheduleRetryPolicy {
    #[serde(default = "default_retry_attempts")]
    pub max_attempts: u8,
    #[serde(default)]
    pub backoff_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct ScheduleRunRecord {
    pub id: String,
    pub started_at: String,
    pub finished_at: String,
    pub attempt: u8,
    pub status: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub delivered: bool,
    #[serde(default)]
    pub delivery_error: Option<String>,
    #[serde(default)]
    pub output: JsonValue,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct SchedulePayload {
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub owner: JsonValue,
    pub trigger: ScheduleTrigger,
    pub executor: ScheduleExecutor,
    #[serde(default)]
    pub delivery: ScheduleDelivery,
    #[serde(default)]
    pub retry: ScheduleRetryPolicy,
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
    let payload = normalize_schedule_payload(payload);
    ensure_schedule_payload(&state, &payload, &request)?;
    let _guard = state.schedule_lock.lock().await;
    let now = now_iso();
    let mut schedules = read_schedules(&state).unwrap_or_default();
    let schedule = ScheduleRecord {
        id: format!("schedule-{}", Uuid::new_v4()),
        name: payload.name,
        description: payload.description,
        owner: payload.owner,
        next_run_at: if payload.enabled {
            next_run_at(&payload.trigger, Utc::now()).map(|item| item.to_rfc3339())
        } else {
            None
        },
        trigger: payload.trigger,
        executor: payload.executor,
        delivery: payload.delivery,
        retry: normalize_retry_policy(payload.retry),
        enabled: payload.enabled,
        last_run_at: None,
        last_status: None,
        last_error: None,
        last_output: None,
        history: Vec::new(),
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
    let payload = normalize_schedule_payload(payload);
    ensure_schedule_payload(&state, &payload, &request)?;
    let _guard = state.schedule_lock.lock().await;
    let mut schedules = read_schedules(&state).unwrap_or_default();
    let Some(schedule) = schedules.iter_mut().find(|item| item.id == schedule_id) else {
        return Err(scoped(ApiError::not_found("schedule not found"), &request));
    };
    schedule.name = payload.name;
    schedule.description = payload.description;
    schedule.owner = payload.owner;
    schedule.trigger = payload.trigger;
    schedule.executor = payload.executor;
    schedule.delivery = payload.delivery;
    schedule.retry = normalize_retry_policy(payload.retry);
    schedule.enabled = payload.enabled;
    schedule.next_run_at = if schedule.enabled {
        next_run_at(&schedule.trigger, Utc::now()).map(|item| item.to_rfc3339())
    } else {
        None
    };
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
    let schedule = {
        let _guard = state.schedule_lock.lock().await;
        read_schedules(state)
            .unwrap_or_default()
            .into_iter()
            .find(|item| item.id == schedule_id)
            .ok_or_else(|| scoped(ApiError::not_found("schedule not found"), request))?
    };

    let execution = execute_schedule(state, &schedule, request).await?;

    {
        let _guard = state.schedule_lock.lock().await;
        let mut schedules = read_schedules(state).unwrap_or_default();
        let Some(current) = schedules.iter_mut().find(|item| item.id == schedule_id) else {
            return Err(scoped(ApiError::not_found("schedule not found"), request));
        };
        current.last_run_at = Some(execution.finished_at.clone());
        current.last_status = Some(execution.status.clone());
        current.last_error = execution.error.clone();
        current.last_output = Some(execution.output.clone());
        current.history.insert(0, execution.record.clone());
        current.history.truncate(20);
        advance_schedule(current, Utc::now());
        current.updated_at = now_iso();
        write_schedules(state, &schedules)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    }

    let response = serde_json::json!({
        "schedule_id": schedule_id,
        "run": execution.record,
    });

    if execution.status == "failed" {
        Err(scoped(
            ApiError::bad_request("schedule executor failed"),
            request,
        ))
    } else {
        Ok(response)
    }
}

struct ScheduleExecution {
    record: ScheduleRunRecord,
    output: JsonValue,
    status: String,
    error: Option<String>,
    finished_at: String,
}

struct ScheduleStepResult {
    ok: bool,
    output: JsonValue,
    error: Option<String>,
}

#[derive(Debug, Clone)]
struct ResolvedDeliveryTarget {
    target: ResolvedConversationTarget,
    content_mode: ScheduleDeliveryContentMode,
}

#[derive(Debug, Clone)]
struct ResolvedConversationTarget {
    conversation_id: Option<String>,
    lane_id: Option<String>,
}

async fn execute_schedule(
    state: &AppState,
    schedule: &ScheduleRecord,
    request: &RequestContext,
) -> Result<ScheduleExecution, ApiError> {
    let started_at = now_iso();
    let retry = normalize_retry_policy(schedule.retry.clone());
    let delivery_target = resolve_delivery_target(state, &schedule.delivery, request).await?;
    let mut attempt = 0u8;
    let mut primary = ScheduleStepResult {
        ok: false,
        output: serde_json::json!({}),
        error: None,
    };

    while attempt < retry.max_attempts.max(1) {
        attempt += 1;
        primary = run_schedule_executor(state, schedule, request).await?;
        if primary.ok || attempt >= retry.max_attempts.max(1) {
            break;
        }
        if retry.backoff_seconds > 0 {
            sleep(StdDuration::from_secs(retry.backoff_seconds.min(3_600))).await;
        }
    }

    let mut delivered = false;
    let mut delivery_error = None;
    let mut delivery_output = JsonValue::Null;

    if primary.ok {
        if delivery_target.target.conversation_id.is_some() {
            match deliver_schedule_result(
                state,
                schedule,
                &delivery_target,
                &primary.output,
                request,
            )
            .await
            {
                Ok(output) => {
                    delivered = true;
                    delivery_output = output;
                }
                Err(error) => {
                    delivery_error = Some(error);
                }
            }
        }
    }

    let status = if primary.ok {
        if delivery_error.is_some() {
            "completed_with_warning".to_string()
        } else {
            "completed".to_string()
        }
    } else {
        "failed".to_string()
    };
    let error = if primary.ok {
        delivery_error.clone()
    } else {
        primary.error.clone()
    };
    let finished_at = now_iso();
    let output = serde_json::json!({
        "schedule_id": schedule.id.clone(),
        "attempts": attempt,
        "executor": primary.output,
        "delivery": delivery_output,
    });
    let record = ScheduleRunRecord {
        id: format!("schedule-run-{}", Uuid::new_v4()),
        started_at,
        finished_at: finished_at.clone(),
        attempt,
        status: status.clone(),
        error: error.clone(),
        delivered,
        delivery_error,
        output: output.clone(),
    };

    Ok(ScheduleExecution {
        record,
        output,
        status,
        error,
        finished_at,
    })
}

async fn run_schedule_executor(
    state: &AppState,
    schedule: &ScheduleRecord,
    request: &RequestContext,
) -> Result<ScheduleStepResult, ApiError> {
    match &schedule.executor {
        ScheduleExecutor::Command { command } => {
            run_command_schedule_executor(command, request).await
        }
        ScheduleExecutor::Agent { agent } => {
            run_agent_schedule_executor(state, schedule, agent, request).await
        }
    }
}

async fn run_command_schedule_executor(
    command: &CommandScheduleExecutor,
    request: &RequestContext,
) -> Result<ScheduleStepResult, ApiError> {
    if command.command.trim().is_empty() {
        return Err(scoped(
            ApiError::bad_request("command is required"),
            request,
        ));
    }

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
    let ok = output.status.success();
    let data = serde_json::json!({
        "kind": "command",
        "command": command.command.clone(),
        "cwd": command.cwd.clone(),
        "timeout_ms": timeout_ms,
        "status_code": status_code,
        "stdout": stdout,
        "stderr": stderr,
    });

    Ok(ScheduleStepResult {
        ok,
        output: data,
        error: if ok {
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

async fn run_agent_schedule_executor(
    state: &AppState,
    schedule: &ScheduleRecord,
    agent: &AgentScheduleExecutor,
    request: &RequestContext,
) -> Result<ScheduleStepResult, ApiError> {
    let context_target = resolve_agent_context(state, &agent.context, request).await?;
    let source_refs = if let Some(conversation_id) = context_target.conversation_id.as_deref() {
        serde_json::json!([{
            "kind": "conversation",
            "id": conversation_id,
            "conversation_id": conversation_id,
        }])
    } else {
        serde_json::json!([])
    };

    let response = dispatch_interface_value(
        state,
        request,
        "run.create",
        serde_json::json!({
            "owner": schedule.owner.clone(),
            "goal": agent.prompt.clone(),
            "trigger": "schedule",
            "participants": [agent.agent_id.clone()],
            "addressed_agents": [agent.agent_id.clone()],
            "source_refs": source_refs,
            "metadata": {
                "schedule_id": schedule.id.clone(),
                "schedule_name": schedule.name.clone(),
                "model_id": agent.model_id.clone(),
                "max_turns": agent.max_turns,
            }
        }),
    )
    .await?;

    Ok(ScheduleStepResult {
        ok: true,
        output: serde_json::json!({
            "kind": "agent",
            "agent_id": agent.agent_id.clone(),
            "model_id": agent.model_id.clone(),
            "max_turns": agent.max_turns,
            "context_conversation_id": context_target.conversation_id.clone(),
            "prompt": agent.prompt.clone(),
            "run": response,
        }),
        error: None,
    })
}

async fn deliver_schedule_result(
    state: &AppState,
    schedule: &ScheduleRecord,
    delivery: &ResolvedDeliveryTarget,
    primary_output: &JsonValue,
    request: &RequestContext,
) -> Result<JsonValue, String> {
    let conversation_id = delivery
        .target
        .conversation_id
        .as_deref()
        .ok_or_else(|| "delivery conversation is required".to_string())?;
    let body = render_delivery_message(schedule, primary_output, delivery.content_mode);
    dispatch_interface_value(
        state,
        request,
        "message.append_agent",
        serde_json::json!({
            "conversation_id": conversation_id,
            "message": {
                "body": body,
                "lane_id": delivery.target.lane_id.clone(),
                "sender": "schedule",
                "role": "system",
                "addressed_agents": ["operator"],
            }
        }),
    )
    .await
    .map_err(|error| format!("{error:?}"))
}

fn render_delivery_message(
    schedule: &ScheduleRecord,
    primary_output: &JsonValue,
    content_mode: ScheduleDeliveryContentMode,
) -> String {
    let title = schedule_title(schedule);
    let executor = schedule_executor_label(&schedule.executor);
    let content = match content_mode {
        ScheduleDeliveryContentMode::Full => render_full_delivery_content(primary_output),
        ScheduleDeliveryContentMode::Summary => summarize_execution_output(primary_output),
        ScheduleDeliveryContentMode::Conclusion => conclude_execution_output(primary_output),
    };
    let section = match content_mode {
        ScheduleDeliveryContentMode::Full => "完整结果",
        ScheduleDeliveryContentMode::Summary => "摘要",
        ScheduleDeliveryContentMode::Conclusion => "最终结论",
    };
    truncate_output(&format!(
        "[定时器] {title}\n执行方式：{executor}\n投递内容：{section}\n\n{content}"
    ))
}

fn schedule_title(schedule: &ScheduleRecord) -> String {
    schedule
        .name
        .clone()
        .filter(|item| !item.trim().is_empty())
        .unwrap_or_else(|| schedule_executor_label(&schedule.executor))
}

fn schedule_executor_label(executor: &ScheduleExecutor) -> String {
    match executor {
        ScheduleExecutor::Command { command } => command.command.clone(),
        ScheduleExecutor::Agent { agent } => format!("agent:{}", agent.agent_id),
    }
}

fn render_full_delivery_content(primary_output: &JsonValue) -> String {
    serde_json::to_string_pretty(primary_output).unwrap_or_else(|_| primary_output.to_string())
}

fn summarize_execution_output(primary_output: &JsonValue) -> String {
    match primary_output.get("kind").and_then(JsonValue::as_str) {
        Some("command") => {
            let status = primary_output
                .get("status_code")
                .and_then(JsonValue::as_i64)
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string());
            let stdout_len = primary_output
                .get("stdout")
                .and_then(JsonValue::as_str)
                .map(|value| value.chars().count())
                .unwrap_or(0);
            let stderr_len = primary_output
                .get("stderr")
                .and_then(JsonValue::as_str)
                .map(|value| value.chars().count())
                .unwrap_or(0);
            format!("命令已执行，退出码 {status}，stdout {stdout_len} 字，stderr {stderr_len} 字。")
        }
        Some("agent") => {
            let agent_id = primary_output
                .get("agent_id")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            let run_id = primary_output
                .get("run")
                .and_then(|run| run.get("run"))
                .and_then(|run| run.get("id"))
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            let decision = primary_output
                .get("run")
                .and_then(|run| run.get("decision"))
                .and_then(|item| item.get("summary"))
                .and_then(JsonValue::as_str)
                .unwrap_or("已创建编排运行。");
            format!("已触发 Agent {agent_id}，run_id={run_id}。{decision}")
        }
        _ => "本次运行已完成。".to_string(),
    }
}

fn conclude_execution_output(primary_output: &JsonValue) -> String {
    match primary_output.get("kind").and_then(JsonValue::as_str) {
        Some("command") => {
            let status_ok = primary_output
                .get("status_code")
                .and_then(JsonValue::as_i64)
                .is_some_and(|code| code == 0);
            if status_ok {
                "命令执行成功。".to_string()
            } else {
                "命令执行失败，请查看完整结果。".to_string()
            }
        }
        Some("agent") => primary_output
            .get("run")
            .and_then(|run| run.get("decision"))
            .and_then(|item| item.get("summary"))
            .and_then(JsonValue::as_str)
            .map(|value| value.to_string())
            .unwrap_or_else(|| "Agent 运行已触发。".to_string()),
        _ => "运行已结束。".to_string(),
    }
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
    const MAX_OUTPUT_CHARS: usize = 8_192;
    value.chars().take(MAX_OUTPUT_CHARS).collect()
}

fn normalize_schedule_payload(mut payload: SchedulePayload) -> SchedulePayload {
    payload.name = normalize_optional_string(payload.name);
    payload.description = normalize_optional_string(payload.description);
    payload.delivery = normalize_delivery(payload.delivery);
    payload.retry = normalize_retry_policy(payload.retry);
    payload.executor = normalize_executor(payload.executor);
    payload
}

fn normalize_executor(executor: ScheduleExecutor) -> ScheduleExecutor {
    match executor {
        ScheduleExecutor::Command { command } => ScheduleExecutor::Command {
            command: CommandScheduleExecutor {
                command: command.command.trim().to_string(),
                cwd: normalize_optional_string(command.cwd),
                timeout_ms: command
                    .timeout_ms
                    .map(|value| value.clamp(1_000, 3_600_000)),
            },
        },
        ScheduleExecutor::Agent { agent } => ScheduleExecutor::Agent {
            agent: AgentScheduleExecutor {
                agent_id: agent.agent_id.trim().to_string(),
                prompt: agent.prompt.trim().to_string(),
                model_id: normalize_optional_string(agent.model_id),
                max_turns: agent.max_turns.map(|value| value.clamp(1, 128)),
                context: normalize_agent_context(agent.context),
            },
        },
    }
}

fn normalize_agent_context(mut context: ScheduleAgentContext) -> ScheduleAgentContext {
    context.conversation_id = normalize_optional_string(context.conversation_id);
    context
}

fn normalize_delivery(mut delivery: ScheduleDelivery) -> ScheduleDelivery {
    delivery.target = normalize_conversation_target(delivery.target);
    if delivery.content_mode.is_none() {
        delivery.content_mode = Some(ScheduleDeliveryContentMode::Full);
    }
    delivery
}

fn normalize_conversation_target(
    mut target: ScheduleConversationTarget,
) -> ScheduleConversationTarget {
    target.conversation_id = normalize_optional_string(target.conversation_id);
    target.lane_id = normalize_optional_string(target.lane_id);
    target
}

fn normalize_optional_string(value: Option<String>) -> Option<String> {
    value.and_then(|item| {
        let trimmed = item.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn ensure_schedule_payload(
    state: &AppState,
    payload: &SchedulePayload,
    request: &RequestContext,
) -> Result<(), ApiError> {
    ensure_schedule_executor(state, &payload.executor, request)?;
    ensure_schedule_delivery(state, &payload.delivery, request)?;
    Ok(())
}

fn ensure_schedule_executor(
    state: &AppState,
    executor: &ScheduleExecutor,
    request: &RequestContext,
) -> Result<(), ApiError> {
    match executor {
        ScheduleExecutor::Command { command } => {
            if command.command.trim().is_empty() {
                return Err(scoped(
                    ApiError::bad_request("command is required"),
                    request,
                ));
            }
        }
        ScheduleExecutor::Agent { agent } => {
            if agent.agent_id.trim().is_empty() {
                return Err(scoped(
                    ApiError::bad_request("agent_id is required"),
                    request,
                ));
            }
            if agent.prompt.trim().is_empty() {
                return Err(scoped(
                    ApiError::bad_request("agent prompt is required"),
                    request,
                ));
            }
            let agents = load_agent_configs(&state.runtime_paths)
                .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
            if !agents.iter().any(|item| item.id == agent.agent_id) {
                return Err(scoped(ApiError::not_found("agent not found"), request));
            }

            resolve_interface_binding(state, "run.create", request)?;
            if agent.context.conversation_id.is_some() {
                resolve_interface_binding(state, "conversation.get", request)?;
            }
        }
    }
    Ok(())
}

async fn resolve_agent_context(
    state: &AppState,
    context: &ScheduleAgentContext,
    request: &RequestContext,
) -> Result<ResolvedConversationTarget, ApiError> {
    let Some(conversation_id) = context.conversation_id.clone() else {
        return Ok(ResolvedConversationTarget {
            conversation_id: None,
            lane_id: None,
        });
    };

    let _ = dispatch_interface_value(
        state,
        request,
        "conversation.get",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;

    Ok(ResolvedConversationTarget {
        conversation_id: Some(conversation_id),
        lane_id: None,
    })
}

fn ensure_schedule_delivery(
    state: &AppState,
    delivery: &ScheduleDelivery,
    request: &RequestContext,
) -> Result<(), ApiError> {
    if delivery.target.lane_id.is_some() && delivery.target.conversation_id.is_none() {
        return Err(scoped(
            ApiError::bad_request("lane_id requires conversation_id"),
            request,
        ));
    }
    if delivery
        .target
        .conversation_id
        .as_deref()
        .filter(|item| !item.trim().is_empty())
        .is_some()
    {
        resolve_interface_binding(state, "message.append_agent", request)?;
        resolve_interface_binding(state, "conversation.get", request)?;
    }
    if delivery
        .target
        .lane_id
        .as_deref()
        .filter(|item| !item.trim().is_empty())
        .is_some()
    {
        resolve_interface_binding(state, "lane.list_by_conversation", request)?;
    }
    Ok(())
}

async fn resolve_conversation_target(
    state: &AppState,
    target: &ScheduleConversationTarget,
    request: &RequestContext,
) -> Result<ResolvedConversationTarget, ApiError> {
    let Some(conversation_id) = target.conversation_id.clone() else {
        return Ok(ResolvedConversationTarget {
            conversation_id: None,
            lane_id: None,
        });
    };

    let _ = dispatch_interface_value(
        state,
        request,
        "conversation.get",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;

    let lane_id = target.lane_id.clone();
    if let Some(lane_id) = lane_id.clone() {
        let lanes = dispatch_interface_value(
            state,
            request,
            "lane.list_by_conversation",
            serde_json::json!({ "conversation_id": conversation_id }),
        )
        .await?;
        let lane_exists = lanes.as_array().is_some_and(|items| {
            items.iter().any(|item| {
                item.get("id")
                    .and_then(JsonValue::as_str)
                    .is_some_and(|value| value == lane_id)
            })
        });
        if !lane_exists {
            return Err(scoped(ApiError::not_found("lane not found"), request));
        }
    }

    Ok(ResolvedConversationTarget {
        conversation_id: Some(conversation_id),
        lane_id,
    })
}

async fn resolve_delivery_target(
    state: &AppState,
    delivery: &ScheduleDelivery,
    request: &RequestContext,
) -> Result<ResolvedDeliveryTarget, ApiError> {
    Ok(ResolvedDeliveryTarget {
        target: resolve_conversation_target(state, &delivery.target, request).await?,
        content_mode: delivery
            .content_mode
            .unwrap_or(ScheduleDeliveryContentMode::Full),
    })
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

fn default_retry_attempts() -> u8 {
    1
}

fn normalize_retry_policy(mut retry: ScheduleRetryPolicy) -> ScheduleRetryPolicy {
    retry.max_attempts = retry.max_attempts.clamp(1, 10);
    retry.backoff_seconds = retry.backoff_seconds.min(3_600);
    retry
}
