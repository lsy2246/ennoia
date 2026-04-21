use super::*;

pub(super) async fn config_list(State(state): State<AppState>) -> Json<Vec<ConfigEntry>> {
    let raw = state.system_config.store.list().await.unwrap_or_default();
    Json(ensure_full_config_set(raw))
}

pub(super) async fn config_get(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(key): Path<String>,
) -> ApiResult<ConfigEntry> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err(scoped(
            ApiError::not_found(format!("unknown config key '{key}'")),
            &request,
        ));
    }
    state
        .system_config
        .store
        .get(&key)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .map(Json)
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("config '{key}' not initialized")),
                &request,
            )
        })
}

pub(super) async fn config_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(key): Path<String>,
    Json(payload): Json<ConfigPutPayload>,
) -> ApiResult<ConfigEntry> {
    if !ALL_CONFIG_KEYS.contains(&key.as_str()) {
        return Err(scoped(
            ApiError::not_found(format!("unknown config key '{key}'")),
            &request,
        ));
    }
    let entry = state
        .system_config
        .store
        .put(&key, &payload.payload, payload.updated_by.as_deref())
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let applied = state
        .system_config
        .apply(&key, &payload.payload)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    if !applied {
        return Err(scoped(
            ApiError::bad_request(format!("unsupported key '{key}'")),
            &request,
        ));
    }

    Ok(Json(entry))
}

pub(super) async fn config_history(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Json<Vec<ConfigChangeRecord>> {
    Json(
        state
            .system_config
            .store
            .history(&key, 50)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn config_snapshot(State(state): State<AppState>) -> Json<SystemConfig> {
    Json(state.system_config.snapshot())
}
