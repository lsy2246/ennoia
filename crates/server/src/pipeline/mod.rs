use ennoia_contract::ApiError;
use ennoia_kernel::{
    ActionPhase, ActionResultMode, HOOK_EVENT_CONVERSATION_CREATED,
    HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};
use ennoia_logs::RequestContext;
use serde_json::Value as JsonValue;

use crate::app::AppState;
use crate::logs_store::{LogEntryWrite, LOGS_COMPONENT_PROXY};
use crate::routes::{
    actions::{
        action_rules_for_key, dispatch_action_rule_execute, dispatch_hook_event,
        ensure_action_execute_available,
    },
    scoped,
};

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
}

fn emit_conversation_message_created(
    state: &AppState,
    request: &RequestContext,
    payload: &JsonValue,
) {
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
}
