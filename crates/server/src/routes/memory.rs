use super::interfaces::dispatch_interface_json;
use super::*;

#[derive(Debug, Clone, Serialize)]
pub(super) struct MemoryExtensionRecord {
    extension_id: String,
    version: String,
    interfaces: Vec<String>,
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
            let interfaces = extension
                .interfaces
                .iter()
                .filter(|item| item.key.starts_with("memory."))
                .map(|item| item.key.clone())
                .collect::<Vec<_>>();
            if interfaces.is_empty() {
                return None;
            }
            Some(MemoryExtensionRecord {
                extension_id: extension.id,
                version: extension.version,
                interfaces,
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
    dispatch_interface_json(&state, &request, "memory.workspace", JsonValue::Null).await
}

pub(super) async fn memory_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "memory.list", JsonValue::Null).await
}

pub(super) async fn memory_episodes_list(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(
        &state,
        &request,
        "memory.episodes_list",
        serde_json::to_value(query).unwrap_or(JsonValue::Null),
    )
    .await
}

pub(super) async fn memory_remember(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "memory.remember", payload).await
}

pub(super) async fn memory_recall(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "memory.recall", payload).await
}

pub(super) async fn memory_review(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "memory.review", payload).await
}

pub(super) async fn memory_assemble_context(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<JsonValue>,
) -> ApiResult<JsonValue> {
    dispatch_interface_json(&state, &request, "memory.assemble_context", payload).await
}
