use std::fs;
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use ennoia_contract::behavior::{BehaviorRunRequest, BehaviorSourceRef, BehaviorTrigger};
use ennoia_contract::{ApiErrorBody, ErrorCode};
use ennoia_kernel::{
    AgentConfig, AgentDocument, DecisionSnapshot, ExtensionHostCapabilityRequest,
    ExtensionRecordAppend, ExtensionRecordEntry, ExtensionRecordUpdate, ExtensionStateEntry,
    ExtensionStateGetQuery, ExtensionStatePut, HookDispatchResponse, HookEventEnvelope,
    ModelEndpointConfig, NextAction, OperationPerformRequest, OperationPerformResponse,
    OperationRecord, OwnerKind, OwnerRef, RunContext, RunStage, RunStageEvent, RuntimeProfile,
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

fn workflow_trace(
    phase: &str,
    conversation_id: &str,
    message_id: Option<&str>,
    agent_id: &str,
    detail: impl AsRef<str>,
) {
    eprintln!(
        "[workflow][{phase}] conv={} msg={} agent={} {}",
        conversation_id,
        message_id.unwrap_or("-"),
        agent_id,
        detail.as_ref()
    );
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentReplyProgress {
    Completed(String),
    Pending,
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

#[derive(Debug, Clone, Deserialize, Serialize)]
struct AgentToolCall {
    id: String,
    name: String,
    #[serde(default)]
    arguments: JsonValue,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
struct DirectReplyState {
    run_id: String,
    conversation_id: String,
    lane_id: Option<String>,
    message_id: Option<String>,
    agent_id: String,
    messages: Vec<JsonValue>,
    pending_tool_calls: Vec<AgentToolCall>,
    pending_operation_id: Option<String>,
    pending_operation_kind: Option<String>,
    next_iteration: usize,
    last_process_text: Option<String>,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone)]
struct AgentReplyLoopState {
    messages: Vec<JsonValue>,
    pending_tool_calls: Vec<AgentToolCall>,
    pending_operation_id: Option<String>,
    pending_operation_kind: Option<String>,
    next_iteration: usize,
    last_process_text: Option<String>,
}

#[derive(Debug, Clone)]
struct OperatorProfileSnapshot {
    display_name: String,
    time_zone: Option<String>,
    operating_system: Option<String>,
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
        agent_id: &str,
        conversation_id: &str,
        lane_id: Option<&str>,
        run_id: &str,
        message_id: Option<&str>,
    ) -> Result<JsonValue, HostApiError> {
        let response = self
            .perform_operation(OperationPerformRequest {
                agent_id: agent_id.to_string(),
                conversation_id: conversation_id.to_string(),
                run_id: run_id.to_string(),
                branch_id: None,
                lane_id: lane_id.map(str::to_string),
                message_id: message_id.map(str::to_string),
                kind: "provider".to_string(),
                name: "generate".to_string(),
                deferred: false,
                input: serde_json::json!({
                    "provider_kind": provider_kind,
                    "params": payload,
                }),
            })
            .await?;
        Ok(response.content)
    }

    async fn provider_generate_deferred(
        &self,
        provider_kind: &str,
        payload: JsonValue,
        agent_id: &str,
        conversation_id: &str,
        lane_id: Option<&str>,
        run_id: &str,
        message_id: Option<&str>,
    ) -> Result<OperationPerformResponse, HostApiError> {
        self.perform_operation(OperationPerformRequest {
            agent_id: agent_id.to_string(),
            conversation_id: conversation_id.to_string(),
            run_id: run_id.to_string(),
            branch_id: None,
            lane_id: lane_id.map(str::to_string),
            message_id: message_id.map(str::to_string),
            kind: "provider".to_string(),
            name: "generate".to_string(),
            deferred: true,
            input: serde_json::json!({
                "provider_kind": provider_kind,
                "params": payload,
            }),
        })
        .await
    }

    async fn execute_operation_deferred(
        &self,
        operation: &str,
        agent_id: &str,
        conversation_id: &str,
        lane_id: Option<&str>,
        run_id: &str,
        message_id: Option<&str>,
        arguments: JsonValue,
    ) -> Result<OperationPerformResponse, HostApiError> {
        self.perform_operation(OperationPerformRequest {
            agent_id: agent_id.to_string(),
            conversation_id: conversation_id.to_string(),
            run_id: run_id.to_string(),
            branch_id: None,
            lane_id: lane_id.map(str::to_string),
            message_id: message_id.map(str::to_string),
            kind: "runtime".to_string(),
            name: operation.to_string(),
            deferred: true,
            input: arguments,
        })
        .await
    }

    async fn perform_operation(
        &self,
        payload: OperationPerformRequest,
    ) -> Result<OperationPerformResponse, HostApiError> {
        let value = self
            .call_json(ExtensionHostCapabilityRequest::OperationPerform { payload })
            .await?;
        serde_json::from_value::<OperationPerformResponse>(value)
            .map_err(|error| internal_error(format!("parse operation response failed: {error}")))
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

pub async fn handle_operation_updated(
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
    envelope: HookEventEnvelope,
) -> Result<HookDispatchResponse, String> {
    let operation: OperationRecord = serde_json::from_value(
        envelope
            .payload
            .get("operation")
            .cloned()
            .unwrap_or(JsonValue::Null),
    )
    .map_err(|error| format!("parse operation payload failed: {error}"))?;
    workflow_trace(
        "operation.updated",
        &operation.conversation_id,
        operation.message_id.as_deref(),
        &operation.agent_id,
        format!(
            "operation_id={} kind={}.{} status={:?}",
            operation.id, operation.kind, operation.name, operation.status
        ),
    );
    if !matches!(
        operation.status,
        ennoia_kernel::OperationStatus::Succeeded
            | ennoia_kernel::OperationStatus::Failed
            | ennoia_kernel::OperationStatus::Cancelled
    ) {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    }
    let Some(direct_state) = load_direct_reply_state(store, &operation.run_id).await? else {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    };
    if direct_state.pending_operation_id.as_deref() != Some(operation.id.as_str()) {
        return Ok(HookDispatchResponse {
            handled: true,
            result: None,
            message: None,
        });
    }
    let client = HostApiClient::new(&runtime.runtime_paths)?;
    let agents = load_agent_configs(&runtime.runtime_paths)?;
    let model_endpoints = load_model_endpoint_configs(&runtime.runtime_paths)?;
    let conversation_messages = client
        .dispatch_action(
            "message.list",
            serde_json::json!({ "conversation_id": operation.conversation_id }),
            JsonValue::Null,
        )
        .await
        .map_err(|error| error.to_string())?;
    let loop_state = AgentReplyLoopState {
        messages: direct_state.messages,
        pending_tool_calls: direct_state.pending_tool_calls,
        pending_operation_id: direct_state.pending_operation_id,
        pending_operation_kind: direct_state.pending_operation_kind,
        next_iteration: direct_state.next_iteration,
        last_process_text: direct_state.last_process_text,
    };
    match continue_agent_reply_from_state(
        &client,
        &runtime.runtime_paths,
        &agents,
        &model_endpoints,
        &operation.conversation_id,
        direct_state.lane_id.as_deref(),
        direct_state.message_id.as_deref(),
        &conversation_messages,
        None,
        &operation.run_id,
        &operation.agent_id,
        loop_state,
        Some(store),
        Some(operation.clone()),
    )
    .await
    {
        Ok(AgentReplyProgress::Completed(reply_body)) => {
            append_agent_conversation_reply(
                &client,
                &operation.conversation_id,
                direct_state.lane_id.as_deref(),
                direct_state.message_id.as_deref(),
                &operation.agent_id,
                if operation.run_id.starts_with("direct-") {
                    None
                } else {
                    Some(operation.run_id.as_str())
                },
                &reply_body,
            )
            .await?;
        }
        Ok(AgentReplyProgress::Pending) => {}
        Err(error) if error.is_permission_approval() => {}
        Err(error) => {
            append_agent_conversation_reply(
                &client,
                &operation.conversation_id,
                direct_state.lane_id.as_deref(),
                direct_state.message_id.as_deref(),
                &operation.agent_id,
                if operation.run_id.starts_with("direct-") {
                    None
                } else {
                    Some(operation.run_id.as_str())
                },
                &format_host_api_error_for_conversation(&error),
            )
            .await?;
        }
    }
    Ok(HookDispatchResponse {
        handled: true,
        result: None,
        message: None,
    })
}

pub async fn recover_stale_conversation_receipts(
    runtime: &WorkflowRuntime,
    store: &SqliteRuntimeStore,
) -> Result<(), String> {
    let client = HostApiClient::new(&runtime.runtime_paths)?;
    let rows = sqlx::query(
        "SELECT conversation_id, message_id, agent_id, updated_at
         FROM conversation_message_receipts
         WHERE status = 'running'
         ORDER BY updated_at ASC",
    )
    .fetch_all(store.pool())
    .await
    .map_err(|error| format!("load running conversation receipts failed: {error}"))?;
    let row_count = rows.len();

    if row_count > 0 {
        eprintln!(
            "[workflow] recovering {} orphaned conversation receipt(s)",
            row_count
        );
    }

    for row in rows {
        let conversation_id = row.get::<String, _>("conversation_id");
        let message_id = row.get::<String, _>("message_id");
        let agent_id = row.get::<String, _>("agent_id");
        let result: Result<(), String> = async {
            let messages = client
                .dispatch_action(
                    "message.list",
                    serde_json::json!({ "conversation_id": conversation_id }),
                    JsonValue::Null,
                )
                .await
                .map_err(|error| error.to_string())?;
            let Some(message_list) = messages.as_array() else {
                return Err("conversation message list payload is not an array".to_string());
            };
            let Some(source_message) = message_list.iter().find(|item| {
                item.get("id").and_then(JsonValue::as_str) == Some(message_id.as_str())
            }) else {
                mark_conversation_message_receipt_status(
                    store,
                    &conversation_id,
                    Some(&message_id),
                    &agent_id,
                    "failed",
                )
                .await?;
                return Ok(());
            };

            let source_branch_scope = workflow_branch_scope_id(
                payload_string_field(source_message, &["branch_id"]).as_deref(),
                payload_string_field(source_message, &["lane_id"]).as_deref(),
            );
            let source_created_at =
                payload_string_field(source_message, &["created_at"]).unwrap_or_default();
            let already_replied = message_list.iter().any(|item| {
                payload_string_field(item, &["role"]).as_deref() == Some("agent")
                    && payload_string_field(item, &["sender"]).as_deref() == Some(agent_id.as_str())
                    && payload_string_field(item, &["created_at"])
                        .is_some_and(|created_at| created_at >= source_created_at)
                    && workflow_branch_scope_id(
                        payload_string_field(item, &["branch_id"]).as_deref(),
                        payload_string_field(item, &["lane_id"]).as_deref(),
                    ) == source_branch_scope
            });
            if already_replied {
                mark_conversation_message_receipt_status(
                    store,
                    &conversation_id,
                    Some(&message_id),
                    &agent_id,
                    "completed",
                )
                .await?;
                return Ok(());
            }

            let payload = serde_json::json!({
                "conversation": { "id": conversation_id },
                "message": source_message,
                "addressed_agents": [agent_id],
                "workflow_receipt_recovery": true,
            });
            generate_conversation_agent_reply(&client, runtime, store, &payload).await
        }
        .await;

        if let Err(error) = result {
            eprintln!(
                "[workflow] conversation receipt recovery failed: conversation_id={conversation_id} message_id={message_id} agent_id={agent_id} error={error}"
            );
            let _ = mark_conversation_message_receipt_status(
                store,
                &conversation_id,
                Some(&message_id),
                &agent_id,
                "failed",
            )
            .await;
        }
    }

    if row_count > 0 {
        eprintln!("[workflow] orphaned conversation receipt recovery finished");
    }

    Ok(())
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
    let workflow_receipt_recovery = payload
        .get("workflow_receipt_recovery")
        .and_then(JsonValue::as_bool)
        .unwrap_or(false);
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
        workflow_trace(
            "conversation.start",
            &conversation_id,
            message_id.as_deref(),
            agent_id,
            format!(
                "body_len={} recovery={} branch_scope={}",
                body.len(),
                workflow_receipt_recovery,
                branch_scope
            ),
        );
        let claimed = if workflow_receipt_recovery {
            if let Some(message_id) = message_id.as_deref() {
                mark_conversation_message_receipt_status(
                    store,
                    &conversation_id,
                    Some(message_id),
                    agent_id,
                    "running",
                )
                .await?;
            }
            true
        } else {
            claim_conversation_message_receipt(
                store,
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                client.processing_stale_after_ms,
            )
            .await?
        };
        if !claimed {
            workflow_trace(
                "conversation.skip",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                "receipt already claimed",
            );
            continue;
        }
        let agent_result: Result<(), String> = async {
            let started = Instant::now();
            workflow_trace(
                "message.list.start",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                "loading conversation messages",
            );
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
            workflow_trace(
                "message.list.done",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                format!(
                    "elapsed_ms={} count={}",
                    started.elapsed().as_millis(),
                    conversation_messages.as_array().map(|items| items.len()).unwrap_or(0)
                ),
            );
            let memory_started = Instant::now();
            workflow_trace(
                "memory.start",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                "assembling memory context",
            );
            let memory_context = assemble_memory_context(
                client,
                &owner,
                &conversation_id,
                visible_recent_messages(&runtime.runtime_paths, &conversation_messages, agent_id),
                permission_actor_context(
                    agent_id,
                    "workflow.memory_context",
                    Some(&conversation_id),
                    None,
                    message_id.as_deref(),
                ),
            )
            .await;
            workflow_trace(
                "memory.done",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                format!(
                    "elapsed_ms={} has_context={}",
                    memory_started.elapsed().as_millis(),
                    memory_context.is_some()
                ),
            );
            let metadata = serde_json::json!({
                "origin": "workflow.conversation_message.created",
                "message_id": message_id,
            });

            let session_started = Instant::now();
            workflow_trace(
                "session.start",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                "loading active workflow session",
            );
            let mut active_session =
                load_active_workflow_session(client, &conversation_id, agent_id, &branch_scope)
                    .await
                    .map_err(|error| error.to_string())?;
            workflow_trace(
                "session.done",
                &conversation_id,
                message_id.as_deref(),
                agent_id,
                format!(
                    "elapsed_ms={} active={}",
                    session_started.elapsed().as_millis(),
                    active_session.is_some()
                ),
            );

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
                    workflow_trace(
                        "route",
                        &conversation_id,
                        message_id.as_deref(),
                        agent_id,
                        "managed_discussion",
                    );
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
                    workflow_trace(
                        "route",
                        &conversation_id,
                        message_id.as_deref(),
                        agent_id,
                        format!("direct_reply run_id={direct_run_id}"),
                    );
                    execute_direct_reply(
                        client,
                        store,
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
                    workflow_trace(
                        "direct.done",
                        &conversation_id,
                        message_id.as_deref(),
                        agent_id,
                        "execute_direct_reply returned",
                    );
                }
            }

            Ok(())
        }
        .await;
        workflow_trace(
            "conversation.finish",
            &conversation_id,
            message_id.as_deref(),
            agent_id,
            if agent_result.is_ok() { "ok" } else { "error" },
        );
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

async fn claim_conversation_message_receipt(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    message_id: Option<&str>,
    agent_id: &str,
    stale_after_ms: u64,
) -> Result<bool, String> {
    let Some(message_id) = message_id.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(false);
    };
    let mut transaction =
        store.pool().begin().await.map_err(|error| {
            format!("begin conversation receipt claim transaction failed: {error}")
        })?;
    let row = sqlx::query(
        "SELECT status, updated_at
         FROM conversation_message_receipts
         WHERE conversation_id = ?1 AND message_id = ?2 AND agent_id = ?3
         LIMIT 1",
    )
    .bind(conversation_id)
    .bind(message_id)
    .bind(agent_id)
    .fetch_optional(&mut *transaction)
    .await
    .map_err(|error| format!("load conversation message receipt failed: {error}"))?;
    let now = Utc::now().to_rfc3339();
    let should_claim = match row {
        Some(item) => {
            let status = item.get::<String, _>("status");
            if status == "completed" {
                false
            } else if status == "running" {
                let updated_at = item.get::<String, _>("updated_at");
                !is_recent_receipt_timestamp(&updated_at, stale_after_ms)
            } else {
                true
            }
        }
        None => true,
    };
    if should_claim {
        sqlx::query(
            "INSERT INTO conversation_message_receipts
             (conversation_id, message_id, agent_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'running', ?4, ?4)
             ON CONFLICT(conversation_id, message_id, agent_id) DO UPDATE SET
               status = 'running',
               updated_at = excluded.updated_at",
        )
        .bind(conversation_id)
        .bind(message_id)
        .bind(agent_id)
        .bind(&now)
        .execute(&mut *transaction)
        .await
        .map_err(|error| format!("claim conversation message receipt failed: {error}"))?;
    }
    transaction
        .commit()
        .await
        .map_err(|error| format!("commit conversation receipt claim failed: {error}"))?;
    Ok(should_claim)
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

async fn load_direct_reply_state(
    store: &SqliteRuntimeStore,
    run_id: &str,
) -> Result<Option<DirectReplyState>, String> {
    let row = sqlx::query(
        "SELECT run_id, conversation_id, lane_id, message_id, agent_id, messages_json,
                pending_tool_calls_json, pending_operation_id, pending_operation_kind,
                next_iteration, last_process_text, created_at, updated_at
         FROM direct_reply_states
         WHERE run_id = ?1
         LIMIT 1",
    )
    .bind(run_id)
    .fetch_optional(store.pool())
    .await
    .map_err(|error| format!("load direct reply state failed: {error}"))?;
    let Some(row) = row else {
        return Ok(None);
    };
    let messages = serde_json::from_str::<Vec<JsonValue>>(&row.get::<String, _>("messages_json"))
        .map_err(|error| format!("parse direct reply messages failed: {error}"))?;
    let pending_tool_calls = serde_json::from_str::<Vec<AgentToolCall>>(
        &row.get::<String, _>("pending_tool_calls_json"),
    )
    .map_err(|error| format!("parse direct reply tool calls failed: {error}"))?;
    Ok(Some(DirectReplyState {
        run_id: row.get("run_id"),
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
        message_id: row.get("message_id"),
        agent_id: row.get("agent_id"),
        messages,
        pending_tool_calls,
        pending_operation_id: row.get("pending_operation_id"),
        pending_operation_kind: row.get("pending_operation_kind"),
        next_iteration: row.get::<i64, _>("next_iteration") as usize,
        last_process_text: row.get("last_process_text"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }))
}

async fn save_direct_reply_state(
    store: &SqliteRuntimeStore,
    state: &DirectReplyState,
) -> Result<(), String> {
    let messages_json =
        serde_json::to_string(&state.messages).map_err(|error| error.to_string())?;
    let pending_tool_calls_json =
        serde_json::to_string(&state.pending_tool_calls).map_err(|error| error.to_string())?;
    sqlx::query(
        "INSERT INTO direct_reply_states
         (run_id, conversation_id, lane_id, message_id, agent_id, messages_json,
          pending_tool_calls_json, pending_operation_id, pending_operation_kind,
          next_iteration, last_process_text, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)
         ON CONFLICT(run_id) DO UPDATE SET
           conversation_id = excluded.conversation_id,
           lane_id = excluded.lane_id,
           message_id = excluded.message_id,
           agent_id = excluded.agent_id,
           messages_json = excluded.messages_json,
           pending_tool_calls_json = excluded.pending_tool_calls_json,
           pending_operation_id = excluded.pending_operation_id,
           pending_operation_kind = excluded.pending_operation_kind,
           next_iteration = excluded.next_iteration,
           last_process_text = excluded.last_process_text,
           updated_at = excluded.updated_at",
    )
    .bind(&state.run_id)
    .bind(&state.conversation_id)
    .bind(&state.lane_id)
    .bind(&state.message_id)
    .bind(&state.agent_id)
    .bind(messages_json)
    .bind(pending_tool_calls_json)
    .bind(&state.pending_operation_id)
    .bind(&state.pending_operation_kind)
    .bind(state.next_iteration as i64)
    .bind(&state.last_process_text)
    .bind(&state.created_at)
    .bind(&state.updated_at)
    .execute(store.pool())
    .await
    .map(|_| ())
    .map_err(|error| format!("save direct reply state failed: {error}"))
}

async fn delete_direct_reply_state(store: &SqliteRuntimeStore, run_id: &str) -> Result<(), String> {
    sqlx::query("DELETE FROM direct_reply_states WHERE run_id = ?1")
        .bind(run_id)
        .execute(store.pool())
        .await
        .map(|_| ())
        .map_err(|error| format!("delete direct reply state failed: {error}"))
}

async fn persist_direct_reply_loop_state(
    store: &SqliteRuntimeStore,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    agent_id: &str,
    run_id: &str,
    state: &AgentReplyLoopState,
) -> Result<(), String> {
    let existing = load_direct_reply_state(store, run_id).await?;
    let now = Utc::now().to_rfc3339();
    let created_at = existing
        .as_ref()
        .map(|item| item.created_at.clone())
        .unwrap_or_else(|| now.clone());
    save_direct_reply_state(
        store,
        &DirectReplyState {
            run_id: run_id.to_string(),
            conversation_id: conversation_id.to_string(),
            lane_id: lane_id.map(str::to_string),
            message_id: message_id.map(str::to_string),
            agent_id: agent_id.to_string(),
            messages: state.messages.clone(),
            pending_tool_calls: state.pending_tool_calls.clone(),
            pending_operation_id: state.pending_operation_id.clone(),
            pending_operation_kind: state.pending_operation_kind.clone(),
            next_iteration: state.next_iteration,
            last_process_text: state.last_process_text.clone(),
            created_at,
            updated_at: now,
        },
    )
    .await
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
    let base_messages = normalize_conversation_messages_for_provider(
        conversation_messages,
        agent_id,
        &resolve_operator_profile(&runtime.runtime_paths).display_name,
    );
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
        let operator_profile = resolve_operator_profile(&runtime.runtime_paths);
        let response = client
            .provider_generate(
                &model_endpoint.kind,
                serde_json::json!({
                    "model_endpoint": model_endpoint_runtime_request_config(model_endpoint),
                    "model": model_id,
                    "instructions": ProviderInstructions {
                        base: build_agent_runtime_prompt(agent, &draft_run_id, &operator_profile),
                    },
                    "system_prompt": build_agent_runtime_prompt(agent, &draft_run_id, &operator_profile),
                    "context": context,
                    "messages": messages,
                    "generation_options": agent.generation_options,
                    "tools": [],
                    "metadata": metadata,
                }),
                agent_id,
                conversation_id,
                lane_id,
                &draft_run_id,
                message_id,
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
    let reply_progress = match generate_real_agent_reply(
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
        Ok(AgentReplyProgress::Completed(reply)) => {
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
        Ok(AgentReplyProgress::Pending) => return Ok(()),
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
        &reply_progress,
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
    store: &SqliteRuntimeStore,
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
    let operator_display_name = resolve_operator_profile(runtime_paths).display_name;
    let initial_state = AgentReplyLoopState {
        messages: normalize_conversation_messages_for_provider(
            conversation_messages,
            agent_id,
            &operator_display_name,
        ),
        pending_tool_calls: Vec::new(),
        pending_operation_id: None,
        pending_operation_kind: None,
        next_iteration: 0,
        last_process_text: None,
    };
    drive_direct_reply(
        client,
        store,
        runtime_paths,
        agents,
        model_endpoints,
        conversation_id,
        lane_id,
        message_id,
        conversation_messages,
        agent_id,
        run_id,
        initial_state,
    )
    .await
}

async fn drive_direct_reply(
    client: &HostApiClient,
    store: &SqliteRuntimeStore,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    agent_id: &str,
    run_id: &str,
    initial_state: AgentReplyLoopState,
) -> Result<(), String> {
    workflow_trace(
        "direct.drive.start",
        conversation_id,
        message_id,
        agent_id,
        format!("run_id={run_id}"),
    );
    match continue_agent_reply_from_state(
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
        initial_state,
        Some(store),
        None,
    )
    .await
    {
        Ok(AgentReplyProgress::Completed(reply_body)) => {
            workflow_trace(
                "direct.drive.completed",
                conversation_id,
                message_id,
                agent_id,
                format!("run_id={run_id} reply_len={}", reply_body.len()),
            );
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
        Ok(AgentReplyProgress::Pending) => {
            workflow_trace(
                "direct.drive.pending",
                conversation_id,
                message_id,
                agent_id,
                format!("run_id={run_id}"),
            );
            Ok(())
        }
        Err(error) if error.is_permission_approval() => {
            workflow_trace(
                "direct.drive.blocked",
                conversation_id,
                message_id,
                agent_id,
                format!("run_id={run_id} error={}", error.message()),
            );
            Ok(())
        }
        Err(error) => {
            workflow_trace(
                "direct.drive.failed",
                conversation_id,
                message_id,
                agent_id,
                format!("run_id={run_id} error={}", error.message()),
            );
            let _ = delete_direct_reply_state(store, run_id).await;
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
    store: &SqliteRuntimeStore,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    conversation_messages: &JsonValue,
    run_response: &JsonValue,
    agent_id: &str,
) -> Result<AgentReplyProgress, HostApiError> {
    let run_id = run_response_id(run_response)
        .unwrap_or_default()
        .to_string();
    continue_agent_reply_from_state(
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
        AgentReplyLoopState {
            messages: normalize_conversation_messages_for_provider(
                conversation_messages,
                agent_id,
                &resolve_operator_profile(runtime_paths).display_name,
            ),
            pending_tool_calls: Vec::new(),
            pending_operation_id: None,
            pending_operation_kind: None,
            next_iteration: 0,
            last_process_text: None,
        },
        Some(store),
        None,
    )
    .await
}

async fn continue_agent_reply_from_state(
    client: &HostApiClient,
    runtime_paths: &Arc<RuntimePaths>,
    agents: &[AgentConfig],
    model_endpoints: &[ModelEndpointConfig],
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    _conversation_messages: &JsonValue,
    plan: Option<&JsonValue>,
    run_id: &str,
    agent_id: &str,
    mut state: AgentReplyLoopState,
    direct_state_store: Option<&SqliteRuntimeStore>,
    resumed_operation: Option<OperationRecord>,
) -> Result<AgentReplyProgress, HostApiError> {
    workflow_trace(
        "reply.loop.start",
        conversation_id,
        message_id,
        agent_id,
        format!(
            "run_id={} iteration={} pending_op={:?} pending_kind={:?} pending_tools={}",
            run_id,
            state.next_iteration,
            state.pending_operation_id,
            state.pending_operation_kind,
            state.pending_tool_calls.len()
        ),
    );
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
    let operator_profile = resolve_operator_profile(runtime_paths);
    let instructions = ProviderInstructions {
        base: build_agent_runtime_prompt(agent, run_id, &operator_profile),
    };
    let metadata = serde_json::json!({
        "conversation_id": conversation_id,
        "lane_id": lane_id,
        "message_id": message_id,
        "run_id": run_id,
        "file_access": agent_file_access_context(),
        "agent_id": agent.id,
        "agent_display_name": agent.display_name,
    });
    let mut resumed_operation = resumed_operation;

    loop {
        if let Some(operation) = resumed_operation.take() {
            workflow_trace(
                "reply.loop.resume",
                conversation_id,
                message_id,
                agent_id,
                format!(
                    "run_id={} operation_id={} status={:?} pending_kind={:?}",
                    run_id, operation.id, operation.status, state.pending_operation_kind
                ),
            );
            if state.pending_operation_id.as_deref() == Some(operation.id.as_str()) {
                if state.pending_operation_kind.as_deref() == Some("provider.generate") {
                    state.pending_operation_id = None;
                    state.pending_operation_kind = None;
                    if let Some(response) = operation.output.clone() {
                        if let Some(reply) = apply_provider_response(
                            client,
                            conversation_id,
                            lane_id,
                            message_id,
                            agent_id,
                            run_id,
                            &mut state,
                            &response,
                            direct_state_store,
                        )
                        .await?
                        {
                            return Ok(AgentReplyProgress::Completed(reply));
                        }
                    } else {
                        return Err(internal_error(operation_error_message(&operation)));
                    }
                } else if state
                    .pending_operation_kind
                    .as_deref()
                    .is_some_and(|value| value.starts_with("runtime."))
                {
                    let tool_call = state.pending_tool_calls.first().cloned().ok_or_else(|| {
                        internal_error("missing pending tool call for resumed operation")
                    })?;
                    state.pending_operation_id = None;
                    state.pending_operation_kind = None;
                    if operation.status == ennoia_kernel::OperationStatus::Succeeded {
                        let result = operation.output.clone().ok_or_else(|| {
                            internal_error("resumed tool operation returned no output")
                        })?;
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
                        state.pending_tool_calls.remove(0);
                        state.messages.push(serde_json::json!({
                            "role": "tool",
                            "tool_call_id": tool_call.id,
                            "content": body,
                        }));
                        if let Some(store) = direct_state_store {
                            persist_direct_reply_loop_state(
                                store,
                                conversation_id,
                                lane_id,
                                message_id,
                                agent_id,
                                run_id,
                                &state,
                            )
                            .await
                            .map_err(internal_error)?;
                        }
                    } else {
                        return Err(internal_error(operation_error_message(&operation)));
                    }
                }
            }
        }

        if let Some(tool_call) = state.pending_tool_calls.first().cloned() {
            workflow_trace(
                "tool.start",
                conversation_id,
                message_id,
                agent_id,
                format!(
                    "run_id={} tool={} tool_call_id={}",
                    run_id, tool_call.name, tool_call.id
                ),
            );
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
                Ok(response)
                    if response.operation.status == ennoia_kernel::OperationStatus::Succeeded =>
                {
                    workflow_trace(
                        "tool.done",
                        conversation_id,
                        message_id,
                        agent_id,
                        format!(
                            "run_id={} tool={} operation_id={} status={:?}",
                            run_id,
                            tool_call.name,
                            response.operation.id,
                            response.operation.status
                        ),
                    );
                    let result = operation_response_output_or_content(response);
                    state.pending_operation_id = None;
                    state.pending_operation_kind = None;
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
                    state.pending_tool_calls.remove(0);
                    state.messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": body,
                    }));
                    if let Some(store) = direct_state_store {
                        persist_direct_reply_loop_state(
                            store,
                            conversation_id,
                            lane_id,
                            message_id,
                            agent_id,
                            run_id,
                            &state,
                        )
                        .await
                        .map_err(internal_error)?;
                    }
                    continue;
                }
                Ok(response) => {
                    workflow_trace(
                        "tool.pending",
                        conversation_id,
                        message_id,
                        agent_id,
                        format!(
                            "run_id={} tool={} operation_id={} status={:?}",
                            run_id,
                            tool_call.name,
                            response.operation.id,
                            response.operation.status
                        ),
                    );
                    state.pending_operation_id = Some(response.operation.id);
                    state.pending_operation_kind = Some(format!(
                        "runtime.{}",
                        tool_call.name.trim().replace('_', ".")
                    ));
                    if let Some(store) = direct_state_store {
                        persist_direct_reply_loop_state(
                            store,
                            conversation_id,
                            lane_id,
                            message_id,
                            agent_id,
                            run_id,
                            &state,
                        )
                        .await
                        .map_err(internal_error)?;
                    }
                    return Ok(AgentReplyProgress::Pending);
                }
                Err(error) => {
                    workflow_trace(
                        "tool.error",
                        conversation_id,
                        message_id,
                        agent_id,
                        format!(
                            "run_id={} tool={} error={}",
                            run_id,
                            tool_call.name,
                            error.message()
                        ),
                    );
                    return Err(error);
                }
            }
        }

        if state.next_iteration >= 6 {
            if let Some(store) = direct_state_store {
                let _ = delete_direct_reply_state(store, run_id).await;
            }
            return Err(internal_error(
                "agent tool loop exceeded maximum iterations",
            ));
        }

        workflow_trace(
            "provider.start",
            conversation_id,
            message_id,
            agent_id,
            format!("run_id={} iteration={}", run_id, state.next_iteration),
        );
        let response = client
            .provider_generate_deferred(
                &model_endpoint.kind,
                serde_json::json!({
                    "model_endpoint": model_endpoint_runtime_request_config(model_endpoint),
                    "model": model_id,
                    "instructions": instructions,
                    "system_prompt": build_agent_runtime_prompt(agent, run_id, &operator_profile),
                    "context": context,
                    "messages": state.messages,
                    "generation_options": agent.generation_options,
                    "tools": tools,
                    "tool_choice": "auto",
                    "metadata": metadata,
                }),
                agent_id,
                conversation_id,
                lane_id,
                run_id,
                message_id,
            )
            .await;
        let response = match response {
            Ok(response)
                if response.operation.status == ennoia_kernel::OperationStatus::Succeeded =>
            {
                workflow_trace(
                    "provider.done",
                    conversation_id,
                    message_id,
                    agent_id,
                    format!(
                        "run_id={} operation_id={} status={:?}",
                        run_id, response.operation.id, response.operation.status
                    ),
                );
                let response = operation_response_output_or_content(response);
                state.pending_operation_id = None;
                state.pending_operation_kind = None;
                response
            }
            Ok(response) => {
                workflow_trace(
                    "provider.pending",
                    conversation_id,
                    message_id,
                    agent_id,
                    format!(
                        "run_id={} operation_id={} status={:?}",
                        run_id, response.operation.id, response.operation.status
                    ),
                );
                state.pending_operation_id = Some(response.operation.id);
                state.pending_operation_kind = Some("provider.generate".to_string());
                if let Some(store) = direct_state_store {
                    persist_direct_reply_loop_state(
                        store,
                        conversation_id,
                        lane_id,
                        message_id,
                        agent_id,
                        run_id,
                        &state,
                    )
                    .await
                    .map_err(internal_error)?;
                }
                return Ok(AgentReplyProgress::Pending);
            }
            Err(error) => {
                workflow_trace(
                    "provider.error",
                    conversation_id,
                    message_id,
                    agent_id,
                    format!("run_id={} error={}", run_id, error.message()),
                );
                return Err(error);
            }
        };
        if let Some(reply) = apply_provider_response(
            client,
            conversation_id,
            lane_id,
            message_id,
            agent_id,
            run_id,
            &mut state,
            &response,
            direct_state_store,
        )
        .await?
        {
            return Ok(AgentReplyProgress::Completed(reply));
        }
    }
}

async fn apply_provider_response(
    client: &HostApiClient,
    conversation_id: &str,
    lane_id: Option<&str>,
    message_id: Option<&str>,
    agent_id: &str,
    run_id: &str,
    state: &mut AgentReplyLoopState,
    response: &JsonValue,
    direct_state_store: Option<&SqliteRuntimeStore>,
) -> Result<Option<String>, HostApiError> {
    state.next_iteration += 1;
    let text = response
        .get("text")
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned);
    let reasoning = read_provider_reasoning(response);
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
        if let Some(store) = direct_state_store {
            let _ = delete_direct_reply_state(store, run_id).await;
        }
        return Ok(Some(
            text.ok_or_else(|| internal_error("provider returned empty text"))?,
        ));
    }
    if let Some(progress_text) = text.as_deref() {
        let normalized_progress = progress_text.trim();
        if !normalized_progress.is_empty()
            && state.last_process_text.as_deref() != Some(normalized_progress)
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
            state.last_process_text = Some(normalized_progress.to_string());
        }
    }
    state.messages.push(serde_json::json!({
        "role": "assistant",
        "content": text.unwrap_or_default(),
        "tool_calls": tool_calls.iter().map(|call| serde_json::json!({
            "id": call.id,
            "name": call.name,
            "arguments": call.arguments,
        })).collect::<Vec<_>>(),
    }));
    state.pending_tool_calls = tool_calls;
    if let Some(store) = direct_state_store {
        persist_direct_reply_loop_state(
            store,
            conversation_id,
            lane_id,
            message_id,
            agent_id,
            run_id,
            state,
        )
        .await
        .map_err(internal_error)?;
    }
    Ok(None)
}

async fn execute_builtin_tool(
    client: &HostApiClient,
    agent_id: &str,
    conversation_id: &str,
    _lane_id: Option<&str>,
    message_id: Option<&str>,
    run_id: &str,
    tool_call: &AgentToolCall,
) -> Result<OperationPerformResponse, HostApiError> {
    match tool_call.name.as_str() {
        "command_exec" => {
            client
                .execute_operation_deferred(
                    "command.exec",
                    agent_id,
                    conversation_id,
                    _lane_id,
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
    _conversation_id: &str,
    agent_id: &str,
    run_id: &str,
) -> Result<Option<JsonValue>, String> {
    let Some(detail) = load_run_detail_json(store, run_id).await? else {
        return Ok(None);
    };
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
    let operator_profile = resolve_operator_profile(runtime_paths);
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
            "workspace_root": "/workspace",
            "artifacts_root": "/artifacts",
            "temp_root": "/tmp",
            "file_access": agent_file_access_context(),
        },
        "operator_profile": {
            "display_name": operator_profile.display_name,
            "time_zone": operator_profile.time_zone,
            "operating_system": operator_profile.operating_system,
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
        "tools": build_agent_tool_contexts(&extensions_runtime),
    })
}

fn agent_file_access_context() -> JsonValue {
    serde_json::json!({
        "default_root": "/workspace",
        "roots": [
            { "id": "workspace", "path": "/workspace", "mode": "read_write" },
            { "id": "artifacts", "path": "/artifacts", "mode": "read_write" },
            { "id": "temp", "path": "/tmp", "mode": "read_write" },
        ],
    })
}

fn build_agent_tool_contexts(snapshot: &JsonValue) -> Vec<JsonValue> {
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
            "capability_id": "command.exec",
            "label": "命令执行",
            "summary": "执行系统命令，并返回 stdout、stderr 和退出码；需要读写文件或访问网络时，也通过命令完成。工作目录优先使用 /workspace、/artifacts、/tmp 这些文件访问根。",
            "kind": "builtin",
            "contract": "command.exec",
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
        Err(error) => {
            let error_details = error.details().clone();
            let error_status = if error.code() == ErrorCode::Forbidden
                && (error_details.get("decision").and_then(JsonValue::as_str) == Some("ask")
                    || error.is_permission_approval())
            {
                "blocked"
            } else {
                "failed"
            };
            ToolMessageEnvelope {
                kind: "ennoia.tool_call",
                tool_call_id: tool_call.id.clone(),
                tool_name: tool_call.name.trim().replace('_', "."),
                status: error_status.to_string(),
                arguments: tool_call.arguments.clone(),
                result: None,
                error: Some(ToolMessageError {
                    code: error_code_string(error.code()),
                    message: error.message().to_string(),
                    details: error_details,
                }),
            }
        }
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

fn build_agent_runtime_prompt(
    agent: &AgentConfig,
    run_id: &str,
    operator_profile: &OperatorProfileSnapshot,
) -> String {
    let mut sections = Vec::new();
    if !agent.system_prompt.trim().is_empty() {
        sections.push(agent.system_prompt.trim().to_string());
    }
    sections.push(format!(
        "你当前运行在 Ennoia 会话系统中。\nagent_id：{}\nagent_name：{}\noperator_name：{}\noperator_time_zone：{}\noperator_operating_system：{}\nrun_id：{}\nworkspace_root：/workspace\nartifacts_root：/artifacts\ntemp_root：/tmp\nfile_access_roots：/workspace, /artifacts, /tmp\n文件访问只接受这些虚拟根及其子路径；除非用户明确需要，否则不要主动复述内部路径或实现细节。直接回答用户，不要伪装成“系统已接收”或“正在处理中”。",
        agent.id,
        agent.display_name,
        operator_profile.display_name,
        operator_profile
            .time_zone
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown"),
        operator_profile
            .operating_system
            .as_deref()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or("unknown"),
        if run_id.trim().is_empty() { "unknown" } else { run_id },
    ));
    sections.push(
        "系统会额外提供结构化上下文。按字段理解并使用，不要向用户原样复述 JSON。".to_string(),
    );
    sections.push("如果用户明确询问你有哪些工具或能力，优先依据上下文里的 tools 字段回答，使用 label 和 summary 做自然语言说明；不要把原始 JSON 对象或 `[object Object]` 直接输出给用户。".to_string());
    sections.push("当用户要求你与操作系统交互时，优先使用 tools 字段里提供的命令执行能力完成任务，例如读取文件、写入文件、运行脚本或发起网络请求。只有在工具调用被权限系统拒绝或需要审批时，才解释阻塞原因。遇到普通的命令执行错误时，按实际错误原因说明。".to_string());
    sections.join("\n\n")
}

fn build_agent_builtin_tool_specs(_agent: &AgentConfig) -> Vec<ToolSpec> {
    vec![
        ToolSpec {
            name: "command_exec".to_string(),
            description: "执行系统命令；需要读写文件、发起网络请求或运行脚本时，都通过命令完成。command 只填可执行程序名，参数拆到 args 里。".to_string(),
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
    ]
}

fn normalize_conversation_messages_for_provider(
    conversation_messages: &JsonValue,
    agent_id: &str,
    operator_display_name: &str,
) -> Vec<JsonValue> {
    let mut messages = conversation_messages
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|message| message_visible_to_agent(message, agent_id))
        .map(|message| normalize_operator_message_sender(message, operator_display_name))
        .rev()
        .take(24)
        .collect::<Vec<_>>();
    messages.reverse();
    messages
}

fn visible_recent_messages(
    runtime_paths: &Arc<RuntimePaths>,
    conversation_messages: &JsonValue,
    agent_id: &str,
) -> Vec<String> {
    let operator_display_name = resolve_operator_profile(runtime_paths).display_name;
    normalize_conversation_messages_for_provider(
        conversation_messages,
        agent_id,
        &operator_display_name,
    )
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

fn normalize_operator_message_sender(
    mut message: JsonValue,
    operator_display_name: &str,
) -> JsonValue {
    let role = message
        .get("role")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let sender = message
        .get("sender")
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    if matches!(role, "operator" | "user")
        && (sender.trim().is_empty()
            || sender.eq_ignore_ascii_case("operator")
            || sender.eq_ignore_ascii_case("user"))
    {
        if let Some(object) = message.as_object_mut() {
            object.insert(
                "sender".to_string(),
                JsonValue::String(operator_display_name.to_string()),
            );
        }
    }
    message
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

fn operation_response_output_or_content(response: OperationPerformResponse) -> JsonValue {
    response.operation.output.unwrap_or(response.content)
}

fn read_server_config(runtime_paths: &Arc<RuntimePaths>) -> Result<ServerConfig, String> {
    let contents = fs::read_to_string(runtime_paths.server_config_file())
        .map_err(|error| format!("read server config failed: {error}"))?;
    toml::from_str::<ServerConfig>(&contents)
        .map(|config| config.normalize())
        .map_err(|error| format!("parse server config failed: {error}"))
}

fn resolve_operator_profile(runtime_paths: &Arc<RuntimePaths>) -> OperatorProfileSnapshot {
    let profile = load_runtime_profile(runtime_paths);
    let display_name = profile
        .as_ref()
        .map(|item| item.display_name.trim())
        .filter(|value| !value.is_empty())
        .unwrap_or("Operator")
        .to_string();
    OperatorProfileSnapshot {
        display_name,
        time_zone: profile
            .as_ref()
            .and_then(|item| non_empty_string(item.time_zone.as_str())),
        operating_system: profile
            .as_ref()
            .and_then(|item| item.operating_system.as_deref())
            .and_then(non_empty_string),
    }
}

fn load_runtime_profile(runtime_paths: &Arc<RuntimePaths>) -> Option<RuntimeProfile> {
    let contents = fs::read_to_string(runtime_paths.profile_config_file()).ok()?;
    toml::from_str::<RuntimeProfile>(&contents).ok()
}

fn non_empty_string(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
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
        agent.working_dir = "/workspace".to_string();
        agent.artifacts_dir = "/artifacts".to_string();
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
        || message.starts_with("文件访问路径已拦截")
    {
        return message.to_string();
    }

    let normalized = message.to_lowercase();
    let heading = if normalized.contains("file access only accepts configured virtual roots")
        || normalized.contains("path cannot escape the selected file access root")
        || normalized.contains("path must stay inside the selected file access root")
    {
        "文件访问路径已拦截"
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

fn operation_error_message(operation: &OperationRecord) -> String {
    operation
        .error
        .as_ref()
        .and_then(|value| value.get("message"))
        .and_then(JsonValue::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("operation failed")
        .to_string()
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
    let code = parse_host_error_code(&error.code);
    HostApiError {
        body: ApiErrorBody {
            code,
            message: error.message,
            request_id: None,
            trace_id: None,
            details: error.details.unwrap_or(JsonValue::Null),
            retryable: matches!(
                code,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::sqlite_store::initialize_workflow_schema;
    use sqlx::SqlitePool;

    #[test]
    fn host_response_error_preserves_structured_details() {
        let error = host_response_error(ennoia_kernel::ExtensionRpcResponse::failure_with_details(
            "forbidden",
            "approval required: action=command.exec, approval_id=apr-1",
            Some(serde_json::json!({
                "decision": "ask",
                "approval_id": "apr-1",
            })),
        ));

        assert_eq!(error.code(), ErrorCode::Forbidden);
        assert_eq!(
            error.details(),
            &serde_json::json!({
                "decision": "ask",
                "approval_id": "apr-1",
            })
        );
    }

    #[test]
    fn approval_required_tool_call_without_details_is_blocked() {
        let tool_call = AgentToolCall {
            id: "call-1".to_string(),
            name: "command_exec".to_string(),
            arguments: serde_json::json!({
                "command": "cmd",
                "args": ["mkdir", "C:/tmp/demo"],
            }),
        };
        let error = HostApiError {
            body: ApiErrorBody {
                code: ErrorCode::Forbidden,
                message: "approval required: action=command.exec, approval_id=apr-1".to_string(),
                request_id: None,
                trace_id: None,
                details: JsonValue::Null,
                retryable: false,
            },
        };

        let serialized =
            serialize_tool_message_envelope(&tool_call, Err(&error)).expect("serialize envelope");
        let parsed: JsonValue = serde_json::from_str(&serialized).expect("parse envelope");
        assert_eq!(
            parsed.get("status").and_then(JsonValue::as_str),
            Some("blocked")
        );
    }

    #[tokio::test]
    async fn missing_direct_run_resume_does_not_error() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        initialize_workflow_schema(&pool)
            .await
            .expect("initialize workflow schema");
        let store = SqliteRuntimeStore::new(pool);

        let detail = load_run_response_for_agent(&store, "conv-demo", "b", "direct-b-msg-demo")
            .await
            .expect("load run response");

        assert!(detail.is_none());
    }

    #[tokio::test]
    async fn claim_conversation_message_receipt_is_atomic_for_recent_running_entry() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        initialize_workflow_schema(&pool)
            .await
            .expect("initialize workflow schema");
        let store = SqliteRuntimeStore::new(pool);

        let first =
            claim_conversation_message_receipt(&store, "conv-1", Some("msg-1"), "b", 60_000)
                .await
                .expect("first claim");
        let second =
            claim_conversation_message_receipt(&store, "conv-1", Some("msg-1"), "b", 60_000)
                .await
                .expect("second claim");

        assert!(first);
        assert!(!second);

        let row = sqlx::query(
            "SELECT status FROM conversation_message_receipts
             WHERE conversation_id = ?1 AND message_id = ?2 AND agent_id = ?3",
        )
        .bind("conv-1")
        .bind("msg-1")
        .bind("b")
        .fetch_one(store.pool())
        .await
        .expect("load receipt");
        assert_eq!(row.get::<String, _>("status"), "running");
    }

    #[tokio::test]
    async fn claim_conversation_message_receipt_reclaims_stale_running_entry() {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("connect sqlite memory");
        initialize_workflow_schema(&pool)
            .await
            .expect("initialize workflow schema");
        let store = SqliteRuntimeStore::new(pool);

        sqlx::query(
            "INSERT INTO conversation_message_receipts
             (conversation_id, message_id, agent_id, status, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'running', ?4, ?4)",
        )
        .bind("conv-1")
        .bind("msg-1")
        .bind("b")
        .bind("2026-01-01T00:00:00Z")
        .execute(store.pool())
        .await
        .expect("insert stale receipt");

        let claimed = claim_conversation_message_receipt(&store, "conv-1", Some("msg-1"), "b", 1)
            .await
            .expect("reclaim claim");

        assert!(claimed);
    }
}
