use ennoia_kernel::{
    ActionPhase, ActionResultMode, CapabilityPermissionMetadata, HookEventEnvelope,
    HookResourceRef, OwnerRef, PermissionRequest, PermissionScope, PermissionTarget,
    PermissionTrigger,
};
use std::time::Instant;

use super::*;
use crate::agent_permissions::PermissionApprovalsQuery;
use crate::app::record_trace_span;
use crate::event_bus::HookEventWrite;
use crate::observability::{
    ObservationLogWrite, ObservationSpanWrite, OBSERVABILITY_COMPONENT_EVENT_BUS,
    OBSERVABILITY_COMPONENT_PROXY,
};
use crate::pipeline::dispatch_action_pipeline;

#[derive(Debug, Serialize)]
pub(super) struct ActionStatusRecord {
    action: String,
    rules: Vec<ActionImplementationRecord>,
    execute_rule_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub(super) struct ActionImplementationRecord {
    extension_id: String,
    capability_id: String,
    method: String,
    phase: String,
    priority: i32,
    enabled: bool,
    result_mode: String,
    when: JsonValue,
    #[serde(default)]
    schema: Option<String>,
    extension_status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PermissionActorContext {
    agent_id: String,
    kind: String,
    #[serde(default)]
    user_initiated: bool,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    message_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct ConversationStreamSnapshotPayload {
    detail: JsonValue,
    approvals: Vec<ennoia_kernel::PermissionApprovalRecord>,
}

#[derive(Debug, Serialize)]
struct ConversationStreamErrorPayload {
    message: String,
}

pub(super) async fn extension_actions(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredActionRuleContribution>> {
    Json(state.extensions.snapshot().actions)
}

pub(super) async fn extension_schedule_actions(
    State(state): State<AppState>,
) -> Json<Vec<RegisteredScheduleActionContribution>> {
    Json(state.extensions.snapshot().schedule_actions)
}

pub(super) async fn actions_status(State(state): State<AppState>) -> Json<Vec<ActionStatusRecord>> {
    Json(list_action_status(&state))
}

pub(super) async fn conversations_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "conversation.list", JsonValue::Null).await
}

pub(super) async fn conversations_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "conversation.create", payload).await
}

pub(super) async fn conversation_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "conversation.get",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_stream(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> Result<Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>>, ApiError>
{
    let initial_snapshot =
        load_conversation_stream_snapshot(&state, &request, &conversation_id).await?;
    let initial_data = serde_json::to_string(&initial_snapshot)
        .unwrap_or_else(|_| "{\"detail\":{},\"approvals\":[]}".to_string());
    let stream_state = state.clone();
    let stream_request = request.clone();
    let stream_conversation_id = conversation_id.clone();

    let stream = async_stream::stream! {
        yield Ok(Event::default().event("conversation.snapshot").data(initial_data.clone()));

        let mut last_event_seq = stream_state
            .event_bus
            .latest_conversation_seq(&stream_conversation_id)
            .unwrap_or(0);
        let mut last_approval_seq = stream_state
            .agent_permissions
            .latest_conversation_approval_seq(&stream_conversation_id)
            .unwrap_or(0);
        let mut interval = tokio::time::interval(Duration::from_millis(500));
        loop {
            interval.tick().await;

            let next_event_seq = match stream_state
                .event_bus
                .latest_conversation_seq(&stream_conversation_id)
            {
                Ok(value) => value,
                Err(error) => {
                    yield Ok(conversation_stream_error_event(error.to_string()));
                    continue;
                }
            };
            let next_approval_seq = match stream_state
                .agent_permissions
                .latest_conversation_approval_seq(&stream_conversation_id)
            {
                Ok(value) => value,
                Err(error) => {
                    yield Ok(conversation_stream_error_event(error.to_string()));
                    continue;
                }
            };

            if next_event_seq <= last_event_seq && next_approval_seq <= last_approval_seq {
                continue;
            }

            match load_conversation_stream_snapshot(
                &stream_state,
                &stream_request,
                &stream_conversation_id,
            )
            .await
            {
                Ok(snapshot) => {
                    last_event_seq = next_event_seq.max(last_event_seq);
                    last_approval_seq = next_approval_seq.max(last_approval_seq);
                    let data = serde_json::to_string(&snapshot).unwrap_or_else(|_| {
                        "{\"detail\":{},\"approvals\":[]}".to_string()
                    });
                    yield Ok(Event::default().event("conversation.snapshot").data(data));
                }
                Err(error) => {
                    yield Ok(conversation_stream_error_event(error.to_string()));
                }
            }
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

pub(super) async fn conversation_delete(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "conversation.delete",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_messages(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "message.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_branches(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "branch.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_branches_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "branch.create",
        serde_json::json!({
            "conversation_id": conversation_id,
            "from_branch_id": payload.get("from_branch_id").cloned(),
            "source_message_id": payload.get("source_message_id").cloned(),
            "source_checkpoint_id": payload.get("source_checkpoint_id").cloned(),
            "name": payload.get("name").cloned(),
            "mode": payload.get("mode").cloned(),
            "activate": payload.get("activate").cloned(),
        }),
    )
    .await
}

pub(super) async fn conversation_branch_switch(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((conversation_id, branch_id)): Path<(String, String)>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "branch.switch",
        serde_json::json!({
            "conversation_id": conversation_id,
            "branch_id": branch_id,
        }),
    )
    .await
}

pub(super) async fn conversation_checkpoints(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "checkpoint.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn conversation_checkpoints_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "checkpoint.create",
        serde_json::json!({
            "conversation_id": conversation_id,
            "branch_id": payload.get("branch_id").cloned(),
            "message_id": payload.get("message_id").cloned(),
            "kind": payload.get("kind").cloned(),
            "label": payload.get("label").cloned(),
        }),
    )
    .await
}

pub(super) async fn conversation_messages_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "message.append",
        serde_json::json!({
            "conversation_id": conversation_id,
            "message": normalize_conversation_message_payload(payload, "operator", "operator")
        }),
    )
    .await
}

pub(super) async fn conversation_lanes(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "lane.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn runs_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "run.create", payload).await
}

pub(super) async fn run_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "run.get",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

pub(super) async fn conversation_runs(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "run.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await
}

pub(super) async fn run_tasks(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "task.list",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

pub(super) async fn run_artifacts(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(run_id): Path<String>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "artifact.list",
        serde_json::json!({ "run_id": run_id }),
    )
    .await
}

fn normalize_conversation_message_payload(
    payload: JsonValue,
    default_role: &str,
    default_sender: &str,
) -> JsonValue {
    let mut message = match payload {
        JsonValue::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    let role_missing = message
        .get("role")
        .and_then(JsonValue::as_str)
        .is_none_or(|item| item.trim().is_empty());
    if role_missing {
        message.insert(
            "role".to_string(),
            JsonValue::String(default_role.to_string()),
        );
    }
    let sender_missing = message
        .get("sender")
        .and_then(JsonValue::as_str)
        .is_none_or(|item| item.trim().is_empty());
    if sender_missing {
        message.insert(
            "sender".to_string(),
            JsonValue::String(default_sender.to_string()),
        );
    }
    JsonValue::Object(message)
}

pub(super) async fn dispatch_action_json(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
) -> ApiResult<JsonValue> {
    dispatch_action_value(state, request, key, params)
        .await
        .map(Json)
}

pub(crate) async fn dispatch_action_value(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
) -> Result<JsonValue, ApiError> {
    dispatch_action_value_with_context(state, request, key, params, JsonValue::Null).await
}

async fn load_conversation_stream_snapshot(
    state: &AppState,
    request: &RequestContext,
    conversation_id: &str,
) -> Result<ConversationStreamSnapshotPayload, ApiError> {
    let detail = load_conversation_detail_value(state, request, conversation_id).await?;
    let approvals = state
        .agent_permissions
        .list_approvals(&PermissionApprovalsQuery {
            agent_id: None,
            conversation_id: Some(conversation_id.to_string()),
            status: None,
            limit: 80,
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    Ok(ConversationStreamSnapshotPayload { detail, approvals })
}

async fn load_conversation_detail_value(
    state: &AppState,
    request: &RequestContext,
    conversation_id: &str,
) -> Result<JsonValue, ApiError> {
    let detail = dispatch_action_value(
        state,
        request,
        "conversation.get",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;
    let conversation = detail
        .get("conversation")
        .cloned()
        .unwrap_or_else(|| detail.clone());
    let lanes = match json_array_field(&detail, "lanes") {
        Some(value) => value,
        None => {
            dispatch_action_value(
                state,
                request,
                "lane.list",
                serde_json::json!({ "conversation_id": conversation_id }),
            )
            .await?
        }
    };
    let branches = match json_array_field(&detail, "branches") {
        Some(value) => value,
        None => {
            dispatch_action_value(
                state,
                request,
                "branch.list",
                serde_json::json!({ "conversation_id": conversation_id }),
            )
            .await?
        }
    };
    let checkpoints = match json_array_field(&detail, "checkpoints") {
        Some(value) => value,
        None => {
            dispatch_action_value(
                state,
                request,
                "checkpoint.list",
                serde_json::json!({ "conversation_id": conversation_id }),
            )
            .await?
        }
    };
    let messages = match json_array_field(&detail, "messages") {
        Some(value) => value,
        None => {
            dispatch_action_value(
                state,
                request,
                "message.list",
                serde_json::json!({ "conversation_id": conversation_id }),
            )
            .await?
        }
    };

    Ok(serde_json::json!({
        "conversation": conversation,
        "lanes": lanes,
        "branches": branches,
        "checkpoints": checkpoints,
        "messages": messages,
        "runs": [],
        "tasks": [],
        "outputs": [],
    }))
}

fn json_array_field(value: &JsonValue, key: &str) -> Option<JsonValue> {
    value.get(key).filter(|item| item.is_array()).cloned()
}

fn conversation_stream_error_event(message: String) -> Event {
    Event::default().event("conversation.error").data(
        serde_json::to_string(&ConversationStreamErrorPayload { message })
            .unwrap_or_else(|_| "{\"message\":\"conversation stream error\"}".to_string()),
    )
}

pub(crate) async fn dispatch_action_value_with_context(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
    context: JsonValue,
) -> Result<JsonValue, ApiError> {
    dispatch_action_pipeline(state, request, key, params, context).await
}

pub(crate) async fn dispatch_action_rule_execute(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    rule: &RegisteredActionRuleContribution,
    params: JsonValue,
    context: JsonValue,
) -> Result<JsonValue, ApiError> {
    let extension = state.extensions.get(&rule.extension_id).ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("extension '{}' not found", rule.extension_id)),
            request,
        )
    })?;
    if extension.worker.is_none() {
        return Err(scoped(
            ApiError::not_found(format!("extension '{}' has no worker", rule.extension_id)),
            request,
        ));
    }

    let permission_grant_id =
        authorize_action_dispatch(state, request, key, rule, &params, &context)?;

    let span_trace = request.child_trace("action_rpc");
    let started = Instant::now();
    let started_at = now_iso();
    let response = state
        .extensions
        .dispatch_rpc(
            &rule.extension_id,
            &rule.action.method,
            ennoia_kernel::ExtensionRpcRequest {
                params,
                context: serde_json::json!({
                    "action": key,
                    "capability_id": rule.action.capability_id,
                    "request_id": request.request_id,
                    "trace": {
                        "request_id": span_trace.request_id,
                        "trace_id": span_trace.trace_id,
                        "span_id": span_trace.span_id,
                        "parent_span_id": span_trace.parent_span_id,
                        "sampled": span_trace.sampled,
                        "source": span_trace.source,
                        "traceparent": span_trace.to_traceparent(),
                    },
                    "extra": context
                }),
            },
        )
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        if let Some(grant_id) = permission_grant_id.as_deref() {
            if let Err(error) = state.agent_permissions.consume_grant(grant_id) {
                let _ = state.observability.append_log_scoped(
                    ObservationLogWrite {
                        event: "runtime.permission.consume_grant_failed".to_string(),
                        level: "warn".to_string(),
                        component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                        source_kind: "permission".to_string(),
                        source_id: Some(grant_id.to_string()),
                        message: "permission grant consume failed".to_string(),
                        attributes: serde_json::json!({
                            "action": key,
                            "extension_id": rule.extension_id,
                            "error": error.to_string(),
                        }),
                        created_at: None,
                    },
                    Some(&span_trace),
                );
            }
        }
        record_trace_span(
            state,
            ObservationSpanWrite {
                trace: span_trace,
                kind: "action_rpc".to_string(),
                name: key.to_string(),
                component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                source_kind: "extension".to_string(),
                source_id: Some(rule.extension_id.clone()),
                status: "ok".to_string(),
                attributes: serde_json::json!({
                    "action": key,
                    "extension_id": rule.extension_id,
                    "capability_id": rule.action.capability_id,
                    "method": rule.action.method,
                }),
                started_at,
                ended_at: now_iso(),
                duration_ms: started.elapsed().as_millis() as i64,
            },
        );
        return Ok(response.data);
    }

    let error = response
        .error
        .map(|item| format!("{}: {}", item.code, item.message))
        .unwrap_or_else(|| format!("action '{key}' failed"));
    record_trace_span(
        state,
        ObservationSpanWrite {
            trace: span_trace,
            kind: "action_rpc".to_string(),
            name: key.to_string(),
            component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
            source_kind: "extension".to_string(),
            source_id: Some(rule.extension_id.clone()),
            status: "error".to_string(),
            attributes: serde_json::json!({
                "action": key,
                "extension_id": rule.extension_id,
                "capability_id": rule.action.capability_id,
                "method": rule.action.method,
                "error": error,
            }),
            started_at,
            ended_at: now_iso(),
            duration_ms: started.elapsed().as_millis() as i64,
        },
    );
    Err(scoped(ApiError::bad_request(error), request))
}

fn authorize_action_dispatch(
    state: &AppState,
    request: &RequestContext,
    _key: &str,
    rule: &RegisteredActionRuleContribution,
    params: &JsonValue,
    context: &JsonValue,
) -> Result<Option<String>, ApiError> {
    let Some(actor) = permission_actor_from_context(context) else {
        return Ok(None);
    };
    let Some(capability) =
        find_action_capability(state, &rule.extension_id, &rule.action.capability_id)
    else {
        return Ok(None);
    };
    let Some(permission) = capability_permission_metadata(&capability.capability.metadata) else {
        return Ok(None);
    };
    let permission_request =
        build_action_permission_request(&actor, rule, &capability, &permission, params);
    let decision = state
        .agent_permissions
        .evaluate_request(&permission_request, Some(request))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    match decision.decision.as_str() {
        "allow" => Ok(decision.grant_id),
        "ask" => {
            let approval_id = decision
                .approval_id
                .clone()
                .unwrap_or_else(|| "unknown".to_string());
            Err(scoped(
                ApiError::forbidden(format!(
                    "approval required: action={}, approval_id={approval_id}",
                    permission_request.action
                ))
                .with_details(serde_json::json!({
                    "decision": decision.decision,
                    "approval_id": decision.approval_id,
                    "agent_id": permission_request.agent_id,
                    "action": permission_request.action,
                    "target": permission_request.target,
                    "scope": permission_request.scope,
                    "reason": decision.reason,
                })),
                request,
            ))
        }
        _ => Err(scoped(
            ApiError::forbidden(format!(
                "permission denied: action={}, reason={}",
                permission_request.action, decision.reason
            ))
            .with_details(serde_json::json!({
                "decision": decision.decision,
                "agent_id": permission_request.agent_id,
                "action": permission_request.action,
                "target": permission_request.target,
                "scope": permission_request.scope,
                "reason": decision.reason,
            })),
            request,
        )),
    }
}

pub(crate) fn action_rules_for_key(
    state: &AppState,
    key: &str,
    phase: Option<ActionPhase>,
) -> Vec<RegisteredActionRuleContribution> {
    let mut matches = state
        .extensions
        .snapshot()
        .actions
        .into_iter()
        .filter(|item| item.action.action == key && item.action.enabled)
        .filter(|item| {
            phase
                .as_ref()
                .is_none_or(|expected| item.action.phase == *expected)
        })
        .collect::<Vec<_>>();
    matches.sort_by(|left, right| {
        right
            .action
            .priority
            .cmp(&left.action.priority)
            .then_with(|| {
                action_phase_rank(&left.action.phase).cmp(&action_phase_rank(&right.action.phase))
            })
            .then_with(|| left.extension_id.cmp(&right.extension_id))
            .then_with(|| left.action.capability_id.cmp(&right.action.capability_id))
    });
    matches
}

pub(crate) fn ensure_action_execute_available(
    state: &AppState,
    key: &str,
    request: &RequestContext,
) -> Result<(), ApiError> {
    if !action_rules_for_key(state, key, Some(ActionPhase::Execute)).is_empty() {
        return Ok(());
    }
    let _ = state.observability.append_log_scoped(
        ObservationLogWrite {
            event: "runtime.action.missing".to_string(),
            level: "warn".to_string(),
            component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
            source_kind: "action".to_string(),
            source_id: Some(key.to_string()),
            message: "action execute rule missing".to_string(),
            attributes: serde_json::json!({ "action": key }),
            created_at: None,
        },
        Some(&request.trace_context()),
    );
    Err(scoped(
        ApiError::not_found(format!("action '{key}' has no execute rule")),
        request,
    ))
}

fn list_action_status(state: &AppState) -> Vec<ActionStatusRecord> {
    let mut by_key = HashMap::<String, Vec<ActionImplementationRecord>>::new();
    for item in state.extensions.snapshot().actions {
        by_key
            .entry(item.action.action.clone())
            .or_default()
            .push(ActionImplementationRecord {
                extension_id: item.extension_id.clone(),
                capability_id: item.action.capability_id,
                method: item.action.method,
                phase: action_phase_label(&item.action.phase).to_string(),
                priority: item.action.priority,
                enabled: item.action.enabled,
                result_mode: action_result_mode_label(&item.action.result_mode).to_string(),
                when: item.action.when,
                schema: item.action.schema,
                extension_status: state
                    .extensions
                    .get(&item.extension_id)
                    .map(|extension| format!("{:?}", extension.health).to_lowercase())
                    .unwrap_or_else(|| "missing".to_string()),
            });
    }

    let mut rows = by_key
        .into_iter()
        .map(|(action, mut rules)| {
            rules.sort_by(|left, right| {
                right
                    .priority
                    .cmp(&left.priority)
                    .then_with(|| {
                        action_phase_name_order(&left.phase)
                            .cmp(&action_phase_name_order(&right.phase))
                    })
                    .then_with(|| left.extension_id.cmp(&right.extension_id))
                    .then_with(|| left.capability_id.cmp(&right.capability_id))
            });
            let execute_rule_count = rules
                .iter()
                .filter(|item| item.enabled && item.phase == "execute")
                .count();
            ActionStatusRecord {
                action,
                rules,
                execute_rule_count,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| left.action.cmp(&right.action));
    rows
}

fn action_phase_rank(phase: &ActionPhase) -> u8 {
    match phase {
        ActionPhase::Before => 0,
        ActionPhase::Execute => 1,
        ActionPhase::AfterSuccess => 2,
        ActionPhase::AfterError => 3,
    }
}

fn action_phase_label(phase: &ActionPhase) -> &'static str {
    match phase {
        ActionPhase::Before => "before",
        ActionPhase::Execute => "execute",
        ActionPhase::AfterSuccess => "after_success",
        ActionPhase::AfterError => "after_error",
    }
}

fn action_phase_name_order(phase: &str) -> u8 {
    match phase {
        "before" => 0,
        "execute" => 1,
        "after_success" => 2,
        "after_error" => 3,
        _ => 9,
    }
}

fn action_result_mode_label(mode: &ActionResultMode) -> &'static str {
    match mode {
        ActionResultMode::Void => "void",
        ActionResultMode::First => "first",
        ActionResultMode::Last => "last",
        ActionResultMode::Collect => "collect",
        ActionResultMode::Merge => "merge",
    }
}

pub(crate) fn dispatch_hook_event(
    state: &AppState,
    request: &RequestContext,
    event: &str,
    resource_kind: &str,
    resource_id: &str,
    payload: JsonValue,
) {
    let envelope = HookEventEnvelope {
        event: event.to_string(),
        occurred_at: now_iso(),
        owner: payload_owner(&payload),
        resource: HookResourceRef {
            kind: resource_kind.to_string(),
            id: resource_id.to_string(),
            conversation_id: payload_string_field(&payload, &["conversation_id"])
                .or_else(|| payload_string_field(&payload, &["conversation", "id"]))
                .or_else(|| payload_string_field(&payload, &["message", "conversation_id"])),
            lane_id: payload_string_field(&payload, &["lane_id"])
                .or_else(|| payload_string_field(&payload, &["lane", "id"]))
                .or_else(|| payload_string_field(&payload, &["message", "lane_id"])),
            run_id: payload_string_field(&payload, &["run_id"])
                .or_else(|| payload_string_field(&payload, &["run", "id"])),
            task_id: None,
            artifact_id: None,
        },
        payload,
    };

    let span_trace = request.child_trace("event_publish");
    let started = Instant::now();
    let started_at = now_iso();
    match state.event_bus.publish(HookEventWrite {
        envelope,
        hooks: state.extensions.hooks_for_event(event),
        trace: span_trace.clone(),
    }) {
        Ok(event_id) => {
            record_trace_span(
                state,
                ObservationSpanWrite {
                    trace: span_trace,
                    kind: "event_publish".to_string(),
                    name: event.to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: resource_kind.to_string(),
                    source_id: Some(resource_id.to_string()),
                    status: "ok".to_string(),
                    attributes: serde_json::json!({
                        "event": event,
                        "event_id": event_id,
                    }),
                    started_at,
                    ended_at: now_iso(),
                    duration_ms: started.elapsed().as_millis() as i64,
                },
            );
        }
        Err(error) => {
            record_trace_span(
                state,
                ObservationSpanWrite {
                    trace: span_trace.clone(),
                    kind: "event_publish".to_string(),
                    name: event.to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: resource_kind.to_string(),
                    source_id: Some(resource_id.to_string()),
                    status: "error".to_string(),
                    attributes: serde_json::json!({
                        "event": event,
                        "error": error.to_string(),
                    }),
                    started_at,
                    ended_at: now_iso(),
                    duration_ms: started.elapsed().as_millis() as i64,
                },
            );
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.event_bus.publish_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_EVENT_BUS.to_string(),
                    source_kind: "hook".to_string(),
                    source_id: Some(event.to_string()),
                    message: "hook event publish failed".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&span_trace),
            );
        }
    }
}

fn permission_actor_from_context(context: &JsonValue) -> Option<PermissionActorContext> {
    serde_json::from_value::<PermissionActorContext>(context.get("permission_actor")?.clone()).ok()
}

fn find_action_capability(
    state: &AppState,
    extension_id: &str,
    capability_id: &str,
) -> Option<RegisteredCapabilityContribution> {
    state
        .extensions
        .snapshot()
        .capabilities
        .into_iter()
        .find(|item| item.extension_id == extension_id && item.capability.id == capability_id)
}

fn capability_permission_metadata(metadata: &JsonValue) -> Option<CapabilityPermissionMetadata> {
    serde_json::from_value(metadata.get("permission")?.clone()).ok()
}

fn build_action_permission_request(
    actor: &PermissionActorContext,
    rule: &RegisteredActionRuleContribution,
    capability: &RegisteredCapabilityContribution,
    permission: &CapabilityPermissionMetadata,
    params: &JsonValue,
) -> PermissionRequest {
    let conversation_id =
        permission_conversation_id(params).or_else(|| actor.conversation_id.clone());
    let run_id = permission_run_id(params).or_else(|| actor.run_id.clone());
    let message_id = permission_message_id(params).or_else(|| actor.message_id.clone());
    let path = permission_path(params);
    let host = permission_host(params);
    let target = PermissionTarget {
        kind: permission.target_kind.clone(),
        id: permission_target_id(
            permission,
            capability,
            params,
            conversation_id.as_deref(),
            run_id.as_deref(),
        ),
        conversation_id: conversation_id.clone(),
        run_id: run_id.clone(),
        path: path.clone(),
        host: host.clone(),
    };
    let scope = PermissionScope {
        conversation_id: permission_scope_conversation_id(permission, conversation_id.clone()),
        run_id: permission_scope_run_id(permission, run_id.clone()),
        message_id,
        extension_id: Some(rule.extension_id.clone()),
        path,
        host,
    };
    PermissionRequest {
        agent_id: actor.agent_id.clone(),
        action: permission.action.clone(),
        target,
        scope,
        trigger: PermissionTrigger {
            kind: actor.kind.clone(),
            user_initiated: actor.user_initiated,
        },
    }
}

fn permission_scope_conversation_id(
    permission: &CapabilityPermissionMetadata,
    conversation_id: Option<String>,
) -> Option<String> {
    match permission.scope_kind.trim().to_ascii_lowercase().as_str() {
        "none" => None,
        "run" | "conversation" | "extension" | "" => conversation_id,
        _ => conversation_id,
    }
}

fn permission_scope_run_id(
    permission: &CapabilityPermissionMetadata,
    run_id: Option<String>,
) -> Option<String> {
    match permission.scope_kind.trim().to_ascii_lowercase().as_str() {
        "run" => run_id,
        _ => None,
    }
}

fn permission_target_id(
    permission: &CapabilityPermissionMetadata,
    capability: &RegisteredCapabilityContribution,
    params: &JsonValue,
    conversation_id: Option<&str>,
    run_id: Option<&str>,
) -> String {
    let normalized_kind = permission.target_kind.trim().to_ascii_lowercase();
    let candidate = match normalized_kind.as_str() {
        "conversation" => json_string_at(params, &["conversation_id"])
            .or_else(|| json_string_at(params, &["conversation", "id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "branch" => json_string_at(params, &["branch_id"])
            .or_else(|| json_string_at(params, &["from_branch_id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "checkpoint" => json_string_at(params, &["checkpoint_id"])
            .or_else(|| json_string_at(params, &["source_checkpoint_id"]))
            .or_else(|| json_string_at(params, &["message_id"]))
            .or_else(|| conversation_id.map(str::to_string)),
        "run" => json_string_at(params, &["run_id"]).or_else(|| run_id.map(str::to_string)),
        "task" => json_string_at(params, &["task_id"]).or_else(|| run_id.map(str::to_string)),
        "artifact" => {
            json_string_at(params, &["artifact_id"]).or_else(|| run_id.map(str::to_string))
        }
        "memory" => json_string_at(params, &["memory_id"])
            .or_else(|| json_string_at(params, &["workspace_id"]))
            .or_else(|| Some("memory".to_string())),
        _ => None,
    };
    candidate.unwrap_or_else(|| capability.capability.id.clone())
}

fn permission_conversation_id(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["conversation_id"])
        .or_else(|| json_string_at(params, &["conversation", "id"]))
        .or_else(|| json_string_at(params, &["message", "conversation_id"]))
}

fn permission_run_id(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["run_id"]).or_else(|| json_string_at(params, &["run", "id"]))
}

fn permission_message_id(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["message_id"]).or_else(|| json_string_at(params, &["message", "id"]))
}

fn permission_path(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["path"])
        .or_else(|| json_string_at(params, &["cwd"]))
        .or_else(|| json_string_at(params, &["file_path"]))
}

fn permission_host(params: &JsonValue) -> Option<String> {
    json_string_at(params, &["host"]).or_else(|| json_string_at(params, &["base_url"]))
}

fn json_string_at(value: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = value;
    for segment in path {
        current = current.get(*segment)?;
    }
    current
        .as_str()
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn payload_string_field(payload: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
}

fn payload_owner(payload: &JsonValue) -> Option<OwnerRef> {
    payload
        .get("owner")
        .cloned()
        .or_else(|| {
            payload
                .get("conversation")
                .and_then(|item| item.get("owner"))
                .cloned()
        })
        .and_then(|value| serde_json::from_value(value).ok())
}
