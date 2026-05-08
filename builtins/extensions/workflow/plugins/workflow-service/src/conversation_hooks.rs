use std::fs;
use std::sync::Arc;

use chrono::Utc;
use ennoia_contract::behavior::{BehaviorRunRequest, BehaviorSourceRef, BehaviorTrigger};
use ennoia_contract::{ApiErrorBody, ErrorCode};
use ennoia_kernel::{
    AgentConfig, AgentDocument, DecisionSnapshot, ExtensionHostCapabilityRequest,
    ExtensionRecordAppend, ExtensionRecordEntry, ExtensionRecordUpdate, ExtensionRpcRequest,
    ExtensionStateEntry, ExtensionStateGetQuery, ExtensionStatePut, HookDispatchResponse,
    HookEventEnvelope, ModelEndpointConfig, NextAction, OwnerKind, OwnerRef,
    PermissionApprovalRecord, RunContext, RunStage, RunStageEvent, RuntimeOperationRequest,
    ServerConfig,
};
use ennoia_paths::RuntimePaths;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::Row;
use uuid::Uuid;

use crate::host_bridge::HostBridge;
use crate::pipeline::{run_behavior, WorkflowRuntime};
use crate::planning::{
    build_planning_prompt, parse_plan_from_text, summarize_plan_steps, validate_plan, PlanSpec,
};
use crate::runtime::{RuntimeStore, SqliteRuntimeStore};

#[derive(Debug)]
struct HostApiClient {
    processing_stale_after_ms: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct ToolSpec {
    name: String,
    description: String,
    parameters: JsonValue,
}

#[derive(Debug, Deserialize, Serialize)]
struct ProviderInstructions {
    base: String,
}

#[derive(Debug, Deserialize)]
struct AgentToolCall {
    id: String,
    name: String,
    #[serde(default)]
    arguments: JsonValue,
}

#[derive(Debug, Serialize)]
struct ToolMessageEnvelope {
    kind: &'static str,
    tool_call_id: String,
    tool_name: String,
    status: String,
    arguments: JsonValue,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<ToolMessageError>,
}

#[derive(Debug, Serialize)]
struct ToolMessageError {
    code: String,
    message: String,
    details: JsonValue,
}

#[derive(Debug, Serialize)]
struct ReasoningMessageEnvelope {
    kind: &'static str,
    format: &'static str,
    content: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct ActiveWorkflowSession {
    draft_id: String,
    record_id: String,
    branch_scope: String,
    status: String,
    revision: i64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct WorkflowDraftRow {
    id: String,
    conversation_id: String,
    agent_id: String,
    status: String,
    goal: String,
    summary: String,
    source_message_id: Option<String>,
    record_id: Option<String>,
    latest_revision: i64,
    plan: PlanSpec,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConversationRoute {
    DirectReply,
    ManagedDiscussion,
}

#[derive(Debug)]
struct HostApiError {
    body: ApiErrorBody,
}

impl std::fmt::Display for HostApiError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.body.message)
    }
}

impl std::error::Error for HostApiError {}

impl HostApiError {
    fn message(&self) -> &str {
        &self.body.message
    }

    fn code(&self) -> ErrorCode {
        self.body.code
    }

    fn details(&self) -> &JsonValue {
        &self.body.details
    }

    fn is_permission_approval(&self) -> bool {
        self.body.message.starts_with("approval required:")
    }
}

impl HostApiClient {
    fn new(runtime_paths: &Arc<RuntimePaths>) -> Result<Self, String> {
        let server_config = read_server_config(runtime_paths)?;
        let request_timeout_ms = server_config.timeout.default_ms.max(1_000);
        let processing_stale_after_ms = [
            request_timeout_ms,
            server_config.extension_runtime.timeout_ms,
            server_config.operations.command.max_timeout_ms,
            server_config.operations.net.max_timeout_ms,
        ]
        .into_iter()
        .max()
        .unwrap_or(30_000)
        .saturating_mul(2);
        Ok(Self {
            processing_stale_after_ms,
        })
    }

    async fn dispatch_action(
        &self,
        action: &str,
        params: JsonValue,
        context: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.call_json(ExtensionHostCapabilityRequest::ActionDispatch {
            action: action.to_string(),
            params,
            context,
        })
        .await
    }

    async fn provider_generate(
        &self,
        provider_kind: &str,
        payload: JsonValue,
        context: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.call_json(ExtensionHostCapabilityRequest::ProviderInvoke {
            provider_kind: provider_kind.to_string(),
            method: "generate".to_string(),
            payload: ExtensionRpcRequest {
                params: payload,
                context,
            },
        })
        .await
    }

    async fn execute_operation(
        &self,
        operation: &str,
        agent_id: &str,
        conversation_id: &str,
        run_id: &str,
        message_id: Option<&str>,
        arguments: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.call_json(ExtensionHostCapabilityRequest::RuntimeOperation {
            operation: operation.to_string(),
            payload: RuntimeOperationRequest {
                agent_id: agent_id.to_string(),
                conversation_id: conversation_id.to_string(),
                run_id: run_id.to_string(),
                message_id: message_id.map(str::to_string),
                arguments,
            },
        })
        .await
    }

    async fn get_extension_state(
        &self,
        extension_id: &str,
        namespace: &str,
        scope_type: &str,
        scope_id: &str,
        key: &str,
    ) -> Result<Option<JsonValue>, HostApiError> {
        match self
            .call_json(ExtensionHostCapabilityRequest::ExtensionStateGet {
                query: ExtensionStateGetQuery {
                    extension_id: extension_id.to_string(),
                    namespace: namespace.to_string(),
                    scope_type: scope_type.to_string(),
                    scope_id: scope_id.to_string(),
                    key: key.to_string(),
                },
            })
            .await
        {
            Ok(value) => serde_json::from_value::<ExtensionStateEntry>(value)
                .map(|entry| Some(entry.value))
                .map_err(|error| internal_error(format!("parse extension state failed: {error}"))),
            Err(error) if error.code() == ErrorCode::NotFound => Ok(None),
            Err(error) => Err(error),
        }
    }

    async fn put_extension_state(
        &self,
        extension_id: &str,
        namespace: &str,
        scope_type: &str,
        scope_id: &str,
        key: &str,
        value: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.call_json(ExtensionHostCapabilityRequest::ExtensionStatePut {
            payload: ExtensionStatePut {
                extension_id: extension_id.to_string(),
                namespace: namespace.to_string(),
                scope_type: scope_type.to_string(),
                scope_id: scope_id.to_string(),
                key: key.to_string(),
                value,
                expires_at: None,
            },
        })
        .await
    }

    async fn delete_extension_state(
        &self,
        extension_id: &str,
        namespace: &str,
        scope_type: &str,
        scope_id: &str,
        key: &str,
    ) -> Result<(), HostApiError> {
        self.call_json(ExtensionHostCapabilityRequest::ExtensionStateDelete {
            query: ExtensionStateGetQuery {
                extension_id: extension_id.to_string(),
                namespace: namespace.to_string(),
                scope_type: scope_type.to_string(),
                scope_id: scope_id.to_string(),
                key: key.to_string(),
            },
        })
        .await
        .map(|_| ())
    }

    async fn append_extension_record(
        &self,
        payload: JsonValue,
    ) -> Result<ExtensionRecordEntry, HostApiError> {
        let value = self
            .call_json(ExtensionHostCapabilityRequest::ExtensionRecordAppend {
                payload: serde_json::from_value::<ExtensionRecordAppend>(payload).map_err(
                    |error| {
                        internal_error(format!(
                            "parse extension record append request failed: {error}"
                        ))
                    },
                )?,
            })
            .await?;
        serde_json::from_value(value).map_err(|error| {
            internal_error(format!(
                "parse extension record append response failed: {error}"
            ))
        })
    }

    async fn update_extension_record(
        &self,
        payload: JsonValue,
    ) -> Result<ExtensionRecordEntry, HostApiError> {
        let value = self
            .call_json(ExtensionHostCapabilityRequest::ExtensionRecordUpdate {
                payload: serde_json::from_value::<ExtensionRecordUpdate>(payload).map_err(
                    |error| {
                        internal_error(format!(
                            "parse extension record update request failed: {error}"
                        ))
                    },
                )?,
            })
            .await?;
        serde_json::from_value(value).map_err(|error| {
            internal_error(format!(
                "parse extension record update response failed: {error}"
            ))
        })
    }

    async fn close_extension_record(
        &self,
        record_id: &str,
    ) -> Result<ExtensionRecordEntry, HostApiError> {
        let value = self
            .call_json(ExtensionHostCapabilityRequest::ExtensionRecordClose {
                record_id: record_id.to_string(),
            })
            .await?;
        serde_json::from_value(value).map_err(|error| {
            internal_error(format!(
                "parse extension record close response failed: {error}"
            ))
        })
    }

    async fn call_json(
        &self,
        request: ExtensionHostCapabilityRequest,
    ) -> Result<JsonValue, HostApiError> {
        let bridge = HostBridge::global().map_err(internal_error)?;
        let response = bridge.call(request).await.map_err(internal_error)?;
        if response.ok {
            Ok(response.data)
        } else {
            Err(host_response_error(response))
        }
    }
}

pub async fn handle_conversation_message_created(
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    envelope: HookEventEnvelope,
) -> Result<HookDispatchResponse, String> {
    let client = HostApiClient::new(&runtime.runtime_paths)?;
    generate_conversation_agent_reply(&client, runtime, store, &envelope.payload).await?;
    Ok(HookDispatchResponse {
        handled: true,
        result: None,
        message: None,
    })
}

pub async fn handle_permission_approval_resolved(
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    envelope: HookEventEnvelope,
) -> Result<HookDispatchResponse, String> {
    let approval: PermissionApprovalRecord = serde_json::from_value(
        envelope
            .payload
            .get("approval")
            .cloned()
            .unwrap_or(JsonValue::Null),
    )
    .map_err(|error| format!("parse permission approval payload failed: {error}"))?;
    if approval.status != "approved" {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    }
    let Some(conversation_id) = approval.scope.conversation_id.clone() else {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    };
    let Some(message_id) = approval.scope.message_id.clone() else {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    };
    let client = HostApiClient::new(&runtime.runtime_paths)?;
    let messages = client
        .dispatch_action(
            "message.list",
            serde_json::json!({ "conversation_id": conversation_id }),
            JsonValue::Null,
        )
        .await
        .map_err(|error| error.to_string())?;
    let Some(message) = messages.as_array().and_then(|items| {
        items
            .iter()
            .find(|item| item.get("id").and_then(JsonValue::as_str) == Some(message_id.as_str()))
    }) else {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: Some("message missing for approval resume".to_string()),
        });
    };
    let payload = serde_json::json!({
        "conversation": { "id": conversation_id },
        "message": message,
        "addressed_agents": [approval.agent_id],
        "workflow_resume_run_id": approval.scope.run_id,
    });
    generate_conversation_agent_reply(&client, runtime, store, &payload).await?;
    Ok(HookDispatchResponse {
        handled: true,
        result: None,
        message: None,
    })
}

async fn generate_conversation_agent_reply(
    client: &HostApiClient,
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    payload: &JsonValue,
) -> Result<(), String> {
    let role = payload_string_field(payload, &["message", "role"])
        .unwrap_or_else(|| "operator".to_string());
    if !matches!(role.as_str(), "operator" | "user") {
        return Ok(());
    }
    let conversation_id = payload_string_field(payload, &["conversation", "id"])
        .or_else(|| payload_string_field(payload, &["message", "conversation_id"]))
        .ok_or_else(|| "conversation id missing".to_string())?;
    let branch_id = payload_string_field(payload, &["branch", "id"])
        .or_else(|| payload_string_field(payload, &["message", "branch_id"]));
    let lane_id = payload_string_field(payload, &["lane", "id"])
        .or_else(|| payload_string_field(payload, &["message", "lane_id"]));
    let branch_scope = workflow_branch_scope_id(branch_id.as_deref(), lane_id.as_deref());
    let body = payload_string_field(payload, &["message", "body"])
        .unwrap_or_default()
        .trim()
        .to_string();
    let message_id = payload_string_field(payload, &["message", "id"]);
    let workflow_resume_run_id = payload_string_field(payload, &["workflow_resume_run_id"]);
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

    let agents = load_agent_configs(&runtime.runtime_paths)?;
    let model_endpoints = load_model_endpoint_configs(&runtime.runtime_paths)?;
    let owner = payload_owner(payload).unwrap_or_else(|| OwnerRef::global("runtime"));

    for agent_id in &addressed_agents {
        if workflow_resume_run_id.is_none()
            && already_processed_conversation_message(
                store,
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                client.processing_stale_after_ms,
            )
            .await?
        {
            continue;
        }
        if workflow_resume_run_id.is_none() {
            mark_conversation_message_receipt_status(
                store,
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                "running",
            )
            .await?;
        }
        let agent_result: Result<(), String> = async {
            let conversation_messages = client
                .dispatch_action(
                    "message.list",
                    serde_json::json!({ "conversation_id": conversation_id }),
                    permission_actor_context(
                        agent_id,
                        "workflow.conversation_to_run",
                        Some(&conversation_id),
                        None,
                        message_id.as_deref(),
                    ),
                )
                .await
                .map_err(|error| error.to_string())?;
            let memory_context = assemble_memory_context(
                client,
                &owner,
                &conversation_id,
                visible_recent_messages(&conversation_messages, agent_id),
                permission_actor_context(
                    agent_id,
                    "workflow.memory_context",
                    Some(&conversation_id),
                    None,
                    message_id.as_deref(),
                ),
            )
            .await;
            let metadata = serde_json::json!({
                "origin": "workflow.conversation_message.created",
                "message_id": message_id,
            });

            if let Some(run_id) = workflow_resume_run_id.as_deref() {
                if let Some(run_response) =
                    load_run_response_for_agent(store, &conversation_id, agent_id, run_id).await?
                {
                    execute_workflow_run(
                        client,
                        runtime,
                        store,
                        &agents,
                        &model_endpoints,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        &conversation_messages,
                        &run_response,
                        agent_id,
                        true,
                    )
                    .await?;
                } else if run_id.starts_with("direct-") {
                    execute_direct_reply(
                        client,
                        &runtime.runtime_paths,
                        &agents,
                        &model_endpoints,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        &conversation_messages,
                        agent_id,
                        run_id,
                    )
                    .await?;
                } else {
                    return Err(format!("workflow run '{run_id}' not found for agent"));
                }
                return Ok(());
            }

            let mut active_session =
                load_active_workflow_session(client, &conversation_id, agent_id, &branch_scope)
                    .await
                    .map_err(|error| error.to_string())?;

            if active_session.is_some() && is_explicit_new_topic(&body) {
                abandon_active_workflow_session(
                    client,
                    store,
                    &conversation_id,
                    agent_id,
                    active_session.as_ref(),
                )
                .await
                .map_err(|error| error.to_string())?;
                active_session = None;
            }

            if is_workflow_execution_confirmation(&body) {
                if let Some(run_response) = resume_pending_workflow_run(
                    client,
                    runtime,
                    store,
                    &owner,
                    &agents,
                    &model_endpoints,
                    &conversation_id,
                    lane_id.as_deref(),
                    message_id.as_deref(),
                    &conversation_messages,
                    agent_id,
                    memory_context.clone(),
                    metadata.clone(),
                    active_session.as_ref(),
                )
                .await?
                {
                    execute_workflow_run(
                        client,
                        runtime,
                        store,
                        &agents,
                        &model_endpoints,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        &conversation_messages,
                        &run_response,
                        agent_id,
                        false,
                    )
                    .await?;
                } else {
                    append_agent_conversation_reply(
                        client,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        agent_id,
                        None,
                        "当前没有可直接继续执行的复杂任务。你可以继续补充要求，或者直接让我处理一个具体操作。",
                    )
                    .await?;
                }
                return Ok(());
            }

            match decide_conversation_route(&body, active_session.as_ref()) {
                ConversationRoute::ManagedDiscussion => {
                    let discussion_reply = upsert_managed_discussion(
                        client,
                        runtime,
                        store,
                        &agents,
                        &model_endpoints,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        &conversation_messages,
                        &body,
                        agent_id,
                        &branch_scope,
                        active_session.as_ref(),
                    )
                    .await
                    .map_err(|error| error.to_string())?;
                    append_agent_conversation_reply(
                        client,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        agent_id,
                        None,
                        &discussion_reply,
                    )
                    .await?;
                }
                ConversationRoute::DirectReply => {
                    let direct_run_id = synthetic_direct_run_id(agent_id, message_id.as_deref());
                    execute_direct_reply(
                        client,
                        &runtime.runtime_paths,
                        &agents,
                        &model_endpoints,
                        &conversation_id,
                        lane_id.as_deref(),
                        message_id.as_deref(),
                        &conversation_messages,
                        agent_id,
                        &direct_run_id,
                    )
                    .await?;
                }
            }

            Ok(())
        }
        .await;
        if workflow_resume_run_id.is_none() {
            mark_conversation_message_receipt_status(
                store,
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                if agent_result.is_ok() {
                    "completed"
                } else {
                    "failed"
                },
            )
            .await?;
        }
        agent_result?;
    }

    Ok(())
}

fn workflow_session_namespace() -> &'static str {
    "workflow.session"
}

fn workflow_branch_scope_id(branch_id: Option<&str>, lane_id: Option<&str>) -> String {
    branch_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .or_else(|| lane_id.map(str::trim).filter(|value| !value.is_empty()))
        .unwrap_or("default")
        .to_string()
}

fn workflow_session_state_key(agent_id: &str, branch_scope: &str) -> String {
    format!("agent:{agent_id}:branch:{branch_scope}:active")
}

async fn load_active_workflow_session(
    client: &HostApiClient,
    conversation_id: &str,
    agent_id: &str,
    branch_scope: &str,
) -> Result<Option<ActiveWorkflowSession>, HostApiError> {
    let Some(value) = client
        .get_extension_state(
            "workflow",
            workflow_session_namespace(),
            "conversation",
            conversation_id,
            &workflow_session_state_key(agent_id, branch_scope),
        )
        .await?
    else {
        return Ok(None);
    };
    serde_json::from_value(value)
        .map(Some)
        .map_err(|error| internal_error(format!("parse active workflow session failed: {error}")))
}

async fn save_active_workflow_session(
    client: &HostApiClient,
    conversation_id: &str,
    agent_id: &str,
    branch_scope: &str,
    session: &ActiveWorkflowSession,
) -> Result<(), HostApiError> {
    client
        .put_extension_state(
            "workflow",
            workflow_session_namespace(),
            "conversation",
            conversation_id,
            &workflow_session_state_key(agent_id, branch_scope),
            serde_json::to_value(session).map_err(|error| {
                internal_error(format!("serialize active workflow session failed: {error}"))
            })?,
        )
        .await
        .map(|_| ())
}

async fn clear_active_workflow_session(
    client: &HostApiClient,
    conversation_id: &str,
    agent_id: &str,
    branch_scope: &str,
) -> Result<(), HostApiError> {
    client
        .delete_extension_state(
            "workflow",
            workflow_session_namespace(),
            "conversation",
            conversation_id,
            &workflow_session_state_key(agent_id, branch_scope),
        )
        .await
}

async fn abandon_active_workflow_session(
    client: &HostApiClient,
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    agent_id: &str,
    session: Option<&ActiveWorkflowSession>,
) -> Result<(), HostApiError> {
    let Some(session) = session else {
        return Ok(());
    };
    let _ = update_workflow_draft_status(store, &session.draft_id, "abandoned").await;
    let _ = client
        .update_extension_record(serde_json::json!({
            "id": session.record_id,
            "status": "abandoned",
            "summary": "这条复杂任务主线已结束，后续消息会按新话题处理。",
        }))
        .await;
    let _ = client.close_extension_record(&session.record_id).await;
    clear_active_workflow_session(client, conversation_id, agent_id, &session.branch_scope).await
}

fn is_explicit_new_topic(body: &str) -> bool {
    let normalized = body.trim().to_ascii_lowercase();
    ["另外", "另一个", "新问题", "换个", "换一下", "重新开一个"]
        .iter()
        .any(|needle| normalized.contains(&needle.to_ascii_lowercase()))
}

fn is_explicit_plan_request(body: &str) -> bool {
    let normalized = body.trim().to_ascii_lowercase();
    [
        "先计划",
        "先出方案",
        "任务编排",
        "按步骤",
        "先别执行",
        "先讨论方案",
    ]
    .iter()
    .any(|needle| normalized.contains(&needle.to_ascii_lowercase()))
}

fn looks_complex_request(body: &str) -> bool {
    let normalized = body.trim().to_ascii_lowercase();
    let mut score = 0;
    for needle in [
        "重构",
        "架构",
        "方案",
        "系统级",
        "一次性",
        "长期",
        "前后端",
        "多文件",
        "多模块",
        "权限系统",
        "任务编排",
        "工作流",
        "设计一下",
    ] {
        if normalized.contains(&needle.to_ascii_lowercase()) {
            score += 1;
        }
    }
    if body.lines().count() >= 4 {
        score += 1;
    }
    score >= 2
}

fn decide_conversation_route(
    body: &str,
    active_session: Option<&ActiveWorkflowSession>,
) -> ConversationRoute {
    if is_explicit_plan_request(body) {
        return ConversationRoute::ManagedDiscussion;
    }
    if active_session.is_some() {
        return ConversationRoute::ManagedDiscussion;
    }
    if looks_complex_request(body) {
        return ConversationRoute::ManagedDiscussion;
    }
    ConversationRoute::DirectReply
}

fn synthetic_direct_run_id(agent_id: &str, message_id: Option<&str>) -> String {
    format!(
        "direct-{}-{}",
        agent_id,
        message_id
            .map(str::to_string)
            .unwrap_or_else(|| Uuid::new_v4().to_string())
    )
}

async fn load_workflow_draft(
    store: &SqliteRuntimeStore,
    draft_id: &str,
) -> Result<Option<WorkflowDraftRow>, String> {
    let row = sqlx::query(
        "SELECT id, conversation_id, agent_id, status, goal, summary, source_message_id, record_id, latest_revision, payload_json, created_at, updated_at
         FROM drafts WHERE id = ?1",
    )
    .bind(draft_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|error| error.to_string())?;
    map_workflow_draft_row(row)
}

async fn already_processed_conversation_message(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    message_id: Option<&str>,
    agent_id: &str,
    stale_after_ms: u64,
) -> Result<bool, String> {
    let Some(message_id) = message_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    let row = sqlx::query(
        "SELECT status, updated_at
         FROM conversation_message_receipts
         WHERE conversation_id = ?1 AND message_id = ?2 AND agent_id = ?3
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(message_id)
    .bind(agent_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|error| format!("load conversation message receipt failed: {error}"))?;
    Ok(row.is_some_and(|item| {
        let status = item.get::<String, _>("status");
        if status == "completed" {
            return true;
        }
        if status != "running" {
            return false;
        }
        let updated_at = item.get::<String, _>("updated_at");
        is_recent_receipt_timestamp(&updated_at, stale_after_ms)
    }))
}

async fn mark_conversation_message_receipt_status(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    message_id: Option<&str>,
    agent_id: &str,
    status: &str,
) -> Result<(), String> {
    let Some(message_id) = message_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(());
    };
    let now = Utc::now().to_rfc3339();
    sqlx::query(
        "INSERT INTO conversation_message_receipts
         (conversation_id, message_id, agent_id, status, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?5)
         ON CONFLICT(conversation_id, message_id, agent_id) DO UPDATE SET
           status = excluded.status,
           updated_at = excluded.updated_at",
    )
    .bind(conversation_id)
    .bind(message_id)
    .bind(agent_id)
    .bind(status)
    .bind(&now)
    .execute(store.pool())
    .await
    .map(|_| ())
    .map_err(|error| format!("save conversation message receipt failed: {error}"))
}

async fn save_workflow_draft(
    store: &SqliteRuntimeStore,
    current: Option<&WorkflowDraftRow>,
    conversation_id: &str,
    agent_id: &str,
    goal: &str,
    summary: &str,
    source_message_id: Option<&str>,
    record_id: &str,
    plan: &PlanSpec,
) -> Result<WorkflowDraftRow, String> {
    let now = Utc::now().to_rfc3339();
    let draft_id = current
        .map(|item| item.id.clone())
        .unwrap_or_else(|| format!("draft-{}", Uuid::new_v4()));
    let revision = current.map(|item| item.latest_revision + 1).unwrap_or(1);
    let plan_json = serde_json::to_string(plan).map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO drafts
         (id, conversation_id, agent_id, status, goal, summary, source_message_id, record_id, latest_revision, payload_json, created_at, updated_at)
         VALUES (?1, ?2, ?3, 'ready', ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?10)
         ON CONFLICT(id) DO UPDATE SET
           status = 'ready',
           goal = excluded.goal,
           summary = excluded.summary,
           source_message_id = excluded.source_message_id,
           record_id = excluded.record_id,
           latest_revision = excluded.latest_revision,
           payload_json = excluded.payload_json,
           updated_at = excluded.updated_at",
    )
    .bind(&draft_id)
    .bind(conversation_id)
    .bind(agent_id)
    .bind(goal)
    .bind(summary)
    .bind(source_message_id)
    .bind(record_id)
    .bind(revision)
    .bind(plan_json)
    .bind(&now)
    .execute(store.pool())
    .await
    .map_err(|error| error.to_string())?;

    sqlx::query(
        "INSERT INTO draft_revisions
         (id, draft_id, revision, goal, summary, source_message_id, payload_json, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
    )
    .bind(format!("draftrev-{}", Uuid::new_v4()))
    .bind(&draft_id)
    .bind(revision)
    .bind(goal)
    .bind(summary)
    .bind(source_message_id)
    .bind(serde_json::to_string(plan).map_err(|error| error.to_string())?)
    .bind(&now)
    .execute(store.pool())
    .await
    .map_err(|error| error.to_string())?;

    load_workflow_draft(store, &draft_id)
        .await?
        .ok_or_else(|| "saved workflow draft missing".to_string())
}

async fn update_workflow_draft_status(
    store: &SqliteRuntimeStore,
    draft_id: &str,
    status: &str,
) -> Result<(), String> {
    sqlx::query("UPDATE drafts SET status = ?2, updated_at = ?3 WHERE id = ?1")
        .bind(draft_id)
        .bind(status)
        .bind(Utc::now().to_rfc3339())
        .execute(store.pool())
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

fn map_workflow_draft_row(
    row: Option<sqlx::sqlite::SqliteRow>,
) -> Result<Option<WorkflowDraftRow>, String> {
    let Some(row) = row else {
        return Ok(None);
    };
    let payload_json: String = row.get("payload_json");
    let plan = serde_json::from_str::<PlanSpec>(&payload_json)
        .map_err(|error| format!("parse workflow draft payload failed: {error}"))?;
    Ok(Some(WorkflowDraftRow {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        agent_id: row.get("agent_id"),
        status: row.get("status"),
        goal: row.get("goal"),
        summary: row.get("summary"),
        source_message_id: row.get("source_message_id"),
        record_id: row.get("record_id"),
        latest_revision: row.get("latest_revision"),
        plan,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

async fn upsert_managed_discussion(
    client: &HostApiClient,
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    body: &str,
    agent_id: &str,
    branch_scope: &str,
    active_session: Option<&ActiveWorkflowSession>,
) -> Result<String, HostApiError> {
    let current = if let Some(session) = active_session {
        load_workflow_draft(store, &session.draft_id)
            .await
            .map_err(internal_error)?
    } else {
        None
    };
    let goal = current
        .as_ref()
        .map(|draft| draft.goal.clone())
        .unwrap_or_else(|| body.trim().to_string());
    let plan = generate_draft_plan(
        client,
        runtime,
        agents,
        model_endpoints,
        conversation_id,
        lane_id,
        message_id,
        conversation_messages,
        &goal,
        current.as_ref().map(|draft| &draft.plan),
        agent_id,
    )
    .await?;
    let revision = current
        .as_ref()
        .map(|draft| draft.latest_revision + 1)
        .unwrap_or(1);
    let reply = build_draft_reply(&goal, &plan, revision);

    let record = if let Some(record_id) = current
        .as_ref()
        .and_then(|draft| draft.record_id.as_deref())
    {
        client
            .update_extension_record(serde_json::json!({
                "id": record_id,
                "status": "ready",
                "title": format!("复杂任务方案 v{revision}"),
                "summary": reply.lines().next().unwrap_or("复杂任务方案已更新"),
                "payload": {
                    "agent_id": agent_id,
                    "goal": goal,
                    "revision": revision,
                    "status": "ready",
                    "steps": summarize_plan_steps(&plan),
                    "plan": plan,
                },
                "related_message_id": message_id,
            }))
            .await?
    } else {
        client
            .append_extension_record(serde_json::json!({
                "extension_id": "workflow",
                "namespace": "workflow.discussion",
                "scope_type": "conversation",
                "scope_id": conversation_id,
                "kind": "workflow.discussion",
                "status": "ready",
                "title": format!("复杂任务方案 v{revision}"),
                "summary": reply.lines().next().unwrap_or("复杂任务方案已创建"),
                "related_message_id": message_id,
                "payload": {
                    "agent_id": agent_id,
                    "goal": goal,
                    "revision": revision,
                    "status": "ready",
                    "steps": summarize_plan_steps(&plan),
                    "plan": plan,
                }
            }))
            .await?
    };

    let draft = save_workflow_draft(
        store,
        current.as_ref(),
        conversation_id,
        agent_id,
        &goal,
        reply.lines().next().unwrap_or("复杂任务方案已更新"),
        message_id,
        &record.id,
        &plan,
    )
    .await
    .map_err(internal_error)?;
    save_active_workflow_session(
        client,
        conversation_id,
        agent_id,
        branch_scope,
        &ActiveWorkflowSession {
            draft_id: draft.id,
            record_id: record.id,
            branch_scope: branch_scope.to_string(),
            status: "ready".to_string(),
            revision: draft.latest_revision,
        },
    )
    .await?;
    Ok(reply)
}

async fn generate_draft_plan(
    client: &HostApiClient,
    runtime: &WorkflowRuntime,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    goal: &str,
    previous_plan: Option<&PlanSpec>,
    agent_id: &str,
) -> Result<PlanSpec, HostApiError> {
    let agent = agents
        .iter()
        .find(|item| item.id == agent_id)
        .ok_or_else(|| not_found_error(format!("agent '{agent_id}' not found")))?;
    let model_endpoint = model_endpoints
        .iter()
        .find(|item| item.id == agent.model_endpoint_id && item.enabled)
        .ok_or_else(|| {
            not_found_error(format!(
                "model endpoint '{}' not found",
                agent.model_endpoint_id
            ))
        })?;
    let model_id = if agent.model_id.trim().is_empty() {
        model_endpoint.default_model.trim().to_string()
    } else {
        agent.model_id.trim().to_string()
    };
    if model_id.is_empty() {
        return Err(bad_request_error(format!(
            "agent '{}' has no model configured",
            agent.id
        )));
    }

    let draft_run_id = format!("draft-{}", message_id.unwrap_or("preview"));
    let base_messages =
        normalize_conversation_messages_for_provider(conversation_messages, agent_id);
    let mut planning_prompt = build_planning_prompt(goal, true);
    if previous_plan.is_some() {
        planning_prompt
            .push_str("\n请基于当前已有方案继续修订，不要换掉任务目标，只修正策略、步骤和约束。\n");
    }
    let previous_plan_value =
        previous_plan.map(|plan| serde_json::to_value(plan).unwrap_or(JsonValue::Null));
    let context = build_agent_provider_context(
        client,
        &runtime.runtime_paths,
        agent,
        conversation_id,
        lane_id,
        message_id,
        &draft_run_id,
        previous_plan_value.as_ref(),
    )
    .await;
    let metadata = serde_json::json!({
        "conversation_id": conversation_id,
        "lane_id": lane_id,
        "message_id": message_id,
        "run_id": draft_run_id,
        "agent_id": agent.id,
        "mode": "workflow_discussion",
    });

    let mut last_reason = "planner did not return a valid plan".to_string();
    let mut previous_text = String::new();
    for attempt in 0..2 {
        let prompt = if attempt == 0 {
            planning_prompt.clone()
        } else {
            format!(
                "{}\n\n上轮输出没有通过计划校验，原因：{}。请继续在原目标上修订。",
                planning_prompt, last_reason
            )
        };
        let mut messages = base_messages.clone();
        if !previous_text.trim().is_empty() {
            messages.push(serde_json::json!({
                "role": "assistant",
                "content": previous_text,
            }));
        }
        messages.push(serde_json::json!({
            "role": "user",
            "content": prompt,
        }));
        let response = client
            .provider_generate(
                &model_endpoint.kind,
                serde_json::json!({
                    "model_endpoint": model_endpoint_runtime_request_config(model_endpoint),
                    "model": model_id,
                    "instructions": ProviderInstructions {
                        base: build_agent_runtime_prompt(agent, &draft_run_id),
                    },
                    "system_prompt": build_agent_runtime_prompt(agent, &draft_run_id),
                    "context": context,
                    "messages": messages,
                    "generation_options": agent.generation_options,
                    "tools": [],
                    "metadata": metadata,
                }),
                permission_actor_context(
                    agent_id,
                    "workflow.provider_generate",
                    Some(conversation_id),
                    Some(&draft_run_id),
                    message_id,
                ),
            )
            .await?;
        if let Some(reasoning) = read_provider_reasoning(&response) {
            append_reasoning_message(
                client,
                conversation_id,
                lane_id,
                message_id,
                agent_id,
                Some(&draft_run_id),
                &reasoning,
            )
            .await?;
        }
        let text = response
            .get("text")
            .and_then(JsonValue::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .ok_or_else(|| internal_error("planner returned empty text"))?
            .to_string();
        previous_text = text.clone();
        match parse_plan_from_text(&text, goal) {
            Ok(plan) => {
                let verdict = validate_plan(&plan);
                if verdict.ready {
                    return Ok(plan);
                }
                last_reason = verdict.reason;
            }
            Err(reason) => last_reason = reason,
        }
    }
    Err(bad_request_error(format!(
        "计划输出未通过校验：{last_reason}"
    )))
}

fn build_draft_reply(goal: &str, plan: &PlanSpec, revision: i64) -> String {
    let mut lines = vec![format!(
        "我先把这件事整理成第 {revision} 版方案，目标还是：{goal}"
    )];
    for item in summarize_plan_steps(plan).into_iter().take(5) {
        lines.push(format!("- {item}"));
    }
    lines.push("如果方向不对，你继续指出要改哪一点，我会在这版上继续修。".to_string());
    lines.push("如果方向已经对了，直接说“开始执行”或“去改吧”，我就按这版进入执行。".to_string());
    lines.join("\n")
}

async fn resume_pending_workflow_run(
    client: &HostApiClient,
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    owner: &OwnerRef,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    agent_id: &str,
    memory_context: Option<RunContext>,
    metadata: JsonValue,
    active_session: Option<&ActiveWorkflowSession>,
) -> Result<Option<JsonValue>, String> {
    if let Some(session) = active_session {
        let Some(draft) = load_workflow_draft(store, &session.draft_id).await? else {
            return Ok(None);
        };
        let execution_record = client
            .append_extension_record(serde_json::json!({
                "extension_id": "workflow",
                "namespace": "workflow.execution",
                "scope_type": "conversation",
                "scope_id": conversation_id,
                "kind": "workflow.execution",
                "status": "running",
                "title": "复杂任务执行中",
                "summary": draft.summary,
                "related_message_id": message_id,
                "payload": {
                    "agent_id": agent_id,
                    "goal": draft.goal,
                    "draft_id": draft.id,
                    "stage": "pending",
                    "steps": summarize_plan_steps(&draft.plan),
                }
            }))
            .await
            .map_err(|error| error.to_string())?;
        let run_metadata = merge_json_objects(
            metadata,
            serde_json::json!({
                "conversation_record_id": execution_record.id,
                "draft_id": draft.id,
                "route": "managed_run",
            }),
        );
        let run_response = create_workflow_run_response(
            runtime,
            store,
            owner,
            conversation_id,
            lane_id,
            message_id,
            &draft.goal,
            agent_id,
            memory_context,
            run_metadata,
        )
        .await?;
        let run = store
            .get_run(run_response_id(&run_response).unwrap_or_default())
            .await
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "workflow run missing after create".to_string())?;
        runtime
            .runtime_store
            .save_plan(&run, &draft.plan, agent_id)
            .await
            .map_err(|error| error.to_string())?;
        let run_response = load_run_detail_json(store, &run.id)
            .await?
            .ok_or_else(|| "workflow run detail missing after save_plan".to_string())?;
        remember_workflow_run(
            client,
            owner,
            conversation_id,
            lane_id,
            &draft.goal,
            agent_id,
            message_id,
            run_response_id(&run_response),
            &run_response,
        )
        .await;
        let _ = client
            .update_extension_record(serde_json::json!({
                "id": session.record_id,
                "status": "closed",
                "summary": "方案讨论已结束，已进入执行。",
            }))
            .await;
        let _ = client.close_extension_record(&session.record_id).await;
        let _ = update_workflow_draft_status(store, &draft.id, "running").await;
        clear_active_workflow_session(client, conversation_id, agent_id, &session.branch_scope)
            .await
            .map_err(|error| error.to_string())?;
        let _ = (agents, model_endpoints, conversation_messages);
        return Ok(Some(run_response));
    }

    load_latest_pending_run_for_agent(store, conversation_id, message_id, lane_id, agent_id).await
}

async fn execute_workflow_run(
    client: &HostApiClient,
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    run_response: &JsonValue,
    agent_id: &str,
    resumed_from_approval: bool,
) -> Result<(), String> {
    let run_response = match prepare_workflow_run_for_execution(
        store,
        run_response,
        resumed_from_approval,
    )
    .await
    {
        Ok(detail) => detail,
        Err(reason) => {
            append_agent_conversation_reply(
                client,
                conversation_id,
                lane_id,
                message_id,
                agent_id,
                run_response_id(run_response),
                &reason,
            )
            .await?;
            return Ok(());
        }
    };
    sync_execution_record(
        client,
        &run_response,
        "running",
        None,
        Some(run_stage_label(&run_response)),
    )
    .await;
    let reply_body = match generate_real_agent_reply(
        client,
        store,
        &runtime.runtime_paths,
        agents,
        model_endpoints,
        conversation_id,
        lane_id,
        message_id,
        conversation_messages,
        &run_response,
        agent_id,
    )
    .await
    {
        Ok(reply) => {
            let _ = transition_workflow_run_stage(
                store,
                run_response_id(&run_response).unwrap_or_default(),
                RunStage::Completed,
                "本轮按计划执行完成",
                "WF_EXECUTION_COMPLETED",
                NextAction::Complete,
            )
            .await;
            sync_execution_record(
                client,
                &run_response,
                "completed",
                Some("执行完成".to_string()),
                Some("completed".to_string()),
            )
            .await;
            reply
        }
        Err(error) if error.is_permission_approval() => {
            let _ = transition_workflow_run_stage(
                store,
                run_response_id(&run_response).unwrap_or_default(),
                RunStage::Blocked,
                "执行过程中等待权限审批",
                "WF_EXECUTION_WAITING_APPROVAL",
                NextAction::EnterBlocked,
            )
            .await;
            sync_execution_record(
                client,
                &run_response,
                "blocked",
                Some("执行过程中等待权限审批".to_string()),
                Some("blocked".to_string()),
            )
            .await;
            return Ok(());
        }
        Err(error) => {
            let _ = transition_workflow_run_stage(
                store,
                run_response_id(&run_response).unwrap_or_default(),
                RunStage::Failed,
                "执行过程中发生错误",
                "WF_EXECUTION_FAILED",
                NextAction::Fail,
            )
            .await;
            sync_execution_record(
                client,
                &run_response,
                "failed",
                Some(format_host_api_error_for_conversation(&error)),
                Some("failed".to_string()),
            )
            .await;
            format_host_api_error_for_conversation(&error)
        }
    };
    append_agent_conversation_reply(
        client,
        conversation_id,
        lane_id,
        message_id,
        agent_id,
        run_response_id(&run_response),
        &reply_body,
    )
    .await
}

async fn sync_execution_record(
    client: &HostApiClient,
    run_response: &JsonValue,
    status: &str,
    summary: Option<String>,
    stage: Option<String>,
) {
    let Some(record_id) = run_response_record_id(run_response) else {
        return;
    };
    let _ = client
        .update_extension_record(serde_json::json!({
            "id": record_id,
            "status": status,
            "summary": summary,
            "payload": {
                "run_id": run_response_id(run_response),
                "goal": run_response_goal(run_response),
                "stage": stage.as_deref().unwrap_or("unknown"),
                "steps": run_response
                    .get("tasks")
                    .and_then(JsonValue::as_array)
                    .cloned()
                    .unwrap_or_default(),
                "artifacts": run_response
                    .get("artifacts")
                    .and_then(JsonValue::as_array)
                    .cloned()
                    .unwrap_or_default(),
            }
        }))
        .await;
    if matches!(status, "completed" | "failed") {
        let _ = client.close_extension_record(record_id).await;
    }
}

fn run_response_record_id(run_response: &JsonValue) -> Option<&str> {
    run_response
        .get("run")
        .and_then(|item| item.get("metadata"))
        .and_then(|item| item.get("conversation_record_id"))
        .and_then(JsonValue::as_str)
}

fn run_response_goal(run_response: &JsonValue) -> String {
    run_response
        .get("run")
        .and_then(|item| item.get("goal"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default()
        .to_string()
}

fn run_stage_label(run_response: &JsonValue) -> String {
    run_response
        .get("run")
        .and_then(|item| item.get("stage"))
        .and_then(JsonValue::as_str)
        .unwrap_or("unknown")
        .to_string()
}

fn merge_json_objects(left: JsonValue, right: JsonValue) -> JsonValue {
    let mut base = left.as_object().cloned().unwrap_or_default();
    for (key, value) in right.as_object().cloned().unwrap_or_default() {
        base.insert(key, value);
    }
    JsonValue::Object(base)
}

async fn execute_direct_reply(
    client: &HostApiClient,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    agent_id: &str,
    run_id: &str,
) -> Result<(), String> {
    match generate_agent_reply(
        client,
        runtime_paths,
        agents,
        model_endpoints,
        conversation_id,
        lane_id,
        message_id,
        conversation_messages,
        None,
        run_id,
        agent_id,
    )
    .await
    {
        Ok(reply_body) => {
            append_agent_conversation_reply(
                client,
                conversation_id,
                lane_id,
                message_id,
                agent_id,
                None,
                &reply_body,
            )
            .await
        }
        Err(error) if error.is_permission_approval() => Ok(()),
        Err(error) => {
            append_agent_conversation_reply(
                client,
                conversation_id,
                lane_id,
                message_id,
                agent_id,
                None,
                &format_host_api_error_for_conversation(&error),
            )
            .await
        }
    }
}

async fn generate_real_agent_reply(
    client: &HostApiClient,
    _store: &SqliteRuntimeStore,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    run_response: &JsonValue,
    agent_id: &str,
) -> Result<String, HostApiError> {
    let run_id = run_response_id(run_response)
        .unwrap_or_default()
        .to_string();
    generate_agent_reply(
        client,
        runtime_paths,
        agents,
        model_endpoints,
        conversation_id,
        lane_id,
        message_id,
        conversation_messages,
        run_response.get("plan"),
        &run_id,
        agent_id,
    )
    .await
}

async fn generate_agent_reply(
    client: &HostApiClient,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    plan: Option<&JsonValue>,
    run_id: &str,
    agent_id: &str,
) -> Result<String, HostApiError> {
    let agent = agents
        .iter()
        .find(|item| item.id == agent_id)
        .ok_or_else(|| not_found_error(format!("agent '{agent_id}' not found")))?;
    let model_endpoint = model_endpoints
        .iter()
        .find(|item| item.id == agent.model_endpoint_id && item.enabled)
        .ok_or_else(|| {
            not_found_error(format!(
                "model endpoint '{}' not found",
                agent.model_endpoint_id
            ))
        })?;
    let model_id = if agent.model_id.trim().is_empty() {
        model_endpoint.default_model.trim().to_string()
    } else {
        agent.model_id.trim().to_string()
    };
    if model_id.is_empty() {
        return Err(bad_request_error(format!(
            "agent '{}' has no model configured",
            agent.id
        )));
    }

    let mut messages =
        normalize_conversation_messages_for_provider(conversation_messages, agent_id);
    let tools = build_agent_builtin_tool_specs(agent);
    let context = build_agent_provider_context(
        client,
        runtime_paths,
        agent,
        conversation_id,
        lane_id,
        message_id,
        run_id,
        plan,
    )
    .await;
    let instructions = ProviderInstructions {
        base: build_agent_runtime_prompt(agent, run_id),
    };
    let metadata = serde_json::json!({
        "conversation_id": conversation_id,
        "lane_id": lane_id,
        "message_id": message_id,
        "run_id": run_id,
        "sandbox_enabled": agent.execution_environment.sandbox_enabled,
        "agent_id": agent.id,
        "agent_display_name": agent.display_name,
    });

    let mut last_process_text: Option<String> = None;
    for _ in 0..6 {
        let response = client
            .provider_generate(
                &model_endpoint.kind,
                serde_json::json!({
                    "model_endpoint": model_endpoint_runtime_request_config(model_endpoint),
                    "model": model_id,
                    "instructions": instructions,
                    "system_prompt": build_agent_runtime_prompt(agent, run_id),
                    "context": context,
                    "messages": messages,
                    "generation_options": agent.generation_options,
                    "tools": tools,
                    "tool_choice": "auto",
                    "metadata": metadata,
                }),
                permission_actor_context(
                    agent_id,
                    "workflow.provider_generate",
                    Some(conversation_id),
                    Some(run_id),
                    message_id,
                ),
            )
            .await?;
        let text = response
            .get("text")
            .and_then(JsonValue::as_str)
            .map(str::trim)
            .filter(|item| !item.is_empty())
            .map(ToOwned::to_owned);
        let reasoning = read_provider_reasoning(&response);
        if let Some(reasoning) = reasoning.as_deref() {
            let _ = append_reasoning_message(
                client,
                conversation_id,
                lane_id,
                message_id,
                agent_id,
                Some(run_id),
                reasoning,
            )
            .await;
        }
        let tool_calls = response
            .get("tool_calls")
            .cloned()
            .map(serde_json::from_value::<Vec<AgentToolCall>>)
            .transpose()
            .map_err(|error| internal_error(format!("parse agent tool calls failed: {error}")))?
            .unwrap_or_default();
        if tool_calls.is_empty() {
            return text.ok_or_else(|| internal_error("provider returned empty text"));
        }
        if let Some(progress_text) = text.as_deref() {
            let normalized_progress = progress_text.trim();
            if !normalized_progress.is_empty()
                && last_process_text.as_deref() != Some(normalized_progress)
            {
                let _ = append_agent_process_message(
                    client,
                    conversation_id,
                    lane_id,
                    message_id,
                    agent_id,
                    Some(run_id),
                    normalized_progress,
                )
                .await;
                last_process_text = Some(normalized_progress.to_string());
            }
        }
        messages.push(serde_json::json!({
            "role": "assistant",
            "content": text.unwrap_or_default(),
            "tool_calls": tool_calls.iter().map(|call| serde_json::json!({
                "id": call.id,
                "name": call.name,
                "arguments": call.arguments,
            })).collect::<Vec<_>>(),
        }));
        for tool_call in tool_calls {
            match execute_builtin_tool(
                client,
                agent_id,
                conversation_id,
                lane_id,
                message_id,
                run_id,
                &tool_call,
            )
            .await
            {
                Ok(result) => {
                    let body = serialize_tool_message_envelope(&tool_call, Ok(result.clone()))
                        .map_err(|error| {
                            internal_error(format!("serialize tool message failed: {error}"))
                        })?;
                    let _ = append_tool_result_message(
                        client,
                        conversation_id,
                        lane_id,
                        message_id,
                        agent_id,
                        &body,
                    )
                    .await;
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": body,
                    }));
                }
                Err(error) => {
                    let body = serialize_tool_message_envelope(&tool_call, Err(&error)).map_err(
                        |serialize_error| {
                            internal_error(format!(
                                "serialize tool message failed: {serialize_error}"
                            ))
                        },
                    )?;
                    let _ = append_tool_result_message(
                        client,
                        conversation_id,
                        lane_id,
                        message_id,
                        agent_id,
                        &body,
                    )
                    .await;
                    if error.is_permission_approval() {
                        return Err(error);
                    }
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": body,
                    }));
                }
            }
        }
    }
    Err(internal_error(
        "agent tool loop exceeded maximum iterations",
    ))
}

async fn execute_builtin_tool(
    client: &HostApiClient,
    agent_id: &str,
    conversation_id: &str,
    _lane_id: Option<&str>,
    message_id: Option<&str>,
    run_id: &str,
    tool_call: &AgentToolCall,
) -> Result<JsonValue, HostApiError> {
    match tool_call.name.as_str() {
        "fs_read" => {
            client
                .execute_operation(
                    "fs.read",
                    agent_id,
                    conversation_id,
                    run_id,
                    message_id,
                    tool_call.arguments.clone(),
                )
                .await
        }
        "fs_write" => {
            client
                .execute_operation(
                    "fs.write",
                    agent_id,
                    conversation_id,
                    run_id,
                    message_id,
                    tool_call.arguments.clone(),
                )
                .await
        }
        "command_exec" => {
            client
                .execute_operation(
                    "command.exec",
                    agent_id,
                    conversation_id,
                    run_id,
                    message_id,
                    tool_call.arguments.clone(),
                )
                .await
        }
        "net_fetch" => {
            client
                .execute_operation(
                    "net.fetch",
                    agent_id,
                    conversation_id,
                    run_id,
                    message_id,
                    tool_call.arguments.clone(),
                )
                .await
        }
        other => Err(bad_request_error(format!(
            "unsupported agent tool '{other}'"
        ))),
    }
}

async fn append_agent_conversation_reply(
    client: &HostApiClient,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    agent_id: &str,
    run_id: Option<&str>,
    body: &str,
) -> Result<(), String> {
    client
        .dispatch_action(
            "message.append",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "agent",
                    "addressed_agents": ["operator"],
                }
            }),
            permission_actor_context(
                agent_id,
                "workflow.run_to_conversation",
                Some(conversation_id),
                run_id,
                message_id,
            ),
        )
        .await
        .map(|_| ())
        .map_err(|error| error.to_string())
}

async fn append_tool_result_message(
    client: &HostApiClient,
    conversation_id: &str,
    lane_id: Option<&str>,
    parent_message_id: Option<&str>,
    agent_id: &str,
    body: &str,
) -> Result<(), HostApiError> {
    client
        .dispatch_action(
            "message.append",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "tool",
                    "parent_message_id": parent_message_id,
                    "addressed_agents": [agent_id],
                }
            }),
            permission_actor_context(
                agent_id,
                "workflow.tool_result",
                Some(conversation_id),
                None,
                parent_message_id,
            ),
        )
        .await?;
    Ok(())
}

async fn append_agent_process_message(
    client: &HostApiClient,
    conversation_id: &str,
    lane_id: Option<&str>,
    parent_message_id: Option<&str>,
    agent_id: &str,
    run_id: Option<&str>,
    body: &str,
) -> Result<(), HostApiError> {
    client
        .dispatch_action(
            "message.append",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "agent",
                    "parent_message_id": parent_message_id,
                    "addressed_agents": ["operator"],
                }
            }),
            permission_actor_context(
                agent_id,
                "workflow.process_message",
                Some(conversation_id),
                run_id,
                parent_message_id,
            ),
        )
        .await
        .map(|_| ())
}

async fn append_reasoning_message(
    client: &HostApiClient,
    conversation_id: &str,
    lane_id: Option<&str>,
    parent_message_id: Option<&str>,
    agent_id: &str,
    run_id: Option<&str>,
    reasoning: &str,
) -> Result<(), HostApiError> {
    let body = serialize_reasoning_message_envelope(reasoning)
        .map_err(|error| internal_error(format!("serialize reasoning message failed: {error}")))?;
    client
        .dispatch_action(
            "message.append",
            serde_json::json!({
                "conversation_id": conversation_id,
                "message": {
                    "body": body,
                    "lane_id": lane_id,
                    "sender": agent_id,
                    "role": "system",
                    "parent_message_id": parent_message_id,
                    "addressed_agents": [agent_id],
                }
            }),
            permission_actor_context(
                agent_id,
                "workflow.reasoning_result",
                Some(conversation_id),
                run_id,
                parent_message_id,
            ),
        )
        .await
        .map(|_| ())
}

async fn assemble_memory_context(
    client: &HostApiClient,
    owner: &OwnerRef,
    conversation_id: &str,
    recent_messages: Vec<String>,
    context: JsonValue,
) -> Option<RunContext> {
    let owner_kind = owner_kind_str(&owner.kind);
    client
        .dispatch_action(
            "memory.build_context",
            serde_json::json!({
                "owner_kind": owner_kind,
                "owner_id": owner.id,
                "conversation_id": conversation_id,
                "recent_messages": recent_messages,
                "active_tasks": [],
            }),
            context,
        )
        .await
        .ok()
        .and_then(|value| serde_json::from_value::<RunContext>(value).ok())
}

async fn remember_workflow_run(
    client: &HostApiClient,
    owner: &OwnerRef,
    conversation_id: &str,
    lane_id: Option<&str>,
    goal: &str,
    agent_id: &str,
    message_id: Option<&str>,
    run_id: Option<&str>,
    run_response: &JsonValue,
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
        .and_then(|item| item.get("reason"))
        .or_else(|| {
            run_response
                .get("decision")
                .and_then(|item| item.get("summary"))
        })
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let payload = serde_json::json!({
        "owner_kind": owner_kind_str(&owner.kind),
        "owner_id": owner.id,
        "namespace": format!("workflow/conversation/{conversation_id}"),
        "memory_kind": "observation",
        "stability": "working",
        "title": format!("Workflow run {run_id}"),
        "content": format!(
            "workflow run={run_id}\nconversation_id={conversation_id}\nlane_id={}\nagent_id={agent_id}\nstage={stage}\ngoal={goal}\ndecision={decision}\ntasks={}\nartifacts={}",
            lane_id.unwrap_or(""),
            tasks.len(),
            artifacts.len(),
        ),
        "summary": format!(
            "Workflow run {run_id} 已记录，stage={stage}，tasks={}，artifacts={}",
            tasks.len(),
            artifacts.len()
        ),
        "confidence": 0.55,
        "importance": 0.5,
        "sources": build_workflow_memory_sources(conversation_id, message_id, run_id, &artifacts),
        "tags": ["workflow", "run", stage, agent_id],
        "entities": [run_id, conversation_id, agent_id],
    });
    let _ = client
        .dispatch_action(
            "memory.ingest",
            payload,
            permission_actor_context(
                agent_id,
                "workflow.memory_ingest",
                Some(conversation_id),
                Some(run_id),
                message_id,
            ),
        )
        .await;
}

async fn load_latest_pending_run_for_agent(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    message_id: Option<&str>,
    lane_id: Option<&str>,
    agent_id: &str,
) -> Result<Option<JsonValue>, String> {
    for run in list_recent_message_runs(store, conversation_id, 12).await? {
        if run
            .source_message_id
            .as_deref()
            .is_some_and(|value| Some(value) == message_id)
        {
            continue;
        }
        let stage = run.stage;
        if !run_stage_can_be_resumed(stage) {
            continue;
        }
        if !workflow_run_matches_lane(run.lane_id.as_deref(), lane_id) {
            continue;
        }
        if let Some(detail) =
            load_run_response_for_agent(store, conversation_id, agent_id, &run.id).await?
        {
            return Ok(Some(detail));
        }
    }
    Ok(None)
}

fn workflow_run_matches_lane(run_lane_id: Option<&str>, current_lane_id: Option<&str>) -> bool {
    match current_lane_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(current_lane_id) => {
            run_lane_id.map(str::trim).filter(|value| !value.is_empty()) == Some(current_lane_id)
        }
        None => run_lane_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .is_none(),
    }
}

async fn load_run_response_for_agent(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    agent_id: &str,
    run_id: &str,
) -> Result<Option<JsonValue>, String> {
    let detail = load_run_detail_json(store, run_id).await?.ok_or_else(|| {
        format!("workflow run '{run_id}' not found for conversation '{conversation_id}'")
    })?;
    Ok(run_response_has_assigned_agent(&detail, agent_id).then_some(detail))
}

async fn create_workflow_run_response(
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    owner: &OwnerRef,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    goal: &str,
    agent_id: &str,
    context: Option<RunContext>,
    metadata: JsonValue,
) -> Result<JsonValue, String> {
    let response = run_behavior(
        runtime,
        BehaviorRunRequest {
            owner: owner.clone(),
            goal: goal.to_string(),
            trigger: BehaviorTrigger::Message,
            participants: vec![agent_id.to_string()],
            addressed_agents: vec![agent_id.to_string()],
            context: context.unwrap_or_default(),
            source_refs: vec![BehaviorSourceRef {
                kind: "conversation".to_string(),
                id: conversation_id.to_string(),
                conversation_id: Some(conversation_id.to_string()),
                lane_id: lane_id.map(str::to_string),
                message_id: message_id.map(str::to_string),
                run_id: None,
                artifact_id: None,
            }],
            metadata,
        },
    )
    .await?;
    if let Some(detail) = load_run_detail_json(store, &response.run.id).await? {
        Ok(detail)
    } else {
        serde_json::to_value(response)
            .map_err(|error| format!("serialize workflow run response failed: {error}"))
    }
}

async fn list_recent_message_runs(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    limit: i64,
) -> Result<Vec<ennoia_kernel::RunSpec>, String> {
    let rows = sqlx::query("SELECT payload_json FROM runs ORDER BY updated_at DESC LIMIT ?1")
        .bind(limit.clamp(1, 200))
        .fetch_all(store.pool())
        .await
        .map_err(|error| format!("list workflow runs failed: {error}"))?;
    let mut runs = Vec::with_capacity(rows.len());
    for row in rows {
        let payload_json: String = row.get("payload_json");
        let run = serde_json::from_str::<ennoia_kernel::RunSpec>(&payload_json)
            .map_err(|error| format!("parse workflow run failed: {error}"))?;
        if run.conversation_id == conversation_id && run.trigger == "message" {
            runs.push(run);
        }
    }
    Ok(runs)
}

async fn prepare_workflow_run_for_execution(
    store: &SqliteRuntimeStore,
    run_response: &JsonValue,
    resumed_from_approval: bool,
) -> Result<JsonValue, String> {
    let run_id = run_response_id(run_response)
        .ok_or_else(|| "workflow run 缺少 run.id，无法进入执行阶段。".to_string())?;
    let plan = parse_valid_plan_from_run_response(run_response)?;
    let mut detail = load_run_detail_json(store, run_id)
        .await?
        .ok_or_else(|| format!("workflow run '{run_id}' not found"))?;
    let current_stage = detail
        .get("run")
        .and_then(|item| item.get("stage"))
        .and_then(JsonValue::as_str)
        .map(RunStage::from_str)
        .unwrap_or(RunStage::Pending);

    if current_stage == RunStage::Planning {
        detail = transition_workflow_run_stage(
            store,
            run_id,
            RunStage::Dispatched,
            "用户已确认执行该计划",
            "WF_PLAN_CONFIRMED",
            NextAction::Dispatch,
        )
        .await?;
    } else if current_stage == RunStage::Blocked && resumed_from_approval {
        detail = transition_workflow_run_stage(
            store,
            run_id,
            RunStage::Dispatched,
            "权限审批已通过，准备恢复执行",
            "WF_APPROVAL_RESUMED",
            NextAction::Dispatch,
        )
        .await?;
    }

    let next_stage = detail
        .get("run")
        .and_then(|item| item.get("stage"))
        .and_then(JsonValue::as_str)
        .map(RunStage::from_str)
        .unwrap_or(RunStage::Pending);
    if next_stage != RunStage::Running {
        detail = transition_workflow_run_stage(
            store,
            run_id,
            RunStage::Running,
            if resumed_from_approval {
                "按已确认的计划恢复执行"
            } else {
                "按已确认的计划开始执行"
            },
            "WF_EXECUTION_STARTED",
            NextAction::StayRunning,
        )
        .await?;
    }

    if detail.get("plan").is_none() || detail.get("plan") == Some(&JsonValue::Null) {
        let mut detail_object = detail.as_object().cloned().unwrap_or_default();
        detail_object.insert(
            "plan".to_string(),
            serde_json::to_value(plan)
                .map_err(|error| format!("serialize workflow plan failed: {error}"))?,
        );
        detail = JsonValue::Object(detail_object);
    }

    Ok(detail)
}

fn parse_valid_plan_from_run_response(run_response: &JsonValue) -> Result<PlanSpec, String> {
    let plan_value = run_response
        .get("plan")
        .filter(|value| !value.is_null())
        .cloned()
        .ok_or_else(|| "当前还没有可执行计划，请先重新发起任务，让我先生成计划。".to_string())?;
    let plan = serde_json::from_value::<PlanSpec>(plan_value)
        .map_err(|error| format!("当前计划结构无效，暂时不能执行：{error}"))?;
    let verdict = validate_plan(&plan);
    if verdict.ready {
        Ok(plan)
    } else {
        Err(format!("当前计划还不能执行，原因：{}", verdict.reason))
    }
}

async fn transition_workflow_run_stage(
    store: &SqliteRuntimeStore,
    run_id: &str,
    to_stage: RunStage,
    reason: &str,
    policy_rule_id: &str,
    next_action: NextAction,
) -> Result<JsonValue, String> {
    if run_id.trim().is_empty() {
        return Err("workflow run 缺少 run.id，无法更新阶段。".to_string());
    }
    let Some(mut run) = store
        .get_run(run_id)
        .await
        .map_err(|error| format!("load workflow run failed: {error}"))?
    else {
        return Err(format!("workflow run '{run_id}' not found"));
    };
    if run.stage == to_stage {
        return load_run_detail_json(store, run_id)
            .await?
            .ok_or_else(|| format!("workflow run '{run_id}' not found"));
    }

    let from_stage = run.stage;
    let now = Utc::now().to_rfc3339();
    run.stage = to_stage;
    run.updated_at = now.clone();
    store
        .save_run(&run)
        .await
        .map_err(|error| format!("save workflow run failed: {error}"))?;

    let stage_event = RunStageEvent {
        id: format!("rse-{}", Uuid::new_v4()),
        run_id: run.id.clone(),
        from_stage: Some(from_stage),
        to_stage,
        policy_rule_id: Some(policy_rule_id.to_string()),
        reason: Some(reason.to_string()),
        at: now.clone(),
    };
    store
        .log_stage_event(&stage_event)
        .await
        .map_err(|error| format!("log workflow stage event failed: {error}"))?;

    let decision = DecisionSnapshot {
        id: format!("dec-{}", Uuid::new_v4()),
        run_id: Some(run.id.clone()),
        task_id: None,
        stage: from_stage.as_str().to_string(),
        signals_json: "{}".to_string(),
        next_action: next_action.as_str().to_string(),
        policy_rule_id: policy_rule_id.to_string(),
        at: now,
    };
    store
        .log_decision(&decision)
        .await
        .map_err(|error| format!("log workflow decision failed: {error}"))?;

    load_run_detail_json(store, run_id)
        .await?
        .ok_or_else(|| format!("workflow run '{run_id}' not found"))
}

async fn load_run_detail_json(
    store: &SqliteRuntimeStore,
    run_id: &str,
) -> Result<Option<JsonValue>, String> {
    let Some(run) = store
        .get_run(run_id)
        .await
        .map_err(|error| format!("load workflow run failed: {error}"))?
    else {
        return Ok(None);
    };
    let plan = store
        .get_plan(run_id)
        .await
        .map_err(|error| format!("load workflow plan failed: {error}"))?;
    let tasks = store
        .list_tasks_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow tasks failed: {error}"))?;
    let artifacts = store
        .list_artifacts_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow artifacts failed: {error}"))?;
    let handoffs = store
        .list_handoffs_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow handoffs failed: {error}"))?;
    let stage_events = store
        .list_stage_events_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow stage events failed: {error}"))?;
    let gate_verdicts = store
        .list_gate_verdicts_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow gate verdicts failed: {error}"))?;
    let decisions = store
        .list_decisions_for_run(run_id)
        .await
        .map_err(|error| format!("load workflow decisions failed: {error}"))?;
    Ok(Some(serde_json::json!({
        "run": run,
        "plan": plan,
        "tasks": tasks,
        "artifacts": artifacts,
        "handoffs": handoffs,
        "stage_events": stage_events,
        "gate_verdicts": gate_verdicts,
        "decisions": decisions,
    })))
}

async fn build_agent_provider_context(
    client: &HostApiClient,
    runtime_paths: &Arc<RuntimePaths>,
    agent: &AgentConfig,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    run_id: &str,
    plan: Option<&JsonValue>,
) -> JsonValue {
    let extensions_runtime = client
        .call_json(ExtensionHostCapabilityRequest::ExtensionsRuntimeSnapshot)
        .await
        .unwrap_or(JsonValue::Null);
    let plan_context = plan
        .cloned()
        .filter(|value| !value.is_null())
        .and_then(|value| serde_json::from_value::<PlanSpec>(value).ok())
        .map(|value| {
            let summary = summarize_plan_steps(&value);
            let ready = validate_plan(&value).ready;
            serde_json::json!({
                "raw": value,
                "summary": summary,
                "ready": ready,
            })
        })
        .unwrap_or(JsonValue::Null);
    serde_json::json!({
        "kind": "ennoia.agent_context",
        "runtime": {
            "agent_id": agent.id,
            "agent_display_name": agent.display_name,
            "run_id": normalize_unknown(run_id),
            "sandbox_enabled": agent.execution_environment.sandbox_enabled,
            "execution_mode": if agent.execution_environment.sandbox_enabled { "sandbox" } else { "host" },
            "workspace_root": if agent.execution_environment.sandbox_enabled { "/workspace".to_string() } else { normalize_display_dir(&agent.working_dir, runtime_paths.display_for_user(runtime_paths.agent_working_dir(&agent.id))) },
            "artifacts_root": if agent.execution_environment.sandbox_enabled { "/artifacts".to_string() } else { normalize_display_dir(&agent.artifacts_dir, runtime_paths.display_for_user(runtime_paths.agent_artifacts_dir(&agent.id))) },
            "temp_root": if agent.execution_environment.sandbox_enabled { "/tmp" } else { "系统临时目录" },
        },
        "conversation": {
            "conversation_id": conversation_id,
            "lane_id": lane_id,
            "message_id": message_id,
        },
        "workflow": {
            "run_id": normalize_unknown(run_id),
            "plan": plan_context,
        },
        "extensions": extract_conversation_extensions(&extensions_runtime),
        "skills": agent.skills,
        "tools": build_agent_tool_contexts(&extensions_runtime, agent),
    })
}

fn build_agent_tool_contexts(snapshot: &JsonValue, agent: &AgentConfig) -> Vec<JsonValue> {
    let mut tools = extract_conversation_extensions(snapshot)
        .into_iter()
        .flat_map(|extension| {
            extension
                .get("capabilities")
                .and_then(JsonValue::as_array)
                .cloned()
                .unwrap_or_default()
                .into_iter()
                .map(|capability| {
                    let capability_id = capability
                        .get("id")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();
                    let kind = capability
                        .get("kind")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();
                    let contract = capability
                        .get("contract")
                        .and_then(JsonValue::as_str)
                        .unwrap_or_default();
                    serde_json::json!({
                        "extension_id": extension.get("id").and_then(JsonValue::as_str).unwrap_or_default(),
                        "extension_name": extension.get("name").and_then(JsonValue::as_str).unwrap_or_default(),
                        "capability_id": capability_id,
                        "label": humanize_agent_tool_label(capability_id, contract),
                        "summary": humanize_agent_tool_summary(capability_id, kind, contract),
                        "kind": kind,
                        "contract": contract,
                    })
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    tools.extend([
        serde_json::json!({
            "extension_id": "runtime",
            "extension_name": "Runtime",
            "capability_id": "fs.read",
            "label": "文件读取",
            "summary": if agent.execution_environment.sandbox_enabled {
                "读取文本文件；优先使用 /workspace、/artifacts、/tmp 这些路径。"
            } else {
                "读取文本文件；相对路径默认按当前工作目录解析，也可以直接使用宿主机绝对路径。"
            },
            "kind": "builtin",
            "contract": "fs.read",
        }),
        serde_json::json!({
            "extension_id": "runtime",
            "extension_name": "Runtime",
            "capability_id": "fs.write",
            "label": "文件写入",
            "summary": if agent.execution_environment.sandbox_enabled {
                "把文本写入文件；优先使用 /workspace、/artifacts、/tmp 这些路径。"
            } else {
                "把文本写入文件；相对路径默认按当前工作目录解析，也可以直接使用宿主机绝对路径。"
            },
            "kind": "builtin",
            "contract": "fs.write",
        }),
        serde_json::json!({
            "extension_id": "runtime",
            "extension_name": "Runtime",
            "capability_id": "command.exec",
            "label": "命令执行",
            "summary": "执行系统命令，并返回 stdout、stderr 和退出码。",
            "kind": "builtin",
            "contract": "command.exec",
        }),
        serde_json::json!({
            "extension_id": "runtime",
            "extension_name": "Runtime",
            "capability_id": "net.fetch",
            "label": "网络请求",
            "summary": "向外部 URL 发起 HTTP 请求，并返回状态码、响应头和文本内容。",
            "kind": "builtin",
            "contract": "net.fetch",
        }),
    ]);
    tools
}

fn extract_conversation_extensions(snapshot: &JsonValue) -> Vec<JsonValue> {
    snapshot
        .get("extensions")
        .and_then(JsonValue::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|extension| {
            extension
                .get("conversation")
                .and_then(|value| value.get("inject"))
                .and_then(JsonValue::as_bool)
                .unwrap_or(false)
        })
        .collect()
}

fn serialize_tool_message_envelope(
    tool_call: &AgentToolCall,
    outcome: Result<JsonValue, &HostApiError>,
) -> Result<String, serde_json::Error> {
    let envelope = match outcome {
        Ok(result) => ToolMessageEnvelope {
            kind: "ennoia.tool_call",
            tool_call_id: tool_call.id.clone(),
            tool_name: tool_call.name.trim().replace('_', "."),
            status: "succeeded".to_string(),
            arguments: tool_call.arguments.clone(),
            result: Some(result),
            error: None,
        },
        Err(error) => ToolMessageEnvelope {
            kind: "ennoia.tool_call",
            tool_call_id: tool_call.id.clone(),
            tool_name: tool_call.name.trim().replace('_', "."),
            status: "failed".to_string(),
            arguments: tool_call.arguments.clone(),
            result: None,
            error: Some(ToolMessageError {
                code: error_code_string(error.code()),
                message: error.message().to_string(),
                details: error.details().clone(),
            }),
        },
    };
    serde_json::to_string(&envelope)
}

fn serialize_reasoning_message_envelope(reasoning: &str) -> Result<String, serde_json::Error> {
    serde_json::to_string(&ReasoningMessageEnvelope {
        kind: "ennoia.reasoning",
        format: "markdown",
        content: reasoning.trim().to_string(),
    })
}

fn is_recent_receipt_timestamp(updated_at: &str, stale_after_ms: u64) -> bool {
    let Ok(updated_at) = chrono::DateTime::parse_from_rfc3339(updated_at) else {
        return false;
    };
    let age_ms = Utc::now()
        .signed_duration_since(updated_at.with_timezone(&Utc))
        .num_milliseconds();
    age_ms >= 0 && age_ms < stale_after_ms.min(i64::MAX as u64) as i64
}

fn read_provider_reasoning(response: &JsonValue) -> Option<String> {
    response
        .get("reasoning")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
}

fn model_endpoint_runtime_request_config(model_endpoint: &ModelEndpointConfig) -> JsonValue {
    serde_json::json!({
        "id": model_endpoint.id,
        "display_name": model_endpoint.display_name,
        "kind": model_endpoint.kind,
        "description": model_endpoint.description,
        "base_url": model_endpoint.base_url,
        "api_key": model_endpoint.api_key,
        "api_key_env": model_endpoint.api_key_env,
        "request_timeout_ms": model_endpoint.request_timeout_ms,
        "default_model": model_endpoint.default_model,
        "available_models": model_endpoint.available_models,
        "model_discovery": model_endpoint.model_discovery,
        "enabled": model_endpoint.enabled,
    })
}

fn build_agent_runtime_prompt(agent: &AgentConfig, run_id: &str) -> String {
    let execution_mode = if agent.execution_environment.sandbox_enabled {
        "sandbox"
    } else {
        "host"
    };
    let workspace_root = if agent.execution_environment.sandbox_enabled {
        "/workspace".to_string()
    } else {
        normalize_display_dir(&agent.working_dir, "当前工作目录".to_string())
    };
    let artifacts_root = if agent.execution_environment.sandbox_enabled {
        "/artifacts".to_string()
    } else {
        normalize_display_dir(&agent.artifacts_dir, "当前产物目录".to_string())
    };
    let mut sections = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        sections.push(agent.system_prompt.trim().to_string());
    }
    sections.push(format!(
        "你当前运行在 Ennoia 会话系统中。\nagent_id：{}\nagent_name：{}\nrun_id：{}\nsandbox_enabled：{}\nexecution_mode：{}\nworkspace_root：{}\nartifacts_root：{}\ntemp_root：{}\n{}\n除非用户明确需要，否则不要主动复述内部路径或实现细节。直接回答用户，不要伪装成“系统已接收”或“正在处理中”。",
        agent.id,
        agent.display_name,
        if run_id.trim().is_empty() { "unknown" } else { run_id },
        agent.execution_environment.sandbox_enabled,
        execution_mode,
        workspace_root,
        artifacts_root,
        if agent.execution_environment.sandbox_enabled { "/tmp" } else { "系统临时目录" },
        if agent.execution_environment.sandbox_enabled {
            "当前处于原生沙盒模式，只使用 /workspace、/artifacts、/tmp 这些虚拟路径。不要把宿主机绝对路径当作可直接访问的路径。"
        } else {
            "当前直接运行在宿主机环境。可以使用宿主机绝对路径；相对路径按当前工作目录解析。不要把普通的目录或命令错误解释成沙箱、容器或权限限制。"
        },
    ));
    sections.push(
        "系统会额外提供结构化上下文。按字段理解并使用，不要向用户原样复述 JSON。".to_string(),
    );
    sections.push("如果用户明确询问你有哪些工具或能力，优先依据上下文里的 tools 字段回答，使用 label 和 summary 做自然语言说明；不要把原始 JSON 对象或 `[object Object]` 直接输出给用户。".to_string());
    sections.push("当用户要求你读取文件、写入文件、执行命令或访问网页/API，并且这些能力已经出现在 tools 字段里时，优先直接调用相应工具完成任务。只有在工具调用被权限系统拒绝或需要审批时，才解释阻塞原因。遇到普通的文件系统或命令执行错误时，按实际错误原因说明。".to_string());
    sections.join("\n\n")
}

fn build_agent_builtin_tool_specs(_agent: &AgentConfig) -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "fs_read".to_string(),
            description: "读取文本文件内容。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "要读取的文件路径" },
                    "max_bytes": { "type": "integer", "description": "最多读取多少字节，默认 32768" }
                },
                "required": ["path"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "fs_write".to_string(),
            description: "把文本写入文件。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "目标文件路径" },
                    "content": { "type": "string", "description": "要写入的文本内容" },
                    "append": { "type": "boolean", "description": "是否追加写入，默认 false" }
                },
                "required": ["path", "content"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "command_exec".to_string(),
            description: "执行系统命令；command 只填可执行程序名，参数拆到 args 里。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "command": { "type": "string" },
                    "args": { "type": "array", "items": { "type": "string" } },
                    "cwd": { "type": "string" },
                    "timeout_ms": { "type": "integer" }
                },
                "required": ["command"],
                "additionalProperties": false
            }),
        },
        ToolSpec {
            name: "net_fetch".to_string(),
            description: "发起 HTTP 请求并返回响应摘要。".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "url": { "type": "string" },
                    "method": { "type": "string" },
                    "headers": { "type": "object", "additionalProperties": { "type": "string" } },
                    "body": { "type": "string" },
                    "timeout_ms": { "type": "integer" }
                },
                "required": ["url"],
                "additionalProperties": false
            }),
        },
    ]
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

fn visible_recent_messages(conversation_messages: &JsonValue, agent_id: &str) -> Vec<String> {
    normalize_conversation_messages_for_provider(conversation_messages, agent_id)
        .into_iter()
        .filter_map(|message| {
            let sender = message
                .get("sender")
                .and_then(JsonValue::as_str)
                .unwrap_or_default();
            let role = message
                .get("role")
                .and_then(JsonValue::as_str)
                .unwrap_or_default();
            let body = message
                .get("body")
                .and_then(JsonValue::as_str)
                .unwrap_or_default();
            if body.trim().is_empty() {
                None
            } else {
                Some(format!("{role}:{sender}:{body}"))
            }
        })
        .collect()
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
        "operator" | "user" => {
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

fn run_response_id(run_response: &JsonValue) -> Option<&str> {
    run_response
        .get("run")
        .and_then(|item| item.get("id"))
        .and_then(JsonValue::as_str)
}

fn run_stage_can_be_resumed(stage: RunStage) -> bool {
    !matches!(
        stage,
        RunStage::Completed | RunStage::Failed | RunStage::Cancelled
    )
}

fn run_response_has_assigned_agent(run_response: &JsonValue, agent_id: &str) -> bool {
    run_response
        .get("tasks")
        .and_then(JsonValue::as_array)
        .is_some_and(|tasks| {
            tasks.iter().any(|task| {
                task.get("assigned_agent_id").and_then(JsonValue::as_str) == Some(agent_id)
            })
        })
}

fn is_workflow_execution_confirmation(body: &str) -> bool {
    let normalized = body
        .trim()
        .chars()
        .filter(|ch| {
            !ch.is_whitespace()
                && !matches!(
                    ch,
                    '，' | '。' | '！' | '？' | ',' | '.' | '!' | '?' | ';' | '；' | ':' | '：'
                )
        })
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "继续"
            | "执行"
            | "开始"
            | "继续执行"
            | "开始执行"
            | "去改吧"
            | "执行吧"
            | "确认执行"
            | "继续吧"
            | "开始吧"
            | "可以执行"
            | "继续处理"
            | "开始处理"
    )
}

fn build_workflow_memory_sources(
    conversation_id: &str,
    message_id: Option<&str>,
    run_id: &str,
    artifacts: &[JsonValue],
) -> Vec<JsonValue> {
    let mut sources = vec![
        serde_json::json!({ "kind": "conversation", "reference": conversation_id }),
        serde_json::json!({ "kind": "workflow.run", "reference": run_id }),
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

fn owner_kind_str(kind: &OwnerKind) -> &'static str {
    match kind {
        OwnerKind::Global => "global",
        OwnerKind::Agent => "agent",
        OwnerKind::Space => "space",
    }
}

fn permission_actor_context(
    agent_id: &str,
    kind: &str,
    conversation_id: Option<&str>,
    run_id: Option<&str>,
    message_id: Option<&str>,
) -> JsonValue {
    serde_json::json!({
        "permission_actor": {
            "agent_id": agent_id,
            "kind": kind,
            "user_initiated": true,
            "conversation_id": conversation_id,
            "run_id": run_id,
            "message_id": message_id,
        }
    })
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
        .map(str::to_string)
        .collect()
}

fn payload_owner(payload: &JsonValue) -> Option<OwnerRef> {
    serde_json::from_value(payload.get("owner")?.clone()).ok()
}

fn humanize_agent_tool_label(id: &str, contract: &str) -> String {
    match id {
        "memory.ingest" => "记忆写入".to_string(),
        "memory.query" => "记忆查询".to_string(),
        "memory.review" => "记忆审查".to_string(),
        "memory.build_context" => "上下文组装".to_string(),
        "run.create" => "运行创建".to_string(),
        "run.get" => "运行详情".to_string(),
        "run.list" => "运行列表".to_string(),
        "task.list" => "任务列表".to_string(),
        "artifact.list" => "产物列表".to_string(),
        _ => contract.to_string(),
    }
}

fn humanize_agent_tool_summary(id: &str, kind: &str, contract: &str) -> String {
    match id {
        "memory.ingest" => "把当前会话里值得保留的信息写入记忆库。".to_string(),
        "memory.query" => "按主题、关键词或上下文从记忆库里检索已有信息。".to_string(),
        "memory.review" => "回顾、审查和整理已有记忆记录。".to_string(),
        "memory.build_context" => "从记忆库里提取相关片段，为当前任务补充上下文。".to_string(),
        "run.create" => "为当前问题创建一条 workflow 执行流程。".to_string(),
        "run.get" => "查看某条 workflow run 的当前状态和详情。".to_string(),
        "run.list" => "按当前会话列出相关的 workflow runs。".to_string(),
        "task.list" => "查看某条 workflow run 拆分出的任务清单。".to_string(),
        "artifact.list" => "查看某条 workflow run 产出的文件和结果。".to_string(),
        _ => match kind {
            "action" => format!("可执行动作，合同标识为 {contract}。"),
            "query" => format!("查询能力，合同标识为 {contract}。"),
            _ => format!("能力入口，合同标识为 {contract}。"),
        },
    }
}

fn read_server_config(runtime_paths: &Arc<RuntimePaths>) -> Result<ServerConfig, String> {
    let contents = fs::read_to_string(runtime_paths.server_config_file())
        .map_err(|error| format!("read server config failed: {error}"))?;
    toml::from_str::<ServerConfig>(&contents)
        .map(|config| config.normalize())
        .map_err(|error| format!("parse server config failed: {error}"))
}

fn load_agent_configs(paths: &Arc<RuntimePaths>) -> Result<Vec<AgentConfig>, String> {
    let mut agents = load_agent_documents(paths)?
        .into_iter()
        .map(|document| document.profile)
        .collect::<Vec<_>>();
    for agent in &mut agents {
        if agent.model_id.is_empty() && !agent.default_model.is_empty() {
            agent.model_id = agent.default_model.clone();
        }
        if agent.default_model.is_empty() && !agent.model_id.is_empty() {
            agent.default_model = agent.model_id.clone();
        }
        if !agent.working_dir.is_empty() {
            agent.working_dir = paths.display_for_user(paths.expand_home_token(&agent.working_dir));
        } else {
            agent.working_dir = paths.display_for_user(paths.agent_working_dir(&agent.id));
        }
        if !agent.artifacts_dir.is_empty() {
            agent.artifacts_dir =
                paths.display_for_user(paths.expand_home_token(&agent.artifacts_dir));
        } else {
            agent.artifacts_dir = paths.display_for_user(paths.agent_artifacts_dir(&agent.id));
        }
    }
    Ok(agents)
}

fn load_agent_documents(paths: &Arc<RuntimePaths>) -> Result<Vec<AgentDocument>, String> {
    if !paths.agents_dir().exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in fs::read_dir(paths.agents_dir()).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_dir()
        {
            continue;
        }
        let path = entry.path().join("agent.toml");
        if !path.exists() {
            continue;
        }
        let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
        items.push(toml::from_str(&contents).map_err(|error| error.to_string())?);
    }
    Ok(items)
}

fn load_model_endpoint_configs(
    paths: &Arc<RuntimePaths>,
) -> Result<Vec<ModelEndpointConfig>, String> {
    if !paths.model_endpoints_config_dir().exists() {
        return Ok(Vec::new());
    }
    let mut items = Vec::new();
    for entry in
        fs::read_dir(paths.model_endpoints_config_dir()).map_err(|error| error.to_string())?
    {
        let entry = entry.map_err(|error| error.to_string())?;
        if !entry
            .file_type()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            continue;
        }
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("toml") {
            continue;
        }
        let contents = fs::read_to_string(path).map_err(|error| error.to_string())?;
        items.push(toml::from_str(&contents).map_err(|error| error.to_string())?);
    }
    Ok(items)
}

fn normalize_display_dir(value: &str, fallback: String) -> String {
    if value.trim().is_empty() {
        fallback
    } else {
        value.trim().to_string()
    }
}

fn normalize_unknown(value: &str) -> String {
    if value.trim().is_empty() {
        "unknown".to_string()
    } else {
        value.trim().to_string()
    }
}

fn format_host_api_error_for_conversation(error: &HostApiError) -> String {
    let message = error.message().trim();
    if message.is_empty() {
        return "系统错误".to_string();
    }
    if message.starts_with("系统内部错误")
        || message.starts_with("系统错误")
        || message.starts_with("上游模型错误")
        || message.starts_with("请求超时")
        || message.starts_with("配置错误")
        || message.starts_with("扩展运行错误")
        || message.starts_with("沙盒路径已拦截")
    {
        return message.to_string();
    }

    let normalized = message.to_lowercase();
    let heading = if normalized.contains("native sandbox only accepts")
        || normalized.contains("path cannot escape the selected execution root")
        || normalized.contains("path must stay inside the selected execution root")
    {
        "沙盒路径已拦截"
    } else if normalized.contains("openai api key is missing")
        || normalized.contains("openai request failed")
        || normalized.contains("upstream returned")
        || normalized.contains("provider returned empty")
        || normalized.contains("当前上游不支持")
    {
        "上游模型错误"
    } else if matches!(error.code(), ErrorCode::Timeout)
        || normalized.contains("request timeout:")
        || normalized.contains("request timeout after")
        || normalized.contains("timed out")
    {
        "请求超时"
    } else if normalized.contains("provider invoke requires params.model_endpoint")
        || normalized.contains("missing field `display_name`")
        || normalized.contains("missing field")
        || normalized.contains("invalid configuration")
    {
        "配置错误"
    } else if normalized.contains("extension rpc failed")
        || normalized.contains("method_not_found")
        || normalized.contains("conversation worker method")
        || normalized.contains("parse extension record")
    {
        "扩展运行错误"
    } else if matches!(error.code(), ErrorCode::BadRequest | ErrorCode::Internal) {
        "系统错误"
    } else {
        "执行失败"
    };

    format!("{heading}\n{message}")
}

fn internal_error(message: impl Into<String>) -> HostApiError {
    HostApiError {
        body: ApiErrorBody {
            code: ErrorCode::Internal,
            message: message.into(),
            request_id: None,
            trace_id: None,
            details: JsonValue::Null,
            retryable: true,
        },
    }
}

fn host_response_error(response: ennoia_kernel::ExtensionRpcResponse) -> HostApiError {
    let Some(error) = response.error else {
        return internal_error("host capability returned failure without error payload");
    };
    HostApiError {
        body: ApiErrorBody {
            code: parse_host_error_code(&error.code),
            message: error.message,
            request_id: None,
            trace_id: None,
            details: JsonValue::Null,
            retryable: matches!(
                parse_host_error_code(&error.code),
                ErrorCode::Internal | ErrorCode::Timeout | ErrorCode::RateLimited
            ),
        },
    }
}

fn parse_host_error_code(code: &str) -> ErrorCode {
    match code.trim().to_ascii_lowercase().as_str() {
        "bad_request" => ErrorCode::BadRequest,
        "unauthorized" => ErrorCode::Unauthorized,
        "forbidden" => ErrorCode::Forbidden,
        "not_found" => ErrorCode::NotFound,
        "conflict" => ErrorCode::Conflict,
        "rate_limited" => ErrorCode::RateLimited,
        "timeout" => ErrorCode::Timeout,
        "payload_too_large" => ErrorCode::PayloadTooLarge,
        _ => ErrorCode::Internal,
    }
}

fn bad_request_error(message: impl Into<String>) -> HostApiError {
    HostApiError {
        body: ApiErrorBody {
            code: ErrorCode::BadRequest,
            message: message.into(),
            request_id: None,
            trace_id: None,
            details: JsonValue::Null,
            retryable: false,
        },
    }
}

fn not_found_error(message: impl Into<String>) -> HostApiError {
    HostApiError {
        body: ApiErrorBody {
            code: ErrorCode::NotFound,
            message: message.into(),
            request_id: None,
            trace_id: None,
            details: JsonValue::Null,
            retryable: false,
        },
    }
}

fn error_code_string(code: ErrorCode) -> String {
    match code {
        ErrorCode::BadRequest => "bad_request",
        ErrorCode::Unauthorized => "unauthorized",
        ErrorCode::Forbidden => "forbidden",
        ErrorCode::NotFound => "not_found",
        ErrorCode::Conflict => "conflict",
        ErrorCode::RateLimited => "rate_limited",
        ErrorCode::Timeout => "timeout",
        ErrorCode::PayloadTooLarge => "payload_too_large",
        ErrorCode::Internal => "internal",
    }
    .to_string()
}
