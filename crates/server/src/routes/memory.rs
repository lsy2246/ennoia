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
    Json(
        state
            .extensions
            .snapshot()
            .memories
            .into_iter()
            .map(|item| {
                let extension_id = item.extension_id.clone();
                let status = state
                    .extensions
                    .get(&extension_id)
                    .map(|extension| extension.health);
                MemoryProviderRecord {
                    id: item.memory.id.clone(),
                    source_kind: "extension".to_string(),
                    extension_id: Some(extension_id.clone()),
                    version: item.memory.version,
                    interfaces: item.memory.interfaces,
                    entry: item.memory.entry,
                    enabled: status.as_ref().is_some_and(|health| {
                        !matches!(health, ennoia_kernel::ExtensionHealth::Stopped)
                    }),
                    healthy: status.is_some_and(|health| {
                        matches!(health, ennoia_kernel::ExtensionHealth::Ready)
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
    resolve_active_memory_record(&state, &request).map(Json)
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
    let record = resolve_active_memory_record(&state, &request)?;
    dispatch_memory_request(&state, &request, &record.id, &path, method, body).await
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
    state
        .extensions
        .snapshot()
        .memories
        .into_iter()
        .map(|item| {
            let extension_id = item.extension_id.clone();
            let status = state
                .extensions
                .get(&extension_id)
                .map(|extension| extension.health);
            MemoryProviderRecord {
                id: item.memory.id.clone(),
                source_kind: "extension".to_string(),
                extension_id: Some(extension_id.clone()),
                version: item.memory.version,
                interfaces: item.memory.interfaces,
                entry: item.memory.entry,
                enabled: status.as_ref().is_some_and(|health| {
                    !matches!(health, ennoia_kernel::ExtensionHealth::Stopped)
                }),
                healthy: status
                    .is_some_and(|health| matches!(health, ennoia_kernel::ExtensionHealth::Ready)),
            }
        })
        .collect()
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

fn resolve_active_memory_record(
    state: &AppState,
    request: &RequestContext,
) -> Result<MemoryProviderRecord, ApiError> {
    let records = list_memory_records(state)
        .into_iter()
        .filter(|item| item.enabled)
        .collect::<Vec<_>>();

    match records.as_slice() {
        [only] => Ok(only.clone()),
        [] => Err(scoped(ApiError::not_found("active memory not found"), request)),
        _ => Err(scoped(
            ApiError::conflict(
                "multiple memory implementations found; use explicit memory ids or interface bindings",
            ),
            request,
        )),
    }
}
