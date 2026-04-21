use super::*;

pub(super) async fn bootstrap_status(State(state): State<AppState>) -> Json<BootstrapState> {
    Json((**state.system_config.bootstrap.load()).clone())
}

pub(super) async fn bootstrap_setup(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<BootstrapSetupPayload>,
) -> ApiResult<BootstrapSetupResponse> {
    let current = (**state.system_config.bootstrap.load()).clone();
    if current.is_initialized {
        return Err(scoped(
            ApiError::conflict("bootstrap already completed"),
            &request,
        ));
    }

    let now = now_iso();
    let locale = ensure_supported_locale(
        &state,
        &request,
        payload
            .locale
            .unwrap_or_else(|| state.ui_config.default_locale.clone()),
    )?;
    let theme_id = ensure_supported_theme_id(
        &state,
        &request,
        payload
            .theme_id
            .unwrap_or_else(|| state.ui_config.default_theme.clone()),
    )?;
    let profile = WorkspaceProfile {
        id: "workspace".to_string(),
        display_name: payload
            .display_name
            .unwrap_or_else(|| "Operator".to_string()),
        locale,
        time_zone: payload
            .time_zone
            .unwrap_or_else(|| "Asia/Shanghai".to_string()),
        default_space_id: payload
            .default_space_id
            .or_else(|| state.spaces.first().map(|space| space.id.clone())),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let saved_profile = db::update_workspace_profile(&state.pool, &profile)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let preference = UiPreference {
        locale: Some(saved_profile.locale.clone()),
        theme_id: Some(theme_id),
        time_zone: Some(saved_profile.time_zone.clone()),
        date_style: payload.date_style,
        density: payload.density,
        motion: payload.motion,
        version: 1,
        updated_at: now.clone(),
    };
    let saved_preference = db::upsert_instance_ui_preference(&state.pool, &preference)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let bootstrap = BootstrapState {
        is_initialized: true,
        initialized_at: Some(now.clone()),
    };
    let boot_value = serde_json::to_value(&bootstrap)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    state
        .system_config
        .store
        .put(CONFIG_KEY_BOOTSTRAP, &boot_value, Some("bootstrap"))
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let _ = state.system_config.apply(CONFIG_KEY_BOOTSTRAP, &boot_value);

    Ok(Json(BootstrapSetupResponse {
        bootstrap,
        profile: saved_profile,
        preference: to_preference_record(saved_preference),
    }))
}

pub(super) async fn runtime_profile(
    State(state): State<AppState>,
) -> Json<Option<WorkspaceProfile>> {
    Json(
        db::get_workspace_profile(&state.pool)
            .await
            .unwrap_or_default(),
    )
}

pub(super) async fn runtime_profile_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<WorkspaceProfilePayload>,
) -> ApiResult<WorkspaceProfile> {
    let current = db::get_workspace_profile(&state.pool)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let now = now_iso();
    let requested_locale = payload
        .locale
        .map(|locale| ensure_supported_locale(&state, &request, locale))
        .transpose()?;
    let profile = WorkspaceProfile {
        id: current
            .as_ref()
            .map(|profile| profile.id.clone())
            .unwrap_or_else(|| "workspace".to_string()),
        display_name: payload
            .display_name
            .or_else(|| current.as_ref().map(|profile| profile.display_name.clone()))
            .unwrap_or_else(|| "Operator".to_string()),
        locale: requested_locale
            .or_else(|| current.as_ref().map(|profile| profile.locale.clone()))
            .unwrap_or_else(|| state.ui_config.default_locale.clone()),
        time_zone: payload
            .time_zone
            .or_else(|| current.as_ref().map(|profile| profile.time_zone.clone()))
            .unwrap_or_else(|| "Asia/Shanghai".to_string()),
        default_space_id: payload.default_space_id.or_else(|| {
            current
                .as_ref()
                .and_then(|profile| profile.default_space_id.clone())
        }),
        created_at: current
            .as_ref()
            .map(|profile| profile.created_at.clone())
            .unwrap_or_else(|| now.clone()),
        updated_at: now,
    };
    let saved = db::update_workspace_profile(&state.pool, &profile)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(saved))
}

pub(super) async fn runtime_preferences(
    State(state): State<AppState>,
) -> Json<Option<UiPreferenceRecord>> {
    Json(
        db::get_instance_ui_preference(&state.pool)
            .await
            .unwrap_or_default()
            .map(to_preference_record),
    )
}

pub(super) async fn runtime_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    let current = db::get_instance_ui_preference(&state.pool)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    validate_ui_preference_payload(&state, &request, &payload)?;
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = db::upsert_instance_ui_preference(&state.pool, &preference)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(to_preference_record(saved)))
}

pub(super) async fn runtime_app_config(State(state): State<AppState>) -> Json<AppConfig> {
    Json(
        read_app_config_from_disk(&state)
            .map(|config| normalize_app_config(&state.runtime_paths, config))
            .unwrap_or_else(|| state.app_config.clone()),
    )
}

pub(super) async fn runtime_app_config_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<AppConfig>,
) -> ApiResult<AppConfig> {
    let normalized = normalize_app_config(&state.runtime_paths, payload);
    if let Some(database_path) = normalized.database_url.strip_prefix("sqlite://") {
        if let Some(parent) = state
            .runtime_paths
            .expand_home_token(database_path)
            .parent()
            .map(|path| path.to_path_buf())
        {
            fs::create_dir_all(parent)
                .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
        }
    }
    let contents = toml::to_string_pretty(&normalized)
        .map_err(|error| scoped(ApiError::bad_request(error.to_string()), &request))?;
    fs::write(state.runtime_paths.app_config_file(), contents)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(normalized))
}

pub(super) fn read_app_config_from_disk(state: &AppState) -> Option<AppConfig> {
    let contents = fs::read_to_string(state.runtime_paths.app_config_file()).ok()?;
    toml::from_str(&contents).ok()
}

pub(super) async fn space_ui_preferences(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(space_id): Path<String>,
) -> ApiResult<Option<UiPreferenceRecord>> {
    let row = db::get_space_ui_preference(&state.pool, &space_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(row.map(to_preference_record)))
}

pub(super) async fn space_ui_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(space_id): Path<String>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    let current = db::get_space_ui_preference(&state.pool, &space_id)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    validate_ui_preference_payload(&state, &request, &payload)?;
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = db::upsert_space_ui_preference(&state.pool, &space_id, &preference)
        .await
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(to_preference_record(saved)))
}
