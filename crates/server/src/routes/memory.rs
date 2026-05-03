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
