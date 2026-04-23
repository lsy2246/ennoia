use std::fs;

use axum::body::Bytes;
use axum::http::Method;
use axum::response::{IntoResponse, Response};
use ennoia_contract::memory::MemoryStatusResponse;

use super::behavior::dispatch_worker_capability_request;
use super::*;
use crate::system_log::{SystemLogWrite, SYSTEM_LOG_COMPONENT_MEMORY};

#[derive(Debug, Clone, Serialize)]
pub(super) struct MemoryProviderRecord {
    id: String,
    source_kind: String,
    #[serde(default)]
    extension_id: Option<String>,
    version: String,
    interfaces: Vec<String>,
    #[serde(default)]
    entry: Option<String>,
    enabled: bool,
    healthy: bool,
}

pub(super) async fn extension_memories(
    State(state): State<AppState>,
) -> Json<Vec<MemoryProviderRecord>> {
    let config = current_memory_config(&state);
    Json(
        state
            .extensions
            .snapshot()
            .memories
            .into_iter()
            .map(|item| {
                let memory_id = item.memory.id.clone();
                let extension_id = item.extension_id.clone();
                MemoryProviderRecord {
                    id: memory_id.clone(),
                    source_kind: "extension".to_string(),
                    extension_id: Some(extension_id.clone()),
                    version: item.memory.version,
                    interfaces: item.memory.interfaces,
                    entry: item.memory.entry,
                    enabled: config
                        .enabled
                        .iter()
                        .any(|id| id == &extension_id || id == &memory_id),
                    healthy: state
                        .extensions
                        .get(&extension_id)
                        .is_some_and(|extension| {
                            matches!(extension.health, ennoia_kernel::ExtensionHealth::Ready)
                        }),
                }
            })
            .collect(),
    )
}

pub(super) async fn memories(State(state): State<AppState>) -> Json<Vec<MemoryProviderRecord>> {
    Json(list_memory_records(&state))
}

pub(super) async fn active_memory(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
) -> ApiResult<MemoryProviderRecord> {
    let config = current_memory_config(&state);
    resolve_memory_record(&state, &config.preferred_read, &request).map(Json)
}

pub(super) async fn memory_status(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(memory_id): Path<String>,
) -> ApiResult<MemoryStatusResponse> {
    let record = resolve_memory_record(&state, &memory_id, &request)?;
    Ok(Json(MemoryStatusResponse {
        memory_id: record.id,
        source_kind: record.source_kind,
        healthy: record.healthy,
        enabled: record.enabled,
        interfaces: record.interfaces,
    }))
}

pub(super) async fn memory_api_proxy(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path((memory_id, path)): Path<(String, String)>,
    method: Method,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    dispatch_memory_request(&state, &request, &memory_id, &path, method, body).await
}

pub(super) async fn active_memory_api_proxy(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(path): Path<String>,
    method: Method,
    body: Bytes,
) -> Result<impl IntoResponse, ApiError> {
    let config = current_memory_config(&state);
    dispatch_memory_request(
        &state,
        &request,
        &config.preferred_read,
        &path,
        method,
        body,
    )
    .await
}

async fn dispatch_memory_request(
    state: &AppState,
    request: &RequestContext,
    memory_id: &str,
    path: &str,
    method: Method,
    body: Bytes,
) -> Result<Response, ApiError> {
    let record = resolve_memory_record(state, memory_id, request)?;
    if !record.enabled {
        return Err(scoped(
            ApiError::forbidden(format!("memory '{memory_id}' is disabled")),
            request,
        ));
    }
    if record.source_kind == "builtin" {
        return dispatch_builtin_journal(state, path, method, body).await;
    }
    let extension_id = record.extension_id.as_deref().ok_or_else(|| {
        scoped(
            ApiError::not_found(format!("memory '{memory_id}' has no extension")),
            request,
        )
    })?;
    dispatch_worker_capability_request(
        state,
        request,
        extension_id,
        record.entry.as_deref(),
        path,
        method,
        body,
        SYSTEM_LOG_COMPONENT_MEMORY,
    )
    .await
    .map(IntoResponse::into_response)
}

fn list_memory_records(state: &AppState) -> Vec<MemoryProviderRecord> {
    let config = current_memory_config(state);
    let journal_enabled = config.enabled.iter().any(|id| id == "journal");
    let mut records = vec![MemoryProviderRecord {
        id: "journal".to_string(),
        source_kind: "builtin".to_string(),
        extension_id: None,
        version: "1".to_string(),
        interfaces: vec![
            "workspace".to_string(),
            "conversations".to_string(),
            "messages".to_string(),
            "lanes".to_string(),
            "handoffs".to_string(),
        ],
        entry: None,
        enabled: journal_enabled,
        healthy: state.server_config.journal.enabled,
    }];

    records.extend(
        state
            .extensions
            .snapshot()
            .memories
            .into_iter()
            .map(|item| {
                let memory_id = item.memory.id.clone();
                let extension_id = item.extension_id.clone();
                MemoryProviderRecord {
                    id: memory_id.clone(),
                    source_kind: "extension".to_string(),
                    extension_id: Some(extension_id.clone()),
                    version: item.memory.version,
                    interfaces: item.memory.interfaces,
                    entry: item.memory.entry,
                    enabled: config
                        .enabled
                        .iter()
                        .any(|id| id == &extension_id || id == &memory_id),
                    healthy: state
                        .extensions
                        .get(&extension_id)
                        .is_some_and(|extension| {
                            matches!(extension.health, ennoia_kernel::ExtensionHealth::Ready)
                        }),
                }
            }),
    );

    records
}

fn current_memory_config(state: &AppState) -> ennoia_kernel::MemoryConfig {
    fs::read_to_string(state.runtime_paths.memory_config_file())
        .ok()
        .and_then(|contents| toml::from_str(&contents).ok())
        .unwrap_or_else(|| state.memory_config.clone())
}

fn resolve_memory_record(
    state: &AppState,
    memory_id: &str,
    request: &RequestContext,
) -> Result<MemoryProviderRecord, ApiError> {
    list_memory_records(state)
        .into_iter()
        .find(|item| {
            item.id == memory_id
                || item
                    .extension_id
                    .as_deref()
                    .is_some_and(|extension_id| extension_id == memory_id)
        })
        .ok_or_else(|| {
            let _ = state.system_log.append(SystemLogWrite {
                event: "runtime.memory.resolve_failed".to_string(),
                level: "warn".to_string(),
                component: SYSTEM_LOG_COMPONENT_MEMORY.to_string(),
                source_kind: "memory".to_string(),
                source_id: Some(memory_id.to_string()),
                summary: "memory provider not found".to_string(),
                payload: serde_json::json!({ "memory_id": memory_id }),
                created_at: None,
            });
            scoped(
                ApiError::not_found(format!("memory '{memory_id}' not found")),
                request,
            )
        })
}

async fn dispatch_builtin_journal(
    state: &AppState,
    path: &str,
    method: Method,
    body: Bytes,
) -> Result<Response, ApiError> {
    let trimmed = path.trim_matches('/');
    let segments = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    match (method, segments.as_slice()) {
        (Method::GET, ["workspace"]) => journal_workspace(State(state.clone()))
            .await
            .map(IntoResponse::into_response),
        (Method::GET, ["conversations"]) => conversations_list(State(state.clone()))
            .await
            .map(IntoResponse::into_response),
        (Method::POST, ["conversations"]) => {
            let payload: CreateConversationPayload = serde_json::from_slice(&body)
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            conversations_create(State(state.clone()), Json(payload))
                .await
                .map(IntoResponse::into_response)
        }
        (Method::GET, ["conversations", conversation_id]) => {
            conversation_detail(State(state.clone()), Path((*conversation_id).to_string()))
                .await
                .map(IntoResponse::into_response)
        }
        (Method::DELETE, ["conversations", conversation_id]) => {
            conversation_delete(State(state.clone()), Path((*conversation_id).to_string()))
                .await
                .map(IntoResponse::into_response)
        }
        (Method::GET, ["conversations", conversation_id, "messages"]) => {
            conversation_messages(State(state.clone()), Path((*conversation_id).to_string()))
                .await
                .map(IntoResponse::into_response)
        }
        (Method::POST, ["conversations", conversation_id, "messages"]) => {
            let payload: ConversationMessagePayload = serde_json::from_slice(&body)
                .map_err(|error| ApiError::bad_request(error.to_string()))?;
            conversation_messages_create(
                State(state.clone()),
                Path((*conversation_id).to_string()),
                Json(payload),
            )
            .await
            .map(IntoResponse::into_response)
        }
        (Method::GET, ["conversations", conversation_id, "lanes"]) => {
            conversation_lanes(State(state.clone()), Path((*conversation_id).to_string()))
                .await
                .map(IntoResponse::into_response)
        }
        _ => Err(ApiError::not_found("journal path not found")),
    }
}
