use super::*;

pub(super) async fn memories_list(State(state): State<AppState>) -> Json<Vec<MemoryRecord>> {
    Json(
        state
            .memory_store
            .list_memories(100)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn memories_create(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RememberPayload>,
) -> ApiResult<RememberReceipt> {
    let owner = OwnerRef {
        kind: owner_kind_from(&payload.owner_kind),
        id: payload.owner_id,
    };
    let remember = RememberRequest {
        owner,
        namespace: payload.namespace,
        memory_kind: MemoryKind::from_str(&payload.memory_kind),
        stability: Stability::from_str(&payload.stability),
        title: payload.title,
        content: payload.content,
        summary: payload.summary,
        confidence: payload.confidence,
        importance: payload.importance,
        valid_from: None,
        valid_to: None,
        sources: payload.sources,
        tags: payload.tags,
        entities: payload.entities,
    };
    state
        .memory_store
        .remember(remember)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

pub(super) async fn memories_recall(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RecallPayload>,
) -> ApiResult<RecallResult> {
    let mode = match payload.mode.as_deref().unwrap_or("namespace") {
        "fts" => RecallMode::Fts,
        "hybrid" => RecallMode::Hybrid,
        _ => RecallMode::Namespace,
    };
    let query = RecallQuery {
        owner: OwnerRef {
            kind: owner_kind_from(&payload.owner_kind),
            id: payload.owner_id,
        },
        conversation_id: payload.conversation_id,
        run_id: payload.run_id,
        query_text: payload.query_text,
        namespace_prefix: payload.namespace_prefix,
        memory_kind: payload.memory_kind.as_deref().map(MemoryKind::from_str),
        mode,
        limit: payload.limit.unwrap_or(20),
    };
    state
        .memory_store
        .recall(query)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}

pub(super) async fn memories_review(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ReviewPayload>,
) -> ApiResult<ReviewReceipt> {
    let action_kind = match payload.action.as_str() {
        "approve" => ReviewActionKind::Approve,
        "reject" => ReviewActionKind::Reject,
        "supersede" => ReviewActionKind::Supersede,
        "retire" => ReviewActionKind::Retire,
        _ => return Err(scoped(ApiError::bad_request("unknown action"), &request)),
    };
    let action = ReviewAction {
        target_memory_id: payload.target_memory_id,
        reviewer: payload.reviewer,
        action: action_kind,
        notes: payload.notes,
    };
    state
        .memory_store
        .review(action)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))
}
