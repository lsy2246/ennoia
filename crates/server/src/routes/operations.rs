use ennoia_kernel::{OperationListQuery, OperationRecord};

use super::*;

#[derive(Debug, Deserialize)]
pub(super) struct OperationsQueryPayload {
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    run_id: Option<String>,
    #[serde(default)]
    message_id: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

pub(super) async fn operations_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<OperationsQueryPayload>,
) -> ApiResult<Vec<OperationRecord>> {
    state
        .operations
        .list_operations(&OperationListQuery {
            conversation_id: query.conversation_id,
            run_id: query.run_id,
            message_id: query.message_id,
            limit: query.limit,
        })
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}
