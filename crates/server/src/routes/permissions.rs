use ennoia_kernel::{AgentPermissionPolicy, PermissionApprovalRecord, PermissionEventRecord};

use crate::agent_permissions::{
    ApprovalResolutionPayload, PermissionApprovalsQuery, PermissionEventsQuery,
    PermissionPolicySummary,
};

use super::interfaces::spawn_approved_conversation_agent_reply;
use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct PermissionEventsQueryPayload {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    action: Option<String>,
    #[serde(default)]
    decision: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

#[derive(Debug, Deserialize)]
pub(super) struct PermissionApprovalsQueryPayload {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    status: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
}

pub(super) async fn agent_policy_detail(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(agent_id): Path<String>,
) -> ApiResult<AgentPermissionPolicy> {
    state
        .agent_permissions
        .load_policy(&agent_id)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn agent_policy_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(agent_id): Path<String>,
    Json(payload): Json<AgentPermissionPolicy>,
) -> ApiResult<AgentPermissionPolicy> {
    state
        .agent_permissions
        .save_policy(&agent_id, &payload)
        .map_err(|error| {
            let api_error = if error.kind() == std::io::ErrorKind::NotFound {
                ApiError::not_found(error.to_string())
            } else {
                ApiError::internal(error.to_string())
            };
            scoped(api_error, &request)
        })?;
    Ok(Json(payload))
}

pub(super) async fn permission_policy_summaries(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<Vec<PermissionPolicySummary>> {
    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let mut rows = agents
        .into_iter()
        .map(|agent| state.agent_permissions.policy_summary(&agent.id))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    rows.sort_by(|left, right| left.agent_id.cmp(&right.agent_id));
    Ok(Json(rows))
}

pub(super) async fn permission_events(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<PermissionEventsQueryPayload>,
) -> ApiResult<Vec<PermissionEventRecord>> {
    state
        .agent_permissions
        .list_events(&PermissionEventsQuery {
            agent_id: query.agent_id,
            action: query.action,
            decision: query.decision,
            limit: query.limit.unwrap_or(100),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn permission_approvals(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<PermissionApprovalsQueryPayload>,
) -> ApiResult<Vec<PermissionApprovalRecord>> {
    state
        .agent_permissions
        .list_approvals(&PermissionApprovalsQuery {
            agent_id: query.agent_id,
            conversation_id: query.conversation_id,
            status: query.status,
            limit: query.limit.unwrap_or(100),
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn permission_approval_resolve(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(approval_id): Path<String>,
    Json(payload): Json<ApprovalResolutionPayload>,
) -> ApiResult<PermissionApprovalRecord> {
    let approval = state
        .agent_permissions
        .resolve_approval(&approval_id, &payload.resolution)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| {
            scoped(
                ApiError::not_found("permission approval not found"),
                &request,
            )
        })?;
    if approval.status == "approved" {
        spawn_approved_conversation_agent_reply(state.clone(), request.clone(), approval.clone());
    }
    Ok(Json(approval))
}
