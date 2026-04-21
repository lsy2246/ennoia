use super::*;

pub(super) async fn agents(State(state): State<AppState>) -> Json<Vec<AgentConfig>> {
    Json(load_agent_configs(&state.runtime_paths).unwrap_or_default())
}

pub(super) async fn agent_detail(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentConfig>, ApiError> {
    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    agents
        .into_iter()
        .find(|agent| agent.id == agent_id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("agent '{agent_id}' not found")))
}

pub(super) async fn agent_create(
    State(state): State<AppState>,
    Json(payload): Json<AgentConfig>,
) -> Result<Json<AgentConfig>, ApiError> {
    write_config_to_dir(
        state.runtime_paths.agents_config_dir(),
        &payload.id,
        &payload,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    agents
        .into_iter()
        .find(|agent| agent.id == payload.id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created agent"))
}

pub(super) async fn agent_update(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(mut payload): Json<AgentConfig>,
) -> Result<Json<AgentConfig>, ApiError> {
    payload.id = agent_id.clone();
    write_config_to_dir(state.runtime_paths.agents_config_dir(), &agent_id, &payload)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let agents = load_agent_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    agents
        .into_iter()
        .find(|agent| agent.id == agent_id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated agent"))
}

pub(super) async fn agent_delete(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_config_from_dir(state.runtime_paths.agents_config_dir(), &agent_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!("agent '{agent_id}' not found")))
    }
}

pub(super) async fn skills(State(state): State<AppState>) -> Json<Vec<SkillConfig>> {
    Json(load_skill_configs(&state.runtime_paths).unwrap_or_default())
}

pub(super) async fn skill_detail(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> Result<Json<SkillConfig>, ApiError> {
    let skills = load_skill_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    skills
        .into_iter()
        .find(|skill| skill.id == skill_id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("skill '{skill_id}' not found")))
}

pub(super) async fn skill_create(
    State(state): State<AppState>,
    Json(payload): Json<SkillConfig>,
) -> Result<Json<SkillConfig>, ApiError> {
    upsert_skill_package(&state.runtime_paths, &payload)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let skills = load_skill_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    skills
        .into_iter()
        .find(|skill| skill.id == payload.id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created skill"))
}

pub(super) async fn skill_update(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
    Json(mut payload): Json<SkillConfig>,
) -> Result<Json<SkillConfig>, ApiError> {
    payload.id = skill_id.clone();
    upsert_skill_package(&state.runtime_paths, &payload)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let skills = load_skill_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    skills
        .into_iter()
        .find(|skill| skill.id == skill_id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated skill"))
}

pub(super) async fn skill_delete(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_skill_package(&state.runtime_paths, &skill_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!("skill '{skill_id}' not found")))
    }
}

pub(super) async fn providers(State(state): State<AppState>) -> Json<Vec<ProviderConfig>> {
    Json(load_provider_configs(&state.runtime_paths).unwrap_or_default())
}

pub(super) async fn provider_detail(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ProviderConfig>, ApiError> {
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    providers
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("provider '{provider_id}' not found")))
}

pub(super) async fn provider_models(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<Json<ProviderModelsResponse>, ApiError> {
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let provider = providers
        .into_iter()
        .find(|item| item.id == provider_id)
        .ok_or_else(|| ApiError::not_found(format!("provider '{provider_id}' not found")))?;

    let contribution = state
        .extensions
        .snapshot()
        .providers
        .into_iter()
        .find(|item| {
            item.provider.kind == provider.kind
                && (provider.extension_id.is_empty()
                    || item.extension_id == provider.extension_id
                    || item.provider.extension_id.as_deref()
                        == Some(provider.extension_id.as_str()))
        });

    let mut models = provider.available_models.clone();
    let mut recommended_model = None;
    let mut source = if models.is_empty() {
        "manual".to_string()
    } else {
        "configured".to_string()
    };
    let mut manual_allowed = provider.model_discovery.manual_allowed;

    if let Some(contribution) = contribution {
        manual_allowed = contribution.provider.manual_model;
        if recommended_model.is_none() {
            recommended_model = contribution.provider.recommended_model.clone();
        }
        if models.is_empty() {
            if let Some(model) = contribution.provider.recommended_model {
                models.push(model.clone());
                recommended_model = Some(model);
            }
            source = if contribution.provider.model_discovery {
                "extension".to_string()
            } else {
                "extension-recommendation".to_string()
            };
        }
    }

    if recommended_model.is_none() && !provider.default_model.is_empty() {
        recommended_model = Some(provider.default_model.clone());
    }

    Ok(Json(ProviderModelsResponse {
        provider_id: provider.id,
        source,
        models,
        recommended_model,
        manual_allowed,
    }))
}

pub(super) async fn provider_create(
    State(state): State<AppState>,
    Json(payload): Json<ProviderConfig>,
) -> Result<Json<ProviderConfig>, ApiError> {
    validate_provider_payload(&payload)?;
    write_config_to_dir(
        state.runtime_paths.providers_config_dir(),
        &payload.id,
        &payload,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    providers
        .into_iter()
        .find(|provider| provider.id == payload.id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created provider"))
}

pub(super) async fn provider_update(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
    Json(mut payload): Json<ProviderConfig>,
) -> Result<Json<ProviderConfig>, ApiError> {
    payload.id = provider_id.clone();
    validate_provider_payload(&payload)?;
    write_config_to_dir(
        state.runtime_paths.providers_config_dir(),
        &provider_id,
        &payload,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let providers = load_provider_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    providers
        .into_iter()
        .find(|provider| provider.id == provider_id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated provider"))
}

pub(super) async fn provider_delete(
    State(state): State<AppState>,
    Path(provider_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_config_from_dir(state.runtime_paths.providers_config_dir(), &provider_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!(
            "provider '{provider_id}' not found"
        )))
    }
}

pub(super) async fn spaces(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::SpaceSpec>> {
    Json(state.spaces)
}

fn validate_provider_payload(payload: &ProviderConfig) -> Result<(), ApiError> {
    if payload.enabled && payload.default_model.trim().is_empty() {
        return Err(ApiError::bad_request(
            "启用上游渠道前必须配置默认模型；无法发现模型时使用手动输入。",
        ));
    }
    Ok(())
}
