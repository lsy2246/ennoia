use super::actions::dispatch_action_json;
use super::*;

#[derive(Debug, Clone, Serialize)]
pub(super) struct MemoryExtensionRecord {
    extension_id: String,
    actions: Vec<String>,
    enabled: bool,
    healthy: bool,
}

pub(super) async fn extension_memories(
    State(state): State<AppState>,
) -> Json<Vec<MemoryExtensionRecord>> {
    let mut items = state
        .extensions
        .snapshot()
        .extensions
        .into_iter()
        .filter_map(|extension| {
            let actions = extension
                .actions
                .iter()
                .filter(|item| item.action.starts_with("memory."))
                .map(|item| item.action.clone())
                .collect::<Vec<_>>();
            if actions.is_empty() {
                return None;
            }
            Some(MemoryExtensionRecord {
                extension_id: extension.id,
                actions,
                enabled: !matches!(extension.health, ennoia_kernel::ExtensionHealth::Stopped),
                healthy: matches!(extension.health, ennoia_kernel::ExtensionHealth::Ready),
            })
        })
        .collect::<Vec<_>>();
    items.sort_by(|left, right| left.extension_id.cmp(&right.extension_id));
    Json(items)
}

pub(super) async fn memory_workspace(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.workspace.get", JsonValue::Null).await
}

pub(super) async fn memory_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.entry.list", JsonValue::Null).await
}

pub(super) async fn memory_episodes_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(
        &state,
        &request,
        "memory.episode.list",
        serde_json::to_value(query).unwrap_or(JsonValue::Null),
    )
    .await
}

pub(super) async fn memory_remember(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.ingest", payload).await
}

pub(super) async fn memory_recall(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.query", payload).await
}

pub(super) async fn memory_review(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.review", payload).await
}

pub(super) async fn memory_assemble_context(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_action_json(&state, &request, "memory.build_context", payload).await
}
