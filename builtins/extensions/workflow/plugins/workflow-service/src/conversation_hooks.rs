use std::fs;
use std::sync::Arc;

use ennoia_contract::behavior::{BehaviorRunRequest, BehaviorSourceRef, BehaviorTrigger};
use ennoia_contract::{ApiErrorBody, ErrorCode};
use ennoia_kernel::{
    AgentConfig, AgentDocument, ExtensionRpcRequest, HookDispatchResponse, HookEventEnvelope,
    ModelEndpointConfig, OwnerKind, OwnerRef, PermissionApprovalRecord, RunContext, RunStage,
    ServerConfig,
};
use ennoia_paths::RuntimePaths;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::Row;

use crate::pipeline::{run_behavior, WorkflowRuntime};
use crate::runtime::{RuntimeStore, SqliteRuntimeStore};

#[derive(Debug)]
struct HostApiClient {
    client: reqwest::Client,
    base_url: String,
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
        let host = normalize_loopback_host(&server_config.host);
        Ok(Self {
            client: reqwest::Client::builder()
                .build()
                .map_err(|error| error.to_string())?,
            base_url: format!("http://{}:{}", host, server_config.port),
        })
    }

    async fn dispatch_action(
        &self,
        action: &str,
        params: JsonValue,
        context: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.post_json(
            &format!("/api/actions/{action}"),
            &serde_json::json!({
                "params": params,
                "context": context,
            }),
        )
        .await
    }

    async fn provider_generate(
        &self,
        provider_kind: &str,
        payload: JsonValue,
        context: JsonValue,
    ) -> Result<JsonValue, HostApiError> {
        self.post_json(
            &format!("/api/extensions/providers/{provider_kind}/generate"),
            &ExtensionRpcRequest {
                params: payload,
                context,
            },
        )
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
        self.post_json(
            &format!("/api/runtime/operations/{operation}"),
            &serde_json::json!({
                "agent_id": agent_id,
                "conversation_id": conversation_id,
                "run_id": run_id,
                "message_id": message_id,
                "arguments": arguments,
            }),
        )
        .await
    }

    async fn get_json(&self, path: &str) -> Result<JsonValue, HostApiError> {
        let response = self
            .client
            .get(format!("{}{}", self.base_url, path))
            .send()
            .await
            .map_err(internal_http_error)?;
        self.read_response(response).await
    }

    async fn post_json<T: Serialize + ?Sized>(
        &self,
        path: &str,
        body: &T,
    ) -> Result<JsonValue, HostApiError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .json(body)
            .send()
            .await
            .map_err(internal_http_error)?;
        self.read_response(response).await
    }

    async fn read_response(&self, response: reqwest::Response) -> Result<JsonValue, HostApiError> {
        let status = response.status();
        let bytes = response.bytes().await.map_err(internal_http_error)?;
        if status.is_success() {
            serde_json::from_slice::<JsonValue>(&bytes).map_err(|error| HostApiError {
                body: ApiErrorBody {
                    code: ErrorCode::Internal,
                    message: format!("parse host response failed: {error}"),
                    request_id: None,
                    trace_id: None,
                    details: JsonValue::Null,
                    retryable: true,
                },
            })
        } else {
            let body =
                serde_json::from_slice::<ApiErrorBody>(&bytes).map_err(|error| HostApiError {
                    body: ApiErrorBody {
                        code: ErrorCode::Internal,
                        message: format!("parse host error failed: {error}"),
                        request_id: None,
                        trace_id: None,
                        details: JsonValue::String(String::from_utf8_lossy(&bytes).to_string()),
                        retryable: false,
                    },
                })?;
            Err(HostApiError { body })
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
    let lane_id = payload_string_field(payload, &["lane", "id"])
        .or_else(|| payload_string_field(payload, &["message", "lane_id"]));
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
        let run_response = if let Some(run_id) = workflow_resume_run_id.as_deref() {
            load_run_response_for_agent(store, &conversation_id, agent_id, run_id)
                .await?
                .ok_or_else(|| format!("workflow run '{run_id}' not found for agent"))?
        } else if is_workflow_execution_confirmation(&body) {
            let Some(run_response) = load_latest_pending_run_for_agent(
                store,
                &conversation_id,
                message_id.as_deref(),
                agent_id,
            )
            .await?
            else {
                append_agent_conversation_reply(
                    client,
                    &conversation_id,
                    lane_id.as_deref(),
                    message_id.as_deref(),
                    agent_id,
                    None,
                    &build_workflow_no_pending_reply(),
                )
                .await?;
                continue;
            };
            run_response
        } else {
            let run_response = create_workflow_run_response(
                runtime,
                &owner,
                &conversation_id,
                lane_id.as_deref(),
                message_id.as_deref(),
                &body,
                agent_id,
                memory_context,
                metadata,
            )
            .await?;
            remember_workflow_run(
                client,
                &owner,
                &conversation_id,
                lane_id.as_deref(),
                &body,
                agent_id,
                message_id.as_deref(),
                run_response_id(&run_response),
                &run_response,
            )
            .await;
            append_agent_conversation_reply(
                client,
                &conversation_id,
                lane_id.as_deref(),
                message_id.as_deref(),
                agent_id,
                run_response_id(&run_response),
                &build_workflow_plan_reply(&run_response),
            )
            .await?;
            continue;
        };
        let reply_body = match generate_real_agent_reply(
            client,
            &runtime.runtime_paths,
            &agents,
            &model_endpoints,
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
            Err(error) if error.is_permission_approval() => continue,
            Err(error) => error.to_string(),
        };
        append_agent_conversation_reply(
            client,
            &conversation_id,
            lane_id.as_deref(),
            message_id.as_deref(),
            agent_id,
            run_response_id(&run_response),
            &reply_body,
        )
        .await?;
    }

    Ok(())
}

async fn generate_real_agent_reply(
    client: &HostApiClient,
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
    let run_id = run_response_id(run_response)
        .unwrap_or_default()
        .to_string();
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
        &run_id,
    )
    .await;
    let instructions = ProviderInstructions {
        base: build_agent_runtime_prompt(agent, &run_id),
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

    for _ in 0..6 {
        let response = client
            .provider_generate(
                &model_endpoint.kind,
                serde_json::json!({
                    "model_endpoint": model_endpoint_runtime_request_config(model_endpoint),
                    "model": model_id,
                    "instructions": instructions,
                    "system_prompt": build_agent_runtime_prompt(agent, &run_id),
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
                    Some(&run_id),
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
                &run_id,
                &tool_call,
            )
            .await
            {
                Ok(result) => {
                    let body = append_tool_result_message(
                        client,
                        conversation_id,
                        lane_id,
                        message_id,
                        agent_id,
                        &tool_call,
                        Ok(result),
                    )
                    .await?;
                    messages.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tool_call.id,
                        "content": body,
                    }));
                }
                Err(error) => {
                    let body = append_tool_result_message(
                        client,
                        conversation_id,
                        lane_id,
                        message_id,
                        agent_id,
                        &tool_call,
                        Err(&error),
                    )
                    .await?;
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
    tool_call: &AgentToolCall,
    outcome: Result<JsonValue, &HostApiError>,
) -> Result<String, HostApiError> {
    let body = serialize_tool_message_envelope(tool_call, outcome)
        .map_err(|error| internal_error(format!("serialize tool message failed: {error}")))?;
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
    Ok(body)
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
        if let Some(detail) =
            load_run_response_for_agent(store, conversation_id, agent_id, &run.id).await?
        {
            return Ok(Some(detail));
        }
    }
    Ok(None)
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
    serde_json::to_value(response)
        .map_err(|error| format!("serialize workflow run response failed: {error}"))
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
) -> JsonValue {
    let extensions_runtime = client
        .get_json("/api/extensions/runtime")
        .await
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

fn model_endpoint_runtime_request_config(model_endpoint: &ModelEndpointConfig) -> JsonValue {
    serde_json::json!({
        "id": model_endpoint.id,
        "kind": model_endpoint.kind,
        "base_url": model_endpoint.base_url,
        "api_key": model_endpoint.api_key,
        "api_key_env": model_endpoint.api_key_env,
        "default_model": model_endpoint.default_model,
        "available_models": model_endpoint.available_models,
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
        RunStage::Completed | RunStage::Failed | RunStage::Cancelled | RunStage::Blocked
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
            | "确认执行"
            | "继续吧"
            | "开始吧"
            | "可以执行"
            | "继续处理"
            | "开始处理"
    )
}

fn build_workflow_plan_reply(run_response: &JsonValue) -> String {
    let goal = run_response
        .get("run")
        .and_then(|item| item.get("goal"))
        .and_then(JsonValue::as_str)
        .unwrap_or_default();
    let mut lines = vec![
        "我先把执行计划整理好了，当前还没有真正开始动手。".to_string(),
        if goal.trim().is_empty() {
            "下面是准备执行的计划：".to_string()
        } else {
            format!("围绕“{}”，我打算这样推进：", goal.trim())
        },
    ];
    if let Some(tasks) = run_response.get("tasks").and_then(JsonValue::as_array) {
        let task_titles = tasks
            .iter()
            .filter_map(|task| task.get("title").and_then(JsonValue::as_str))
            .take(6)
            .enumerate()
            .map(|(index, title)| format!("{}. {}", index + 1, title.trim()))
            .collect::<Vec<_>>();
        if !task_titles.is_empty() {
            lines.extend(task_titles);
        }
    }
    if lines.len() <= 2 {
        lines.push("1. 梳理目标与约束".to_string());
        lines.push("2. 拆分关键步骤".to_string());
        lines.push("3. 确认交付位置与执行方式".to_string());
    }
    lines.push("如果这个计划没问题，回复“继续执行”或“开始执行”，我就按这个计划继续。".to_string());
    lines.join("\n")
}

fn build_workflow_no_pending_reply() -> String {
    "当前没有可继续执行的编排任务。先发一条任务请求，我会先给出编排结果。".to_string()
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

fn normalize_loopback_host(host: &str) -> String {
    match host.trim() {
        "" | "0.0.0.0" | "::" | "[::]" => "127.0.0.1".to_string(),
        value => value.to_string(),
    }
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

fn internal_http_error(error: reqwest::Error) -> HostApiError {
    internal_error(format!("host request failed: {error}"))
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
