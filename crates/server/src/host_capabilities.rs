use std::io;

use ennoia_contract::{ApiError, ErrorCode};
use ennoia_extension_host::{HostCapabilityDispatcher, ResolvedExtensionSnapshot};
use ennoia_kernel::{
    ExtensionHostCapabilityRequest, ExtensionRpcResponse, ExtensionStateEntry,
    HookEventPublishRequest, RuntimeOperationRequest,
};
use ennoia_logs::{next_request_id, next_span_id, next_trace_id, RequestContext};

use crate::app::AppState;
use crate::routes::{actions, extensions};
use crate::runtime_bridge;

#[derive(Clone)]
pub(crate) struct ServerHostCapabilityDispatcher {
    state: AppState,
    runtime: tokio::runtime::Handle,
}

impl ServerHostCapabilityDispatcher {
    pub(crate) fn new(state: AppState, runtime: tokio::runtime::Handle) -> Self {
        Self { state, runtime }
    }
}

impl HostCapabilityDispatcher for ServerHostCapabilityDispatcher {
    fn dispatch(
        &self,
        extension: &ResolvedExtensionSnapshot,
        request: ExtensionHostCapabilityRequest,
    ) -> io::Result<ExtensionRpcResponse> {
        let request_context = synthetic_request_context(extension);
        Ok(match request {
            ExtensionHostCapabilityRequest::ExtensionsRuntimeSnapshot => {
                ExtensionRpcResponse::success(
                    serde_json::to_value(self.state.extensions.snapshot())
                        .map_err(io::Error::other)?,
                )
            }
            ExtensionHostCapabilityRequest::ActionDispatch {
                action,
                params,
                context,
            } => map_api_result(self.runtime.block_on(async {
                actions::dispatch_action_value_with_context(
                    &self.state,
                    &request_context,
                    &action,
                    params,
                    context,
                )
                .await
            })),
            ExtensionHostCapabilityRequest::ProviderInvoke {
                provider_kind,
                method,
                payload,
            } => map_api_result(self.runtime.block_on(async {
                extensions::invoke_provider_json_with_request(
                    &self.state,
                    &request_context,
                    &provider_kind,
                    &method,
                    payload,
                )
                .await
            })),
            ExtensionHostCapabilityRequest::RuntimeOperation { operation, payload } => {
                dispatch_runtime_operation(
                    &self.state,
                    &self.runtime,
                    &request_context,
                    &operation,
                    payload,
                )
            }
            ExtensionHostCapabilityRequest::HookEventPublish { payload } => {
                publish_hook_event(&self.state, &request_context, payload)
            }
            ExtensionHostCapabilityRequest::OperationPerform { payload } => {
                map_api_result(self.runtime.block_on(async {
                    runtime_bridge::perform_operation(
                        &self.state,
                        &request_context,
                        &extension.id,
                        payload,
                    )
                    .await
                    .and_then(|response| {
                        serde_json::to_value(response)
                            .map_err(|error| ApiError::internal(error.to_string()))
                    })
                }))
            }
            ExtensionHostCapabilityRequest::ExtensionStateGet { query } => self
                .state
                .extension_runtime_store
                .get_state(&query)
                .map(|entry| match entry {
                    Some(entry) => {
                        ExtensionRpcResponse::success(serde_json::to_value(entry).unwrap())
                    }
                    None => ExtensionRpcResponse::failure("not_found", "extension state not found"),
                })?,
            ExtensionHostCapabilityRequest::ExtensionStatePut { payload } => self
                .state
                .extension_runtime_store
                .put_state(&payload)
                .map(extension_state_success)?,
            ExtensionHostCapabilityRequest::ExtensionStateDelete { query } => self
                .state
                .extension_runtime_store
                .delete_state(&query)
                .map(|deleted| {
                    ExtensionRpcResponse::success(serde_json::json!({ "deleted": deleted }))
                })?,
            ExtensionHostCapabilityRequest::ExtensionRecordAppend { payload } => self
                .state
                .extension_runtime_store
                .append_record(&payload)
                .map(|entry| ExtensionRpcResponse::success(serde_json::to_value(entry).unwrap()))?,
            ExtensionHostCapabilityRequest::ExtensionRecordUpdate { payload } => self
                .state
                .extension_runtime_store
                .update_record(&payload)
                .map(|entry| match entry {
                    Some(entry) => {
                        ExtensionRpcResponse::success(serde_json::to_value(entry).unwrap())
                    }
                    None => {
                        ExtensionRpcResponse::failure("not_found", "extension record not found")
                    }
                })?,
            ExtensionHostCapabilityRequest::ExtensionRecordClose { record_id } => self
                .state
                .extension_runtime_store
                .close_record(&record_id)
                .map(|entry| match entry {
                    Some(entry) => {
                        ExtensionRpcResponse::success(serde_json::to_value(entry).unwrap())
                    }
                    None => {
                        ExtensionRpcResponse::failure("not_found", "extension record not found")
                    }
                })?,
        })
    }
}

fn dispatch_runtime_operation(
    state: &AppState,
    runtime: &tokio::runtime::Handle,
    request_context: &RequestContext,
    operation: &str,
    payload: RuntimeOperationRequest,
) -> ExtensionRpcResponse {
    map_api_result(runtime.block_on(async {
        runtime_bridge::execute_runtime_operation(state, request_context, operation, payload)
            .await
            .map(|result| result.content)
    }))
}

fn publish_hook_event(
    state: &AppState,
    request_context: &RequestContext,
    payload: HookEventPublishRequest,
) -> ExtensionRpcResponse {
    actions::dispatch_hook_event(
        state,
        request_context,
        &payload.event,
        &payload.resource_kind,
        &payload.resource_id,
        payload.payload,
    );
    ExtensionRpcResponse::success(serde_json::json!({ "published": true }))
}

fn extension_state_success(entry: ExtensionStateEntry) -> ExtensionRpcResponse {
    ExtensionRpcResponse::success(serde_json::to_value(entry).unwrap())
}

fn map_api_result(result: Result<serde_json::Value, ApiError>) -> ExtensionRpcResponse {
    match result {
        Ok(data) => ExtensionRpcResponse::success(data),
        Err(error) => ExtensionRpcResponse::failure_with_details(
            error_code_string(error.code()),
            error.message(),
            match error.details() {
                serde_json::Value::Null => None,
                details => Some(details.clone()),
            },
        ),
    }
}

fn synthetic_request_context(extension: &ResolvedExtensionSnapshot) -> RequestContext {
    RequestContext {
        request_id: next_request_id(),
        trace_id: next_trace_id(),
        span_id: next_span_id(),
        parent_span_id: None,
        sampled: true,
        source: format!("extension_host.{}", extension.id),
    }
}

fn error_code_string(code: ErrorCode) -> &'static str {
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
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_api_result_preserves_error_details() {
        let response = map_api_result(Err(ApiError::forbidden("approval required").with_details(
            serde_json::json!({
                "decision": "ask",
                "approval_id": "apr-1",
            }),
        )));

        let error = response.error.expect("error");
        assert_eq!(error.code, "forbidden");
        assert_eq!(
            error.details,
            Some(serde_json::json!({
                "decision": "ask",
                "approval_id": "apr-1",
            }))
        );
    }

    #[test]
    fn host_capability_request_serializes_hook_event_publish() {
        let request = ExtensionHostCapabilityRequest::HookEventPublish {
            payload: HookEventPublishRequest {
                event: ennoia_kernel::HOOK_EVENT_ARTIFACT_CREATED.to_string(),
                resource_kind: "artifact".to_string(),
                resource_id: "art-1".to_string(),
                payload: serde_json::json!({ "artifact_id": "art-1" }),
            },
        };

        assert_eq!(
            serde_json::to_value(request).unwrap(),
            serde_json::json!({
                "kind": "hook_event_publish",
                "payload": {
                    "event": "artifact.created",
                    "resource_kind": "artifact",
                    "resource_id": "art-1",
                    "payload": { "artifact_id": "art-1" },
                }
            })
        );
    }
}
