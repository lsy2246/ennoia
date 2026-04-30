use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Duration;

use ennoia_contract::ApiError;
use ennoia_extension_host::RegisteredProviderContribution;
use ennoia_kernel::{
    ActionPhase, ActionResultMode, AgentConfig, OwnerKind, OwnerRef, PermissionApprovalRecord,
    PermissionRequest, PermissionScope, PermissionTarget, PermissionTrigger, ProviderConfig,
    RunContext, HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED,
};
use ennoia_observability::RequestContext;
use serde::Serialize;
use serde_json::Value as JsonValue;

use crate::app::{load_agent_configs, load_provider_configs, AppState};
use crate::observability::{ObservationLogWrite, OBSERVABILITY_COMPONENT_PROXY};
use crate::routes::{
    actions::{
        action_rules_for_key, dispatch_action_rule_execute, dispatch_action_value,
        dispatch_action_value_with_context, dispatch_hook_event, ensure_action_execute_available,
    },
    scoped,
};

const AGENT_REPLY_DELAY_MS: u64 = 700;
const PROVIDER_NODE_RUNNER: &str = r#"
import { pathToFileURL } from 'node:url';

const entry = process.argv[1];
const mod = await import(pathToFileURL(entry).href);
const chunks = [];
for await (const chunk of process.stdin) {
  chunks.push(chunk);
}
const raw = Buffer.concat(chunks).toString('utf8').trim();
const request = raw ? JSON.parse(raw) : {};
const params = request.params ?? {};
const result = request.method === 'list_models'
  ? await mod.listModels(params)
  : await mod.generate(params);
process.stdout.write(JSON.stringify({ ok: true, result }));
"#;

#[derive(Debug, Serialize)]
struct AgentProviderInstructions {
    base: String,
}

#[derive(Debug, Serialize)]
struct AgentProviderContext {
    kind: &'static str,
    runtime: AgentRuntimeContext,
    conversation: AgentConversationContext,
    extensions: Vec<AgentExtensionContext>,
    skills: Vec<AgentSkillContext>,
}

#[derive(Debug, Serialize)]
struct AgentRuntimeContext {
    agent_id: String,
    agent_display_name: String,
    run_id: String,
    runtime_home: String,
    agent_working_dir: String,
    agent_artifacts_dir: String,
}

#[derive(Debug, Serialize)]
struct AgentConversationContext {
    conversation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    lane_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentExtensionContext {
    id: String,
    name: String,
    description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<String>,
    resource_types: Vec<AgentResourceTypeContext>,
    capabilities: Vec<AgentCapabilityContext>,
}

#[derive(Debug, Serialize)]
struct AgentResourceTypeContext {
    id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    content_kind: String,
    operations: Vec<String>,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AgentCapabilityContext {
    id: String,
    contract: String,
    kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
}

#[derive(Debug, Serialize)]
struct AgentSkillContext {
    id: String,
    display_name: String,
    description: String,
    entry: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    docs: Option<String>,
    keywords: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineStage {
    Before,
    AfterSuccess,
    AfterError,
}

#[derive(Debug, Clone, Copy)]
enum PipelineHandlerAction {
    BuildRunMemoryContext,
    EmitConversationCreated,
    EmitConversationDeleted,
    EmitConversationMessageCreated,
    QueueConversationAutoReply,
    RememberWorkflowRun,
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
        id: "message.created.emit.user",
        target: "message.append",
        stage: PipelineStage::AfterSuccess,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::EmitConversationMessageCreated,
    },
    PipelineHandler {
        id: "message.created.auto_run.user",
        target: "message.append",
        stage: PipelineStage::AfterSuccess,
        priority: 200,
        enabled: true,
        action: PipelineHandlerAction::QueueConversationAutoReply,
    },
    PipelineHandler {
        id: "run.context.memory",
        target: "run.create",
        stage: PipelineStage::Before,
        priority: 300,
        enabled: true,
        action: PipelineHandlerAction::BuildRunMemoryContext,
    },
    PipelineHandler {
        id: "run.memory.remember",
        target: "run.create",
        stage: PipelineStage::AfterSuccess,
        priority: 100,
        enabled: true,
        action: PipelineHandlerAction::RememberWorkflowRun,
    },
    PipelineHandler {
        id: "message.created.emit.agent",
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
    context: &JsonValue,
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
            PipelineHandlerAction::BuildRunMemoryContext => {
                enrich_run_create_params_with_memory_context(state, request, params, context).await;
            }
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
            PipelineHandlerAction::QueueConversationAutoReply => {
                if let Some(payload) = result.filter(|payload| should_queue_auto_reply(payload)) {
                    queue_conversation_message_pipeline(
                        state.clone(),
                        request.clone(),
                        payload.clone(),
                    );
                }
            }
            PipelineHandlerAction::RememberWorkflowRun => {
                if let Some(payload) = result {
                    remember_workflow_run_from_pipeline(state, request, params, payload, context)
                        .await;
                }
            }
        }
    }

    if stage == PipelineStage::AfterError {
        if let Some(error) = error {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.pipeline.action_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
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

fn should_queue_auto_reply(payload: &JsonValue) -> bool {
    matches!(
        payload
            .get("message")
            .and_then(|item| item.get("role"))
            .and_then(JsonValue::as_str),
        Some("operator" | "user")
    )
}

async fn enrich_run_create_params_with_memory_context(
    state: &AppState,
    request: &RequestContext,
    params: &mut JsonValue,
    context: &JsonValue,
) {
    if params.get("context").is_some_and(|value| !value.is_null()) {
        return;
    }
    let Some(conversation_id) = run_create_conversation_id(params) else {
        return;
    };
    let owner = run_create_owner(params).unwrap_or_else(|| OwnerRef::global("runtime"));
    let recent_messages = match execute_action_rules(
        state,
        request,
        "message.list",
        &serde_json::json!({ "conversation_id": conversation_id }),
        &JsonValue::Null,
    )
    .await
    {
        Ok(messages) => visible_recent_messages(&messages, "operator"),
        Err(_) => Vec::new(),
    };
    let Some(run_context) = assemble_workflow_memory_context_from_rules(
        state,
        request,
        &owner,
        &conversation_id,
        recent_messages,
        context.clone(),
    )
    .await
    else {
        return;
    };
    if let Some(object) = params.as_object_mut() {
        object.insert(
            "context".to_string(),
            serde_json::to_value(run_context).unwrap_or(JsonValue::Null),
        );
    }
}

async fn remember_workflow_run_from_pipeline(
    state: &AppState,
    request: &RequestContext,
    params: &JsonValue,
    payload: &JsonValue,
    context: &JsonValue,
) {
    let Some(conversation_id) = run_create_conversation_id(params) else {
        return;
    };
    let owner = run_create_owner(params).unwrap_or_else(|| OwnerRef::global("runtime"));
    let goal = params
        .get("goal")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    let lane_id = run_create_lane_id(params);
    let message_id = run_create_message_id(params);
    let run_id = payload
        .get("run")
        .and_then(|item| item.get("id"))
        .and_then(JsonValue::as_str);
    let agent_id = params
        .get("participants")
        .and_then(JsonValue::as_array)
        .and_then(|items| items.first())
        .and_then(JsonValue::as_str)
        .unwrap_or("runtime")
        .to_string();
    let actor_context = if context.is_null() {
        permission_actor_context(
            &agent_id,
            "pipeline.workflow_to_memory",
            true,
            Some(&conversation_id),
            run_id,
            message_id.as_deref(),
        )
    } else {
        context.clone()
    };
    remember_workflow_run(
        state,
        request,
        &owner,
        &conversation_id,
        lane_id.as_deref(),
        &goal,
        &agent_id,
        message_id.as_deref(),
        run_id,
        payload,
        actor_context,
    )
    .await;
}

fn run_create_owner(params: &JsonValue) -> Option<OwnerRef> {
    serde_json::from_value(params.get("owner")?.clone()).ok()
}

fn run_create_conversation_id(params: &JsonValue) -> Option<String> {
    params
        .get("source_refs")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| {
            item.get("conversation_id")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
                .or_else(|| {
                    (item.get("kind").and_then(JsonValue::as_str) == Some("conversation"))
                        .then(|| {
                            item.get("id")
                                .and_then(JsonValue::as_str)
                                .map(str::to_string)
                        })
                        .flatten()
                })
        })
}

fn run_create_lane_id(params: &JsonValue) -> Option<String> {
    params
        .get("source_refs")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| {
            item.get("lane_id")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
        })
}

fn run_create_message_id(params: &JsonValue) -> Option<String> {
    params
        .get("source_refs")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .find_map(|item| {
            item.get("message_id")
                .and_then(JsonValue::as_str)
                .map(str::to_string)
        })
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

pub(crate) fn queue_conversation_message_pipeline(
    state: AppState,
    request: RequestContext,
    payload: JsonValue,
) {
    tokio::spawn(async move {
        tokio::time::sleep(Duration::from_millis(AGENT_REPLY_DELAY_MS)).await;
        if let Err(error) = generate_conversation_agent_reply(&state, &request, &payload).await {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.pipeline.conversation_reply_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "pipeline".to_string(),
                    source_id: payload_string_field(&payload, &["conversation", "id"]).or_else(
                        || payload_string_field(&payload, &["message", "conversation_id"]),
                    ),
                    message: "conversation to workflow pipeline failed".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
        }
    });
}

pub(crate) fn queue_permission_approval_resume(
    state: AppState,
    request: RequestContext,
    approval: PermissionApprovalRecord,
) {
    if approval.status != "approved" || approval.action != "provider.generate" {
        return;
    }
    let conversation_id = approval.scope.conversation_id.clone();
    let message_id = approval.scope.message_id.clone();
    let agent_id = approval.agent_id.clone();
    tokio::spawn(async move {
        let Some(conversation_id) = conversation_id else {
            return;
        };
        let Some(message_id) = message_id else {
            return;
        };
        tokio::time::sleep(Duration::from_millis(AGENT_REPLY_DELAY_MS)).await;
        if let Err(error) = generate_conversation_agent_reply_from_permission(
            &state,
            &request,
            &conversation_id,
            &message_id,
            &agent_id,
        )
        .await
        {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.pipeline.permission_resume_failed".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "pipeline".to_string(),
                    source_id: Some(approval.approval_id),
                    message: "approved permission could not resume pipeline reply".to_string(),
                    attributes: serde_json::json!({
                        "agent_id": agent_id,
                        "conversation_id": conversation_id,
                        "message_id": message_id,
                        "error": error.to_string(),
                    }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
        }
    });
}

async fn generate_conversation_agent_reply_from_permission(
    state: &AppState,
    request: &RequestContext,
    conversation_id: &str,
    message_id: &str,
    agent_id: &str,
) -> Result<(), ApiError> {
    let messages = dispatch_action_value(
        state,
        request,
        "message.list",
        serde_json::json!({ "conversation_id": conversation_id }),
    )
    .await?;
    let Some(message) = messages
        .as_array()
        .and_then(|items| {
            items
                .iter()
                .find(|item| item.get("id").and_then(JsonValue::as_str) == Some(message_id))
        })
        .cloned()
    else {
        return Err(scoped(
            ApiError::not_found(format!("conversation message '{message_id}' not found")),
            request,
        ));
    };
    let payload = serde_json::json!({
        "conversation": { "id": conversation_id },
        "message": message,
        "addressed_agents": [agent_id],
    });
    generate_conversation_agent_reply(state, request, &payload).await
}

async fn generate_conversation_agent_reply(
    state: &AppState,
    request: &RequestContext,
    payload: &JsonValue,
) -> Result<(), ApiError> {
    let role = payload_string_field(payload, &["message", "role"])
        .unwrap_or_else(|| "operator".to_string());
    if role != "operator" {
        return Ok(());
    }

    let conversation_id = payload_string_field(payload, &["conversation", "id"])
        .or_else(|| payload_string_field(payload, &["message", "conversation_id"]))
        .ok_or_else(|| scoped(ApiError::internal("conversation id missing"), request))?;
    let lane_id = payload_string_field(payload, &["lane", "id"])
        .or_else(|| payload_string_field(payload, &["message", "lane_id"]));
    let body = payload_string_field(payload, &["message", "body"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let message_id = payload_string_field(payload, &["message", "id"]);
    let addressed_agents = {
        let explicit = payload_string_array_field(payload, &["addressed_agents"]);
        if explicit.is_empty() {
            payload_string_array_field(payload, &["message", "addressed_agents"])
        } else {
            explicit
        }
    };
    if body.is_empty() || addressed_agents.is_empty() {
        return Ok(());
    }

    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let owner = payload_owner(payload).unwrap_or_else(|| OwnerRef::global("runtime"));
    let agent_runtime_paths = addressed_agents
        .iter()
        .filter_map(|agent_id| {
            agents
                .iter()
                .find(|agent| agent.id == *agent_id)
                .map(|agent| {
                    serde_json::json!({
                        "agent_id": agent.id,
                        "display_name": agent.display_name,
                        "working_dir": agent.working_dir,
                        "artifacts_dir": agent.artifacts_dir,
                    })
                })
        })
        .collect::<Vec<_>>();

    for agent_id in &addressed_agents {
        let actor_context = permission_actor_context(
            agent_id,
            "pipeline.conversation_to_workflow",
            true,
            Some(&conversation_id),
            None,
            message_id.as_deref(),
        );
        let conversation_messages = dispatch_action_value_with_context(
            state,
            request,
            "message.list",
            serde_json::json!({ "conversation_id": conversation_id }),
            actor_context.clone(),
        )
        .await?;
        let memory_context = assemble_workflow_memory_context(
            state,
            request,
            &owner,
            &conversation_id,
            visible_recent_messages(&conversation_messages, agent_id),
            actor_context.clone(),
        )
        .await;
        let mut metadata = serde_json::json!({
            "origin": "pipeline.conversation_to_workflow",
            "message_id": message_id,
            "runtime_home": state.runtime_paths.display_for_user(state.runtime_paths.home()),
            "agent_paths": agent_runtime_paths,
        });
        if let Some(context) = memory_context.as_ref() {
            metadata["memory_context_injected"] = JsonValue::Bool(true);
            metadata["memory_context_total_chars"] =
                JsonValue::from(u64::from(context.total_chars));
        }
        let run_response = dispatch_action_value_with_context(
            state,
            request,
            "run.create",
            serde_json::json!({
                "owner": owner,
                "goal": body,
                "trigger": "conversation_message",
                "participants": [agent_id.clone()],
                "addressed_agents": [agent_id.clone()],
                "context": memory_context,
                "source_refs": [{
                    "kind": "conversation",
                    "id": conversation_id,
                    "conversation_id": conversation_id,
                    "lane_id": lane_id,
                    "message_id": message_id,
                }],
                "metadata": metadata
            }),
            actor_context.clone(),
        )
        .await?;
        let run_id = run_response
            .get("run")
            .and_then(|item| item.get("id"))
            .and_then(JsonValue::as_str);
        let reply_body = match generate_real_conversation_agent_reply(
            state,
            request,
            &agents,
            &providers,
            &conversation_id,
            lane_id.as_deref(),
            message_id.as_deref(),
            &conversation_messages,
            &run_response,
            agent_id,
        )
        .await
        {
            Ok(reply) => reply,
            Err(error) if is_permission_approval_error(&error) => continue,
            Err(error) => error.to_string(),
        };
        let _ = dispatch_action_value_with_context(
            state,
            request,
            "message.append",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": reply_body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "agent",
                    "addressed_agents": ["operator"],
                }
            }),
            permission_actor_context(
                agent_id,
                "pipeline.workflow_to_conversation",
                true,
                Some(&conversation_id),
                run_id,
                message_id.as_deref(),
            ),
        )
        .await?;
    }

    Ok(())
}

async fn generate_real_conversation_agent_reply(
    state: &AppState,
    request: &RequestContext,
    agents: &[AgentConfig],
    providers: &[ProviderConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    run_response: &JsonValue,
    agent_id: &str,
) -> Result<String, ApiError> {
    let agent = agents
        .iter()
        .find(|item| item.id == agent_id)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("agent '{agent_id}' not found")),
                request,
            )
        })?;
    let provider = providers
        .iter()
        .find(|item| item.id == agent.provider_id && item.enabled)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("provider '{}' not found", agent.provider_id)),
                request,
            )
        })?;
    let contribution = resolve_provider_contribution_for_generate(state, provider, request)?;
    let entry = resolve_provider_entry_path(&contribution)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    let model_id = if agent.model_id.trim().is_empty() {
        provider.default_model.trim().to_string()
    } else {
        agent.model_id.trim().to_string()
    };
    if model_id.is_empty() {
        return Err(scoped(
            ApiError::bad_request(format!("agent '{}' has no model configured", agent.id)),
            request,
        ));
    }

    let run_id = run_response
        .get("run")
        .and_then(|item| item.get("id"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string();
    let messages = normalize_conversation_messages_for_provider(conversation_messages, agent_id);
    let instructions = build_agent_provider_instructions(state, agent, &run_id);
    let context =
        build_agent_provider_context(state, agent, conversation_id, lane_id, message_id, &run_id);
    let request_payload = serde_json::json!({
        "method": "generate",
        "params": {
            "provider": provider_runtime_request_config(provider),
            "model": model_id,
            "instructions": instructions,
            "system_prompt": build_agent_runtime_prompt(state, agent, &run_id),
            "context": context,
            "messages": messages,
            "generation_options": agent.generation_options,
            "metadata": {
                "conversation_id": conversation_id,
                "lane_id": lane_id,
                "message_id": message_id,
                "run_id": run_id,
                "runtime_home": state.runtime_paths.display_for_user(state.runtime_paths.home()),
                "working_dir": agent.working_dir,
                "artifacts_dir": agent.artifacts_dir,
                "agent_id": agent.id,
                "agent_display_name": agent.display_name,
            }
        }
    });
    let provider_grant_id = authorize_provider_generate(
        state,
        request,
        agent,
        provider,
        &contribution,
        conversation_id,
        &run_id,
        message_id,
    )?;
    let response = invoke_provider_method(&entry, &request_payload, provider)
        .map_err(|error| scoped(ApiError::internal(error), request))?;
    if let Some(grant_id) = provider_grant_id.as_deref() {
        state
            .agent_permissions
            .consume_grant(grant_id)
            .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    }
    let text = response
        .get("result")
        .and_then(|item| item.get("text"))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| scoped(ApiError::internal("provider returned empty text"), request))?;
    Ok(text)
}

async fn assemble_workflow_memory_context(
    state: &AppState,
    request: &RequestContext,
    owner: &OwnerRef,
    conversation_id: &str,
    recent_messages: Vec<String>,
    actor_context: JsonValue,
) -> Option<RunContext> {
    let owner_kind = owner_kind_str(&owner.kind);
    let result = dispatch_action_value_with_context(
        state,
        request,
        "memory.build_context",
        serde_json::json!({
            "owner_kind": owner_kind,
            "owner_id": owner.id,
            "conversation_id": conversation_id,
            "recent_messages": recent_messages,
            "active_tasks": [],
        }),
        actor_context,
    )
    .await;
    match result {
        Ok(value) => serde_json::from_value::<RunContext>(value).ok(),
        Err(error) => {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.pipeline.memory_context_skipped".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "pipeline".to_string(),
                    source_id: Some(conversation_id.to_string()),
                    message: "workflow memory context assembly skipped".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
            None
        }
    }
}

async fn assemble_workflow_memory_context_from_rules(
    state: &AppState,
    request: &RequestContext,
    owner: &OwnerRef,
    conversation_id: &str,
    recent_messages: Vec<String>,
    actor_context: JsonValue,
) -> Option<RunContext> {
    let owner_kind = owner_kind_str(&owner.kind);
    let payload = serde_json::json!({
        "owner_kind": owner_kind,
        "owner_id": owner.id,
        "conversation_id": conversation_id,
        "recent_messages": recent_messages,
        "active_tasks": [],
    });
    match execute_action_rules(
        state,
        request,
        "memory.build_context",
        &payload,
        &actor_context,
    )
    .await
    {
        Ok(value) => serde_json::from_value::<RunContext>(value).ok(),
        Err(error) => {
            let _ = state.observability.append_log_scoped(
                ObservationLogWrite {
                    event: "runtime.pipeline.memory_context_skipped".to_string(),
                    level: "warn".to_string(),
                    component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                    source_kind: "pipeline".to_string(),
                    source_id: Some(conversation_id.to_string()),
                    message: "workflow memory context assembly skipped".to_string(),
                    attributes: serde_json::json!({ "error": error.to_string() }),
                    created_at: None,
                },
                Some(&request.trace_context()),
            );
            None
        }
    }
}

async fn remember_workflow_run(
    state: &AppState,
    request: &RequestContext,
    owner: &OwnerRef,
    conversation_id: &str,
    lane_id: Option<&str>,
    goal: &str,
    agent_id: &str,
    message_id: Option<&str>,
    run_id: Option<&str>,
    run_response: &JsonValue,
    actor_context: JsonValue,
) {
    let Some(run_id) = run_id else {
        return;
    };
    let artifacts = run_response
        .get("artifacts")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let tasks = run_response
        .get("tasks")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default();
    let stage = run_response
        .get("run")
        .and_then(|item| item.get("stage"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown");
    let decision = run_response
        .get("decision")
        .and_then(|item| item.get("summary"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let artifact_refs = artifacts
        .iter()
        .filter_map(|item| item.get("id").and_then(JsonValue::as_str))
        .collect::<Vec<_>>();
    let content = format!(
        "workflow run={run_id}\nconversation_id={conversation_id}\nlane_id={}\nagent_id={agent_id}\nstage={stage}\ngoal={goal}\ndecision={decision}\ntasks={}\nartifacts={}",
        lane_id.unwrap_or(""),
        tasks.len(),
        artifact_refs.join(",")
    );
    let summary = format!(
        "Workflow run {run_id} 已记录，stage={stage}，tasks={}，artifacts={}",
        tasks.len(),
        artifacts.len()
    );
    let payload = serde_json::json!({
        "owner_kind": owner_kind_str(&owner.kind),
        "owner_id": owner.id,
        "namespace": format!("workflow/conversation/{conversation_id}"),
        "memory_kind": "observation",
        "stability": "working",
        "title": format!("Workflow run {run_id}"),
        "content": content,
        "summary": summary,
        "confidence": 0.55,
        "importance": 0.5,
        "sources": build_workflow_memory_sources(conversation_id, message_id, run_id, &artifacts),
        "tags": ["workflow", "run", stage, agent_id],
        "entities": [run_id, conversation_id, agent_id],
    });
    if let Err(error) =
        execute_action_rules(state, request, "memory.ingest", &payload, &actor_context).await
    {
        let _ = state.observability.append_log_scoped(
            ObservationLogWrite {
                event: "runtime.pipeline.workflow_memory_failed".to_string(),
                level: "warn".to_string(),
                component: OBSERVABILITY_COMPONENT_PROXY.to_string(),
                source_kind: "pipeline".to_string(),
                source_id: Some(run_id.to_string()),
                message: "workflow result could not be remembered".to_string(),
                attributes: serde_json::json!({ "error": error.to_string() }),
                created_at: None,
            },
            Some(&request.trace_context()),
        );
    }
}

fn build_workflow_memory_sources(
    conversation_id: &str,
    message_id: Option<&str>,
    run_id: &str,
    artifacts: &[JsonValue],
) -> Vec<JsonValue> {
    let mut sources = vec![
        serde_json::json!({
            "kind": "conversation",
            "reference": conversation_id,
        }),
        serde_json::json!({
            "kind": "workflow.run",
            "reference": run_id,
        }),
    ];
    if let Some(message_id) = message_id {
        sources.push(serde_json::json!({
            "kind": "conversation.message",
            "reference": message_id,
        }));
    }
    for artifact_id in artifacts
        .iter()
        .filter_map(|item| item.get("id").and_then(JsonValue::as_str))
    {
        sources.push(serde_json::json!({
            "kind": "workflow.artifact",
            "reference": artifact_id,
        }));
    }
    sources
}

fn provider_runtime_request_config(provider: &ProviderConfig) -> JsonValue {
    serde_json::json!({
        "id": provider.id,
        "kind": provider.kind,
        "base_url": provider.base_url,
        "api_key_env": provider.api_key_env,
        "default_model": provider.default_model,
    })
}

fn authorize_provider_generate(
    state: &AppState,
    request: &RequestContext,
    agent: &AgentConfig,
    provider: &ProviderConfig,
    contribution: &RegisteredProviderContribution,
    conversation_id: &str,
    run_id: &str,
    message_id: Option<&str>,
) -> Result<Option<String>, ApiError> {
    let permission_request = PermissionRequest {
        agent_id: agent.id.clone(),
        action: "provider.generate".to_string(),
        target: PermissionTarget {
            kind: "provider".to_string(),
            id: provider.id.clone(),
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            path: None,
            host: normalize_optional_runtime_value(&provider.base_url),
        },
        scope: PermissionScope {
            conversation_id: Some(conversation_id.to_string()),
            run_id: Some(run_id.to_string()),
            message_id: message_id.map(str::to_string),
            extension_id: Some(contribution.extension_id.clone()),
            path: None,
            host: normalize_optional_runtime_value(&provider.base_url),
        },
        trigger: PermissionTrigger {
            kind: "pipeline.workflow_to_conversation".to_string(),
            user_initiated: true,
        },
    };
    let decision = state
        .agent_permissions
        .evaluate_request(&permission_request, Some(request))
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    match decision.decision.as_str() {
        "allow" => Ok(decision.grant_id),
        "ask" => Err(scoped(
            ApiError::forbidden(format!(
                "approval required: action=provider.generate, approval_id={}",
                decision
                    .approval_id
                    .clone()
                    .unwrap_or_else(|| "unknown".to_string())
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
        )),
        _ => Err(scoped(
            ApiError::forbidden(format!(
                "permission denied: action=provider.generate, reason={}",
                decision.reason
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

fn resolve_provider_contribution_for_generate(
    state: &AppState,
    provider: &ProviderConfig,
    request: &RequestContext,
) -> Result<RegisteredProviderContribution, ApiError> {
    let matches = state
        .extensions
        .snapshot()
        .providers
        .into_iter()
        .filter(|item| item.provider.kind == provider.kind || item.provider.id == provider.kind)
        .filter(|item| {
            item.provider
                .interfaces
                .iter()
                .any(|name| name == "generate")
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(scoped(
            ApiError::not_found(format!(
                "provider kind '{}' has no generate implementation",
                provider.kind
            )),
            request,
        )),
        _ => Err(scoped(
            ApiError::conflict(format!(
                "provider kind '{}' has multiple generate implementations",
                provider.kind
            )),
            request,
        )),
    }
}

fn resolve_provider_entry_path(
    contribution: &RegisteredProviderContribution,
) -> std::io::Result<PathBuf> {
    let entry = contribution
        .provider
        .entry
        .as_deref()
        .ok_or_else(|| std::io::Error::other("provider entry missing"))?;
    let path = PathBuf::from(&contribution.install_dir).join(entry);
    std::fs::canonicalize(path)
}

fn normalize_conversation_messages_for_provider(
    conversation_messages: &JsonValue,
    agent_id: &str,
) -> Vec<JsonValue> {
    let mut messages = conversation_messages
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|message| message_visible_to_agent(message, agent_id))
        .rev()
        .take(24)
        .collect::<Vec<_>>();
    messages.reverse();
    messages
}

fn message_visible_to_agent(message: &JsonValue, agent_id: &str) -> bool {
    let role = message
        .get("role")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let sender = message
        .get("sender")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    match role {
        "operator" => {
            let mentions = message_mentions(message);
            mentions.is_empty() || mentions.iter().any(|mention| mention == agent_id)
        }
        "agent" => sender == agent_id && !looks_like_synthetic_agent_error(message),
        _ => false,
    }
}

fn message_mentions(message: &JsonValue) -> Vec<String> {
    message
        .get("mentions")
        .and_then(JsonValue::as_array)
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn looks_like_synthetic_agent_error(message: &JsonValue) -> bool {
    let body = message
        .get("body")
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    body.starts_with("error:")
        || body.contains("request failed:")
        || body.contains("empty completion")
        || body.contains("provider returned empty text")
}

fn build_agent_runtime_prompt(state: &AppState, agent: &AgentConfig, run_id: &str) -> String {
    let mut sections = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        sections.push(agent.system_prompt.trim().to_string());
    }
    sections.push(format!(
        "你当前运行在 Ennoia 会话系统中。\nagent_id：{}\nagent_name：{}\nrun_id：{}\nruntime_home：{}\nagent_working_dir：{}\nagent_artifacts_dir：{}\n`agent_working_dir` 和 `agent_artifacts_dir` 是当前 Agent 的内部运行目录，不等同于用户项目工作区。只有在用户明确询问路径、文件位置、产物位置，或者任务确实需要读写这些目录时才使用；否则不要主动向用户复述这些内部路径。直接回答用户，不要伪装成“系统已接收”或“正在处理中”。",
        agent.id,
        agent.display_name,
        if run_id.trim().is_empty() { "unknown" } else { run_id },
        state.runtime_paths.display_for_user(state.runtime_paths.home()),
        agent.working_dir,
        agent.artifacts_dir,
    ));
    sections.push(
        "系统会额外提供一份结构化 JSON 上下文，里面包含当前运行时、会话、已注入扩展目录和已启用技能目录。按字段理解并使用，不要向用户原样复述 JSON，也不要主动枚举内部路径、目录清单或所有可用能力，除非用户明确要求。"
            .to_string(),
    );
    sections.join("\n\n")
}

fn build_agent_provider_instructions(
    state: &AppState,
    agent: &AgentConfig,
    run_id: &str,
) -> AgentProviderInstructions {
    AgentProviderInstructions {
        base: build_agent_runtime_prompt(state, agent, run_id),
    }
}

fn build_agent_provider_context(
    state: &AppState,
    agent: &AgentConfig,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    run_id: &str,
) -> AgentProviderContext {
    AgentProviderContext {
        kind: "ennoia.agent_context",
        runtime: AgentRuntimeContext {
            agent_id: agent.id.clone(),
            agent_display_name: agent.display_name.clone(),
            run_id: normalize_unknown(run_id),
            runtime_home: state
                .runtime_paths
                .display_for_user(state.runtime_paths.home()),
            agent_working_dir: agent.working_dir.clone(),
            agent_artifacts_dir: agent.artifacts_dir.clone(),
        },
        conversation: AgentConversationContext {
            conversation_id: conversation_id.to_string(),
            lane_id: lane_id.map(str::to_string),
            message_id: message_id.map(str::to_string),
        },
        extensions: build_agent_extension_contexts(state),
        skills: build_agent_skill_contexts(state, agent),
    }
}

fn build_agent_extension_contexts(state: &AppState) -> Vec<AgentExtensionContext> {
    state
        .extensions
        .snapshot()
        .extensions
        .into_iter()
        .filter(|extension| extension.conversation.inject)
        .map(|extension| {
            let resource_types = extension
                .resource_types
                .iter()
                .filter(|resource_type| {
                    extension.conversation.resource_types.is_empty()
                        || extension
                            .conversation
                            .resource_types
                            .iter()
                            .any(|id| id == &resource_type.id)
                })
                .map(|resource_type| AgentResourceTypeContext {
                    id: resource_type.id.clone(),
                    title: resource_type
                        .title
                        .as_ref()
                        .map(|item| item.fallback.clone()),
                    content_kind: resource_type.content_kind.clone(),
                    operations: resource_type.operations.clone(),
                    tags: resource_type.tags.clone(),
                })
                .collect::<Vec<_>>();
            let capabilities = extension
                .capability_rows
                .iter()
                .filter(|capability| {
                    extension.conversation.capabilities.is_empty()
                        || extension
                            .conversation
                            .capabilities
                            .iter()
                            .any(|id| id == &capability.id)
                })
                .map(|capability| AgentCapabilityContext {
                    id: capability.id.clone(),
                    contract: capability.contract.clone(),
                    kind: capability.kind.clone(),
                    title: capability.title.as_ref().map(|item| item.fallback.clone()),
                })
                .collect::<Vec<_>>();
            AgentExtensionContext {
                id: extension.id.clone(),
                name: extension.name.clone(),
                description: normalize_catalog_text(&extension.description, "无描述"),
                docs: extension
                    .docs
                    .as_deref()
                    .map(|value| resolve_catalog_path(&extension.source_root, value)),
                resource_types,
                capabilities,
            }
        })
        .collect()
}

fn build_agent_skill_contexts(state: &AppState, agent: &AgentConfig) -> Vec<AgentSkillContext> {
    agent
        .skills
        .iter()
        .map(|item| item.trim())
        .filter(|item| !item.is_empty())
        .filter_map(|skill_id| {
            state
                .skills
                .iter()
                .find(|skill| skill.id == skill_id && skill.enabled)
        })
        .map(|skill| AgentSkillContext {
            id: skill.id.clone(),
            display_name: skill.display_name.clone(),
            description: normalize_catalog_text(&skill.description, "无描述"),
            entry: skill.entry.clone(),
            docs: skill.docs.as_deref().map(|value| {
                resolve_catalog_path(
                    &state
                        .runtime_paths
                        .display_for_user(state.runtime_paths.skill_dir(&skill.id)),
                    value,
                )
            }),
            keywords: skill.keywords.clone(),
        })
        .collect()
}

fn resolve_catalog_path(base: &str, value: &str) -> String {
    let candidate = PathBuf::from(value);
    if candidate.is_absolute() {
        return candidate.to_string_lossy().replace('\\', "/");
    }
    PathBuf::from(base)
        .join(value)
        .to_string_lossy()
        .replace('\\', "/")
}

fn normalize_catalog_text(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.replace('\n', " ")
    }
}

fn normalize_unknown(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        "unknown".to_string()
    } else {
        trimmed.to_string()
    }
}

fn normalize_optional_runtime_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn is_permission_approval_error(error: &ApiError) -> bool {
    error.message().starts_with("approval required:")
}

fn permission_actor_context(
    agent_id: &str,
    kind: &str,
    user_initiated: bool,
    conversation_id: Option<&str>,
    run_id: Option<&str>,
    message_id: Option<&str>,
) -> JsonValue {
    serde_json::json!({
        "permission_actor": {
            "agent_id": agent_id,
            "kind": kind,
            "user_initiated": user_initiated,
            "conversation_id": conversation_id,
            "run_id": run_id,
            "message_id": message_id,
        }
    })
}

fn invoke_provider_method(
    entry: &PathBuf,
    payload: &JsonValue,
    provider: &ProviderConfig,
) -> Result<JsonValue, String> {
    let payload_bytes = serde_json::to_vec(payload)
        .map_err(|error| format!("serialize provider request failed: {error}"))?;
    let entry_string = entry
        .to_str()
        .ok_or_else(|| "provider entry path is not valid utf-8".to_string())?
        .to_string();
    let mut command = Command::new("node");
    command
        .args([
            "--input-type=module",
            "-e",
            PROVIDER_NODE_RUNNER,
            &entry_string,
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if let Some((name, value)) = resolve_provider_env_binding(provider) {
        command.env(name, value);
    }
    let mut child = command
        .spawn()
        .map_err(|error| format!("spawn provider runner failed: {error}"))?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(&payload_bytes)
            .map_err(|error| format!("write provider request failed: {error}"))?;
    }
    let output = child
        .wait_with_output()
        .map_err(|error| format!("wait provider runner failed: {error}"))?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(if detail.is_empty() {
            format!("provider runner exited with status {}", output.status)
        } else {
            detail
        });
    }
    serde_json::from_slice::<JsonValue>(&output.stdout)
        .map_err(|error| format!("parse provider response failed: {error}"))
}

fn resolve_provider_env_binding(provider: &ProviderConfig) -> Option<(String, String)> {
    let env_name = provider.api_key_env.trim();
    if env_name.is_empty() {
        return None;
    }
    resolve_env_value(env_name).map(|value| (env_name.to_string(), value))
}

fn resolve_env_value(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .or_else(|| load_os_environment_value(name))
}

#[cfg(not(windows))]
fn load_os_environment_value(_name: &str) -> Option<String> {
    None
}

#[cfg(windows)]
fn load_os_environment_value(name: &str) -> Option<String> {
    use winreg::enums::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    use winreg::RegKey;

    let current_user = RegKey::predef(HKEY_CURRENT_USER);
    read_windows_environment_value(&current_user, "Environment", name).or_else(|| {
        let local_machine = RegKey::predef(HKEY_LOCAL_MACHINE);
        read_windows_environment_value(
            &local_machine,
            r"SYSTEM\CurrentControlSet\Control\Session Manager\Environment",
            name,
        )
    })
}

#[cfg(windows)]
fn read_windows_environment_value(hive: &winreg::RegKey, path: &str, name: &str) -> Option<String> {
    let key = hive.open_subkey(path).ok()?;
    key.get_value::<String, _>(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn payload_string_field(payload: &JsonValue, path: &[&str]) -> Option<String> {
    let mut current = payload;
    for segment in path {
        current = current.get(*segment)?;
    }
    current.as_str().map(str::to_string)
}

fn payload_string_array_field(payload: &JsonValue, path: &[&str]) -> Vec<String> {
    let mut current = payload;
    for segment in path {
        let Some(next) = current.get(*segment) else {
            return Vec::new();
        };
        current = next;
    }
    current
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
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

fn owner_kind_str(kind: &OwnerKind) -> &'static str {
    match kind {
        OwnerKind::Global => "global",
        OwnerKind::Agent => "agent",
        OwnerKind::Space => "space",
    }
}

fn visible_recent_messages(conversation_messages: &JsonValue, agent_id: &str) -> Vec<String> {
    normalize_conversation_messages_for_provider(conversation_messages, agent_id)
        .into_iter()
        .rev()
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .filter_map(|message| {
            let sender = message
                .get("sender")
                .and_then(JsonValue::as_str)
                .unwrap_or("unknown");
            let body = message
                .get("body")
                .and_then(JsonValue::as_str)
                .unwrap_or("")
                .trim();
            if body.is_empty() {
                None
            } else {
                Some(format!("{sender}: {body}"))
            }
        })
        .collect()
}
