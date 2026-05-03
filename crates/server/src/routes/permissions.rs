use ennoia_kernel::{PermissionApprovalRecord, PermissionEventRecord};

use crate::agent_permissions::{
    ApprovalResolutionPayload, PermissionApprovalsQuery, PermissionEventsQuery,
    PermissionGrantRecord, PermissionGrantsQuery, PermissionPolicySummary,
};
use crate::pipeline::queue_permission_approval_resume;

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

#[derive(Debug, Deserialize)]
pub(super) struct PermissionGrantsQueryPayload {
    #[serde(default)]
    agent_id: Option<String>,
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    limit: Option<usize>,
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

pub(super) async fn permission_grants(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<PermissionGrantsQueryPayload>,
) -> ApiResult<Vec<PermissionGrantRecord>> {
    state
        .agent_permissions
        .list_grants(&PermissionGrantsQuery {
            agent_id: query.agent_id,
            conversation_id: query.conversation_id,
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
        queue_permission_approval_resume(state.clone(), request.clone(), approval.clone());
    }
    Ok(Json(approval))
}

pub(super) async fn permission_grant_revoke(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(grant_id): Path<String>,
) -> ApiResult<PermissionGrantRecord> {
    state
        .agent_permissions
        .revoke_grant(&grant_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| scoped(ApiError::not_found("permission grant not found"), &request))
}
