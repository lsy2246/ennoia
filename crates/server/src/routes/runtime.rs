use super::*;
use ennoia_kernel::apply_server_log_env_overrides;

pub(super) async fn bootstrap_status(State(state): State<AppState>) -> Json<BootstrapState> {
    Json(
        read_server_config_from_disk(&state)
            .map(|config| config.bootstrap)
            .unwrap_or_else(|| state.server_config.bootstrap.clone()),
    )
}

pub(super) async fn bootstrap_setup(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<BootstrapSetupPayload>,
) -> ApiResult<BootstrapSetupResponse> {
    let current = read_server_config_from_disk(&state)
        .map(|config| config.bootstrap)
        .unwrap_or_else(|| state.server_config.bootstrap.clone());
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
    let profile = RuntimeProfile {
        id: "runtime".to_string(),
        display_name: payload
            .display_name
            .unwrap_or_else(|| state.ui_config.default_display_name.clone()),
        locale,
        time_zone: payload
            .time_zone
            .unwrap_or_else(|| state.ui_config.default_time_zone.clone()),
        default_space_id: payload
            .default_space_id
            .or_else(|| state.spaces.first().map(|space| space.id.clone())),
        created_at: now.clone(),
        updated_at: now.clone(),
    };
    let saved_profile = persist_runtime_profile(&state, &profile)
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
    let saved_preference = persist_instance_ui_preference(&state, &preference)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    let bootstrap = BootstrapState {
        is_initialized: true,
        initialized_at: Some(now.clone()),
    };
    let mut server_config =
        read_server_config_from_disk(&state).unwrap_or_else(|| state.server_config.clone());
    server_config.bootstrap = bootstrap.clone();
    persist_server_config(&state, &server_config)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;

    Ok(Json(BootstrapSetupResponse {
        bootstrap,
        profile: saved_profile,
        preference: saved_preference,
    }))
}

pub(super) async fn runtime_profile(State(state): State<AppState>) -> Json<Option<RuntimeProfile>> {
    Json(read_runtime_profile_from_disk(&state))
}

pub(super) async fn runtime_profile_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<RuntimeProfilePayload>,
) -> ApiResult<RuntimeProfile> {
    let current = read_runtime_profile_from_disk(&state);
    let now = now_iso();
    let requested_locale = payload
        .locale
        .map(|locale| ensure_supported_locale(&state, &request, locale))
        .transpose()?;
    let profile = RuntimeProfile {
        id: current
            .as_ref()
            .map(|profile| profile.id.clone())
            .unwrap_or_else(|| "runtime".to_string()),
        display_name: payload
            .display_name
            .or_else(|| current.as_ref().map(|profile| profile.display_name.clone()))
            .unwrap_or_else(|| state.ui_config.default_display_name.clone()),
        locale: requested_locale
            .or_else(|| current.as_ref().map(|profile| profile.locale.clone()))
            .unwrap_or_else(|| state.ui_config.default_locale.clone()),
        time_zone: payload
            .time_zone
            .or_else(|| current.as_ref().map(|profile| profile.time_zone.clone()))
            .unwrap_or_else(|| state.ui_config.default_time_zone.clone()),
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
    let saved = persist_runtime_profile(&state, &profile)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(saved))
}

pub(super) async fn runtime_preferences(
    State(state): State<AppState>,
) -> Json<Option<UiPreferenceRecord>> {
    Json(read_instance_ui_preference_from_disk(&state))
}

pub(super) async fn runtime_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    let current = read_instance_ui_preference_from_disk(&state);
    validate_ui_preference_payload(&state, &request, &payload)?;
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = persist_instance_ui_preference(&state, &preference)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(saved))
}

pub(super) async fn runtime_server_config(State(state): State<AppState>) -> Json<ServerConfig> {
    let mut config =
        read_server_config_from_disk(&state).unwrap_or_else(|| state.server_config.clone());
    config = config.normalize();
    apply_server_log_env_overrides(&mut config.logging);
    Json(config)
}

pub(super) async fn runtime_server_config_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ServerConfig>,
) -> ApiResult<ServerConfig> {
    let payload = payload.normalize();
    persist_server_config(&state, &payload)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(payload))
}

fn read_server_config_from_disk(state: &AppState) -> Option<ServerConfig> {
    let contents = fs::read_to_string(state.runtime_paths.server_config_file()).ok()?;
    toml::from_str(&contents).ok()
}

fn persist_server_config(state: &AppState, config: &ServerConfig) -> std::io::Result<()> {
    if let Some(parent) = state.runtime_paths.server_config_file().parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(config).map_err(std::io::Error::other)?;
    fs::write(state.runtime_paths.server_config_file(), contents)
}

pub(super) async fn space_ui_preferences(
    State(state): State<AppState>,
    Extension(_request): Extension<RequestContext>,
    Path(space_id): Path<String>,
) -> ApiResult<Option<UiPreferenceRecord>> {
    Ok(Json(read_space_ui_preference_from_disk(&state, &space_id)))
}

pub(super) async fn space_ui_preferences_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(space_id): Path<String>,
    Json(payload): Json<UiPreferencePayload>,
) -> ApiResult<UiPreferenceRecord> {
    let current = read_space_ui_preference_from_disk(&state, &space_id);
    validate_ui_preference_payload(&state, &request, &payload)?;
    let preference = merge_ui_preference(current.as_ref().map(|row| &row.preference), payload);
    let saved = persist_space_ui_preference(&state, &space_id, &preference)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    Ok(Json(saved))
}

pub(super) fn read_runtime_profile_from_disk(state: &AppState) -> Option<RuntimeProfile> {
    read_toml_file(state.runtime_paths.profile_config_file())
}

pub(super) fn read_instance_ui_preference_from_disk(
    state: &AppState,
) -> Option<UiPreferenceRecord> {
    let preference = read_toml_file(state.runtime_paths.instance_preference_file())?;
    Some(UiPreferenceRecord {
        subject_id: "instance".to_string(),
        preference,
    })
}

pub(super) fn read_space_ui_preference_from_disk(
    state: &AppState,
    space_id: &str,
) -> Option<UiPreferenceRecord> {
    let preference = read_toml_file(state.runtime_paths.space_preference_file(space_id))?;
    Some(UiPreferenceRecord {
        subject_id: space_id.to_string(),
        preference,
    })
}

pub(super) fn list_space_ui_preferences_from_disk(state: &AppState) -> Vec<UiPreferenceRecord> {
    let Ok(entries) = fs::read_dir(state.runtime_paths.space_preferences_dir()) else {
        return Vec::new();
    };

    let mut records = entries
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let path = entry.path();
            let space_id = path.file_stem()?.to_string_lossy().to_string();
            let preference = read_toml_file::<UiPreference>(path)?;
            Some(UiPreferenceRecord {
                subject_id: space_id,
                preference,
            })
        })
        .collect::<Vec<_>>();
    records.sort_by(|left, right| left.subject_id.cmp(&right.subject_id));
    records
}

pub(super) fn ui_preference_version_from_disk(state: &AppState) -> u64 {
    let instance = read_instance_ui_preference_from_disk(state)
        .map(|item| item.preference.version)
        .unwrap_or(0);
    let spaces = list_space_ui_preferences_from_disk(state)
        .into_iter()
        .map(|item| item.preference.version)
        .max()
        .unwrap_or(0);
    instance.max(spaces)
}

fn persist_runtime_profile(
    state: &AppState,
    profile: &RuntimeProfile,
) -> std::io::Result<RuntimeProfile> {
    write_toml_file(state.runtime_paths.profile_config_file(), profile)?;
    Ok(profile.clone())
}

fn persist_instance_ui_preference(
    state: &AppState,
    preference: &UiPreference,
) -> std::io::Result<UiPreferenceRecord> {
    write_toml_file(state.runtime_paths.instance_preference_file(), preference)?;
    Ok(UiPreferenceRecord {
        subject_id: "instance".to_string(),
        preference: preference.clone(),
    })
}

fn persist_space_ui_preference(
    state: &AppState,
    space_id: &str,
    preference: &UiPreference,
) -> std::io::Result<UiPreferenceRecord> {
    write_toml_file(
        state.runtime_paths.space_preference_file(space_id),
        preference,
    )?;
    Ok(UiPreferenceRecord {
        subject_id: space_id.to_string(),
        preference: preference.clone(),
    })
}

fn read_toml_file<T>(path: impl AsRef<std::path::Path>) -> Option<T>
where
    T: serde::de::DeserializeOwned,
{
    let contents = fs::read_to_string(path).ok()?;
    toml::from_str(&contents).ok()
}

fn write_toml_file<T>(path: impl AsRef<std::path::Path>, value: &T) -> std::io::Result<()>
where
    T: serde::Serialize,
{
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let contents = toml::to_string_pretty(value).map_err(std::io::Error::other)?;
    fs::write(path, contents)
}
