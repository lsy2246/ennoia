use crate::agent_permissions::PermissionApprovalsQuery;
use crate::app::{dispatch_extension_rpc, live_server_config};
use crate::realtime::RealtimeEvent;
use crate::routes::actions::dispatch_action_value;
use ennoia_kernel::ExtensionRpcRequest;

use super::*;
const WORKFLOW_EXTENSION_ID: &str = "workflow";

#[derive(Debug, Deserialize)]
pub(super) struct PermissionStreamQuery {
    agent_id: String,
}

#[derive(Debug, Deserialize)]
pub(super) struct WorkflowStreamQuery {
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) async fn conversation_stream(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(conversation_id): Path<String>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let state = state.clone();
    let request = request.clone();
    let stream = async_stream::stream! {
        let mut first = true;
        let mut last_event_seq = state.event_bus.latest_conversation_seq(&conversation_id).unwrap_or(0);
        let mut last_approval_seq = state.agent_permissions.latest_conversation_approval_seq(&conversation_id).unwrap_or(0);

        loop {
            if first {
                first = false;
            } else {
                tokio::time::sleep(Duration::from_millis(
                    live_server_config(&state).streams.conversation_poll_ms,
                )).await;
                let next_event_seq = state.event_bus.latest_conversation_seq(&conversation_id).unwrap_or(last_event_seq);
                let next_approval_seq = state.agent_permissions.latest_conversation_approval_seq(&conversation_id).unwrap_or(last_approval_seq);
                if next_event_seq == last_event_seq && next_approval_seq == last_approval_seq {
                    continue;
                }
                last_event_seq = next_event_seq;
                last_approval_seq = next_approval_seq;
            }

            match build_conversation_stream_snapshot(&state, &request, &conversation_id).await {
                Ok(payload) => {
                    yield Ok(Event::default().event("conversation.snapshot").data(payload));
                }
                Err(error) => {
                    let payload = serde_json::json!({
                        "message": error.to_string(),
                    });
                    yield Ok(Event::default().event("conversation.error").data(payload.to_string()));
                }
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub(super) async fn conversations_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut receiver = state.realtime.subscribe();
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("conversations.changed").data(stream_signal_payload("conversations")));
        loop {
            match receiver.recv().await {
                Ok(RealtimeEvent::ConversationsChanged)
                | Ok(RealtimeEvent::ConversationChanged { .. })
                | Ok(RealtimeEvent::AgentsChanged) => {
                    yield Ok(Event::default().event("conversations.changed").data(stream_signal_payload("conversations")));
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("conversations.changed").data(stream_signal_payload("conversations")));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub(super) async fn schedules_stream(
    State(state): State<AppState>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut receiver = state.realtime.subscribe();
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("schedules.changed").data(stream_signal_payload("schedules")));
        loop {
            match receiver.recv().await {
                Ok(RealtimeEvent::SchedulesChanged) => {
                    yield Ok(Event::default().event("schedules.changed").data(stream_signal_payload("schedules")));
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("schedules.changed").data(stream_signal_payload("schedules")));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub(super) async fn permissions_stream(
    State(state): State<AppState>,
    Query(query): Query<PermissionStreamQuery>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let agent_id = query.agent_id;
    let mut receiver = state.realtime.subscribe();
    let stream = async_stream::stream! {
        yield Ok(Event::default().event("permissions.changed").data(stream_signal_payload("permissions")));
        loop {
            match receiver.recv().await {
                Ok(RealtimeEvent::PermissionAgentChanged { agent_id: changed }) if changed == agent_id => {
                    yield Ok(Event::default().event("permissions.changed").data(stream_signal_payload("permissions")));
                }
                Ok(_) => {}
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                    yield Ok(Event::default().event("permissions.changed").data(stream_signal_payload("permissions")));
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

pub(super) async fn workflow_stream(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<WorkflowStreamQuery>,
) -> Sse<impl futures_core::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let state = state.clone();
    let request = request.clone();
    let stream = async_stream::stream! {
        let mut first = true;
        let mut last_payload = String::new();

        loop {
            if !first {
                tokio::time::sleep(Duration::from_millis(
                    live_server_config(&state).streams.workflow_poll_ms,
                )).await;
            }
            first = false;

            match build_workflow_stream_snapshot(&state, &request, &query).await {
                Ok(payload) => {
                    if payload != last_payload {
                        last_payload = payload.clone();
                        yield Ok(Event::default().event("workflow.snapshot").data(payload));
                    }
                }
                Err(error) => {
                    let payload = serde_json::json!({
                        "message": error.to_string(),
                    });
                    yield Ok(Event::default().event("workflow.error").data(payload.to_string()));
                }
            }
        }
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}

async fn build_conversation_stream_snapshot(
    state: &AppState,
    request: &RequestContext,
    conversation_id: &str,
) -> Result<String, ApiError> {
    let detail = dispatch_action_value(
        state,
        request,
        "conversation.get",
        serde_json::json!({
            "conversation_id": conversation_id,
        }),
    )
    .await?;
    let approvals = state
        .agent_permissions
        .list_approvals(&PermissionApprovalsQuery {
            conversation_id: Some(conversation_id.to_string()),
            limit: 80,
            ..PermissionApprovalsQuery::default()
        })
        .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;
    Ok(serde_json::json!({
        "detail": detail,
        "approvals": approvals,
    })
    .to_string())
}

async fn build_workflow_stream_snapshot(
    state: &AppState,
    request: &RequestContext,
    query: &WorkflowStreamQuery,
) -> Result<String, ApiError> {
    let workspace = workflow_rpc_json(state, request, "workspace", serde_json::json!({})).await?;
    let runs = workflow_rpc_json(
        state,
        request,
        "workflow/runs/list-by-conversation",
        serde_json::json!({
            "conversation_id": normalize_optional_string(query.conversation_id.clone()),
            "stage": normalize_optional_string(query.stage.clone()),
            "q": normalize_optional_string(query.q.clone()),
            "limit": query.limit.unwrap_or(120),
        }),
    )
    .await?;
    let detail = if let Some(run_id) = normalize_optional_string(query.run_id.clone()) {
        Some(
            workflow_rpc_json(
                state,
                request,
                "workflow/runs/get",
                serde_json::json!({ "run_id": run_id }),
            )
            .await?,
        )
    } else {
        None
    };

    Ok(serde_json::json!({
        "workspace": workspace,
        "runs": runs,
        "detail": detail,
    })
    .to_string())
}

async fn workflow_rpc_json(
    state: &AppState,
    request: &RequestContext,
    method: &str,
    params: JsonValue,
) -> Result<JsonValue, ApiError> {
    let trace = request.child_trace("workflow_stream_rpc");
    let response = dispatch_extension_rpc(
        state,
        WORKFLOW_EXTENSION_ID,
        method,
        ExtensionRpcRequest {
            params,
            context: serde_json::json!({
                "trace": {
                    "request_id": trace.request_id,
                    "trace_id": trace.trace_id,
                    "span_id": trace.span_id,
                    "parent_span_id": trace.parent_span_id,
                    "sampled": trace.sampled,
                    "source": trace.source,
                    "traceparent": trace.to_traceparent(),
                }
            }),
        },
    )
    .await
    .map_err(|error| scoped(ApiError::internal(error.to_string()), request))?;

    if response.ok {
        return Ok(response.data);
    }

    let message = response
        .error
        .map(|error| format!("{}: {}", error.code, error.message))
        .unwrap_or_else(|| format!("workflow rpc '{method}' failed"));
    Err(scoped(ApiError::internal(message), request))
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

fn stream_signal_payload(topic: &str) -> String {
    serde_json::json!({
        "topic": topic,
        "at": chrono::Utc::now().to_rfc3339(),
    })
    .to_string()
}
