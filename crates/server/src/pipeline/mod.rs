use ennoia_contract::ApiError;
use ennoia_kernel::{
    ActionPhase, ActionResultMode, ExtensionStateGetQuery, PipelineActivationScope,
    PipelineHandlerActivationSpec, PipelineHandlerResponse, PipelineHandlerStage,
    HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
    HOOK_EVENT_RUN_REQUESTED,
};
use ennoia_logs::RequestContext;
use serde_json::Value as JsonValue;

use crate::app::{dispatch_extension_rpc, AppState};
use crate::logs_store::{LogEntryWrite, LOGS_COMPONENT_PROXY};
use crate::realtime::RealtimeEvent;
use crate::routes::{
    actions::{
        action_rules_for_key, dispatch_action_rule_execute, dispatch_hook_event,
        ensure_action_execute_available,
    },
    scoped,
};

const PIPELINE_EVENT_OPERATOR_MESSAGE_RECEIVED: &str = "conversation.operator_message.received";
const PIPELINE_SLOT_CONVERSATION_RESPONSE: &str = "conversation.response";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineStage {
    Before,
    AfterSuccess,
    AfterError,
}

#[derive(Debug, Clone, Copy)]
enum PipelineHandlerAction {
    EmitConversationCreated,
    EmitConversationDeleted,
    EmitConversationMessageCreated,
    EmitRunRequested,
}

#[derive(Debug, Clone, Copy)]
struct PipelineHandler {
    id: &'static str,
    target: &'static str,
    stage: PipelineStage,
    priority: i32,
    enabled: bool,
    action: PipelineHandlerAction,
}

const PIPELINE_HANDLERS: &[PipelineHandler] = &[
    PipelineHandler {
        id: "conversation.created.emit",
        target: "conversation.create",
        stage: PipelineStage::AfterSuccess,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::EmitConversationCreated,
    },
    PipelineHandler {
        id: "conversation.deleted.emit",
        target: "conversation.delete",
        stage: PipelineStage::AfterSuccess,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::EmitConversationDeleted,
    },
    PipelineHandler {
        id: "message.created.emit",
        target: "message.append",
        stage: PipelineStage::AfterSuccess,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::EmitConversationMessageCreated,
    },
    PipelineHandler {
        id: "run.requested.emit",
        target: "run.create",
        stage: PipelineStage::AfterSuccess,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::EmitRunRequested,
    },
];

pub(crate) async fn dispatch_action_pipeline(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: JsonValue,
    context: JsonValue,
) -> Result<JsonValue, ApiError> {
    let mut params = params;
    run_pipeline_stage(
        state,
        request,
        key,
        PipelineStage::Before,
        &context,
        &mut params,
        None,
        None,
    )
    .await;
    ensure_action_execute_available(state, key, request)?;
    match execute_action_rules(state, request, key, &params, &context).await {
        Ok(result) => {
            run_pipeline_stage(
                state,
                request,
                key,
                PipelineStage::AfterSuccess,
                &context,
                &mut params,
                Some(&result),
                None,
            )
            .await;
            Ok(result)
        }
        Err(error) => {
            run_pipeline_stage(
                state,
                request,
                key,
                PipelineStage::AfterError,
                &context,
                &mut params,
                None,
                Some(&error),
            )
            .await;
            Err(error)
        }
    }
}

async fn run_pipeline_stage(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    stage: PipelineStage,
    _context: &JsonValue,
    params: &mut JsonValue,
    result: Option<&JsonValue>,
    error: Option<&ApiError>,
) {
    let mut handlers = PIPELINE_HANDLERS
        .iter()
        .copied()
        .filter(|handler| handler.enabled && handler.target == key && handler.stage == stage)
        .collect::<Vec<_>>();
    handlers.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| left.id.cmp(right.id))
    });

    for handler in handlers {
        match handler.action {
            PipelineHandlerAction::EmitConversationCreated => {
                if let Some(payload) = result {
                    emit_conversation_created(state, request, payload);
                }
            }
            PipelineHandlerAction::EmitConversationDeleted => {
                if let Some(payload) = result {
                    emit_conversation_deleted(state, request, Some(params), payload);
                }
            }
            PipelineHandlerAction::EmitConversationMessageCreated => {
                if let Some(payload) = result {
                    emit_conversation_message_created(state, request, payload);
                    drive_operator_message_received(state, request, payload).await;
                }
            }
            PipelineHandlerAction::EmitRunRequested => {
                if let Some(payload) = result {
                    emit_run_requested(state, request, payload);
                }
            }
        }
    }

    if stage == PipelineStage::AfterError {
        if let Some(error) = error {
            let _ = state.logs.append_log_scoped(
                LogEntryWrite {
                    event: "runtime.pipeline.action_failed".to_string(),
                    level: "warn".to_string(),
                    component: LOGS_COMPONENT_PROXY.to_string(),
                    source_kind: "pipeline".to_string(),
                    source_id: Some(key.to_string()),
                    message: "action pipeline failed".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
        }
    }
}

async fn execute_action_rules(
    state: &AppState,
    request: &RequestContext,
    key: &str,
    params: &JsonValue,
    context: &JsonValue,
) -> Result<JsonValue, ApiError> {
    let rules = action_rules_for_key(state, key, Some(ActionPhase::Execute));
    let mut aggregate = JsonValue::Null;
    let mut matched = false;

    for rule in rules {
        if !action_rule_matches_when(&rule.action.when, params) {
            continue;
        }
        matched = true;
        let value = dispatch_action_rule_execute(
            state,
            request,
            key,
            &rule,
            params.clone(),
            context.clone(),
        )
        .await?;
        aggregate_rule_result(&mut aggregate, &rule.action.result_mode, value);
    }

    if matched {
        Ok(aggregate)
    } else {
        Err(scoped(
            ApiError::bad_request(format!("action '{key}' has no matching execute rule")),
            request,
        ))
    }
}

fn aggregate_rule_result(target: &mut JsonValue, mode: &ActionResultMode, value: JsonValue) {
    match mode {
        ActionResultMode::Void => {}
        ActionResultMode::First => {
            if target.is_null() {
                *target = value;
            }
        }
        ActionResultMode::Last => {
            *target = value;
        }
        ActionResultMode::Collect => {
            if let JsonValue::Array(items) = target {
                items.push(value);
            } else {
                let previous = std::mem::replace(target, JsonValue::Null);
                *target = if previous.is_null() {
                    JsonValue::Array(vec![value])
                } else {
                    JsonValue::Array(vec![previous, value])
                };
            }
        }
        ActionResultMode::Merge => merge_json_value(target, value),
    }
}

fn merge_json_value(target: &mut JsonValue, value: JsonValue) {
    match (target, value) {
        (JsonValue::Object(current), JsonValue::Object(next)) => {
            for (key, value) in next {
                current.insert(key, value);
            }
        }
        (slot, next) if slot.is_null() => {
            *slot = next;
        }
        (slot, next) => {
            *slot = next;
        }
    }
}

fn action_rule_matches_when(when: &JsonValue, params: &JsonValue) -> bool {
    if when.is_null() {
        return true;
    }
    if let Some(allowed_roles) = when.get("message_role_in").and_then(JsonValue::as_array) {
        let role = params
            .get("message")
            .and_then(|item| item.get("role"))
            .and_then(JsonValue::as_str)
            .unwrap_or_default();
        return allowed_roles
            .iter()
            .filter_map(JsonValue::as_str)
            .any(|item| item == role);
    }
    true
}

fn emit_conversation_created(state: &AppState, request: &RequestContext, payload: &JsonValue) {
    let resource_id = payload
        .get("conversation")
        .and_then(|item| item.get("id"))
        .or_else(|| payload.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    dispatch_hook_event(
        state,
        request,
        HOOK_EVENT_CONVERSATION_CREATED,
        "conversation",
        resource_id,
        payload.clone(),
    );
    state.realtime.publish(RealtimeEvent::ConversationsChanged);
    state.realtime.publish(RealtimeEvent::ConversationChanged {
        conversation_id: resource_id.to_string(),
    });
}

fn emit_conversation_deleted(
    state: &AppState,
    request: &RequestContext,
    params: Option<&JsonValue>,
    payload: &JsonValue,
) {
    if !payload
        .get("deleted")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false)
    {
        return;
    }
    let resource_id = payload
        .get("conversation_id")
        .or_else(|| params.and_then(|item| item.get("conversation_id")))
        .or_else(|| payload.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    dispatch_hook_event(
        state,
        request,
        "conversation.deleted",
        "conversation",
        resource_id,
        payload.clone(),
    );
    state.realtime.publish(RealtimeEvent::ConversationsChanged);
}

fn emit_conversation_message_created(
    state: &AppState,
    request: &RequestContext,
    payload: &JsonValue,
) {
    let conversation_id = payload
        .get("conversation")
        .and_then(|item| item.get("id"))
        .or_else(|| {
            payload
                .get("message")
                .and_then(|item| item.get("conversation_id"))
        })
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    let resource_id = payload
        .get("message")
        .and_then(|item| item.get("id"))
        .or_else(|| payload.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    dispatch_hook_event(
        state,
        request,
        HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
        "message",
        resource_id,
        payload.clone(),
    );
    state.realtime.publish(RealtimeEvent::ConversationsChanged);
    state.realtime.publish(RealtimeEvent::ConversationChanged {
        conversation_id: conversation_id.to_string(),
    });
}

async fn drive_operator_message_received(
    state: &AppState,
    request: &RequestContext,
    payload: &JsonValue,
) {
    let role = payload
        .get("message")
        .and_then(|item| item.get("role"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if !matches!(role, "operator" | "user") {
        return;
    }

    let handlers = state.extensions.pipeline_handlers_for(
        PIPELINE_EVENT_OPERATOR_MESSAGE_RECEIVED,
        Some(PipelineHandlerStage::Drive),
        Some(PIPELINE_SLOT_CONVERSATION_RESPONSE),
    );
    if handlers.is_empty() {
        return;
    }

    for contribution in handlers {
        let rpc_request = ennoia_kernel::ExtensionRpcRequest {
            params: payload.clone(),
            context: serde_json::json!({
                "pipeline": {
                    "event": PIPELINE_EVENT_OPERATOR_MESSAGE_RECEIVED,
                    "stage": "drive",
                    "slot": PIPELINE_SLOT_CONVERSATION_RESPONSE,
                    "handler_id": contribution.handler.id,
                    "activation": resolve_pipeline_activation(
                        state,
                        &contribution.extension_id,
                        contribution.handler.activation.as_ref(),
                        payload,
                    ),
                    "trace": {
                        "request_id": request.request_id,
                    }
                }
            }),
        };
        let handler_id = contribution.handler.id.clone();
        let extension_id = contribution.extension_id.clone();
        let operation = contribution.handler.operation.clone();
        match dispatch_extension_rpc(state, &extension_id, &operation, rpc_request).await {
            Ok(response) if response.ok => {
                if let Ok(outcome) =
                    serde_json::from_value::<PipelineHandlerResponse>(response.data)
                {
                    match outcome.outcome.as_str() {
                        "claim" | "complete" => {
                            record_pipeline_drive_outcome(
                                state,
                                request,
                                &extension_id,
                                &handler_id,
                                &operation,
                                &outcome,
                                "ok",
                            );
                            return;
                        }
                        "skip" | "continue" => {
                            record_pipeline_drive_outcome(
                                state,
                                request,
                                &extension_id,
                                &handler_id,
                                &operation,
                                &outcome,
                                "skipped",
                            );
                        }
                        "fail" => {
                            record_pipeline_drive_outcome(
                                state,
                                request,
                                &extension_id,
                                &handler_id,
                                &operation,
                                &outcome,
                                "error",
                            );
                        }
                        other => {
                            let outcome = PipelineHandlerResponse {
                                outcome: other.to_string(),
                                message: Some("unknown pipeline outcome".to_string()),
                                ..PipelineHandlerResponse::default()
                            };
                            record_pipeline_drive_outcome(
                                state,
                                request,
                                &extension_id,
                                &handler_id,
                                &operation,
                                &outcome,
                                "warn",
                            );
                        }
                    }
                }
            }
            Ok(response) => {
                let message = response
                    .error
                    .map(|error| format!("{}: {}", error.code, error.message))
                    .unwrap_or_else(|| "pipeline handler returned failure".to_string());
                let outcome = PipelineHandlerResponse {
                    outcome: "fail".to_string(),
                    message: Some(message),
                    ..PipelineHandlerResponse::default()
                };
                record_pipeline_drive_outcome(
                    state,
                    request,
                    &extension_id,
                    &handler_id,
                    &operation,
                    &outcome,
                    "error",
                );
            }
            Err(error) => {
                let outcome = PipelineHandlerResponse {
                    outcome: "fail".to_string(),
                    message: Some(error.to_string()),
                    ..PipelineHandlerResponse::default()
                };
                record_pipeline_drive_outcome(
                    state,
                    request,
                    &extension_id,
                    &handler_id,
                    &operation,
                    &outcome,
                    "error",
                );
            }
        }
    }
}

fn resolve_pipeline_activation(
    state: &AppState,
    extension_id: &str,
    activation: Option<&PipelineHandlerActivationSpec>,
    payload: &JsonValue,
) -> JsonValue {
    let Some(activation) = activation else {
        return serde_json::json!({ "enabled": true });
    };
    let scope_id = match activation.scope {
        PipelineActivationScope::Conversation => payload
            .get("conversation")
            .and_then(|item| item.get("id"))
            .or_else(|| {
                payload
                    .get("message")
                    .and_then(|item| item.get("conversation_id"))
            })
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown"),
        PipelineActivationScope::Agent => payload
            .get("message")
            .and_then(|item| item.get("sender"))
            .and_then(JsonValue::as_str)
            .unwrap_or("unknown"),
        PipelineActivationScope::Space => payload
            .get("conversation")
            .and_then(|item| item.get("space_id"))
            .and_then(JsonValue::as_str)
            .unwrap_or("default"),
        PipelineActivationScope::Global => "global",
    };
    let scope_type = match activation.scope {
        PipelineActivationScope::Conversation => "conversation",
        PipelineActivationScope::Agent => "agent",
        PipelineActivationScope::Space => "space",
        PipelineActivationScope::Global => "global",
    };
    let stored = state
        .extension_runtime_store
        .get_state(&ExtensionStateGetQuery {
            extension_id: extension_id.to_string(),
            namespace: "pipeline.activation".to_string(),
            scope_type: scope_type.to_string(),
            scope_id: scope_id.to_string(),
            key: activation.key.clone(),
        })
        .ok()
        .flatten();
    let enabled = stored
        .as_ref()
        .and_then(|entry| entry.value.as_bool())
        .unwrap_or(activation.default);
    serde_json::json!({
        "enabled": enabled,
        "scope": scope_type,
        "scope_id": scope_id,
        "key": activation.key,
        "default": activation.default,
        "label": activation.label,
    })
}

fn record_pipeline_drive_outcome(
    state: &AppState,
    request: &RequestContext,
    extension_id: &str,
    handler_id: &str,
    operation: &str,
    outcome: &PipelineHandlerResponse,
    level: &str,
) {
    let _ = state.logs.append_log_scoped(
        LogEntryWrite {
            event: "runtime.pipeline.handler_outcome".to_string(),
            level: level.to_string(),
            component: LOGS_COMPONENT_PROXY.to_string(),
            source_kind: "pipeline_handler".to_string(),
            source_id: Some(handler_id.to_string()),
            message: "pipeline handler returned outcome".to_string(),
            attributes: serde_json::json!({
                "extension_id": extension_id,
                "operation": operation,
                "outcome": outcome.outcome,
                "slot": outcome.slot,
                "run_id": outcome.run_id,
                "operation_id": outcome.operation_id,
                "message": outcome.message,
            }),
            created_at: None,
        },
        Some(&request.trace_context()),
    );
}

fn emit_run_requested(state: &AppState, request: &RequestContext, payload: &JsonValue) {
    let run_id = payload
        .get("run")
        .and_then(|item| item.get("id"))
        .or_else(|| payload.get("run_id"))
        .or_else(|| payload.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    dispatch_hook_event(
        state,
        request,
        HOOK_EVENT_RUN_REQUESTED,
        "run",
        run_id,
        payload.clone(),
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pipeline_handlers_include_run_requested_after_run_create_success() {
        assert!(PIPELINE_HANDLERS.iter().any(|handler| {
            handler.id == "run.requested.emit"
                && handler.target == "run.create"
                && handler.stage == PipelineStage::AfterSuccess
                && handler.priority == 300
                && handler.enabled
                && matches!(handler.action, PipelineHandlerAction::EmitRunRequested)
        }));
    }
}
