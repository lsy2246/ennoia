use super::*;
use crate::app::{delete_agent_config, write_agent_config};
use crate::pipeline::{
    invoke_provider_method, provider_runtime_request_config, resolve_provider_entry_path,
};

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
    write_agent_config(&state.runtime_paths, &payload)
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
    write_agent_config(&state.runtime_paths, &payload)
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
    let deleted = delete_agent_config(&state.runtime_paths, &agent_id)
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

    provider_models_response(&state, &provider).map(Json)
}

pub(super) async fn provider_discover_models(
    State(state): State<AppState>,
    Json(payload): Json<ProviderConfig>,
) -> Result<Json<ProviderModelsResponse>, ApiError> {
    provider_models_response(&state, &payload).map(Json)
}

fn provider_models_response(
    state: &AppState,
    provider: &ProviderConfig,
) -> Result<ProviderModelsResponse, ApiError> {
    let contribution = resolve_provider_contribution(&state, &provider.kind)?;
    let mut models = provider.available_models.clone();
    let mut source = if models.is_empty() {
        "manual".to_string()
    } else {
        "configured".to_string()
    };
    let mut manual_allowed = provider.model_discovery.manual_allowed;
    let mut generation_options = Vec::new();

    if let Some(contribution) = contribution {
        manual_allowed = contribution.provider.manual_model;
        generation_options = contribution.provider.generation_options.clone();
        if contribution.provider.model_discovery
            && contribution
                .provider
                .interfaces
                .iter()
                .any(|name| name == "models")
        {
            let entry = resolve_provider_entry_path(&contribution)
                .map_err(|error| ApiError::internal(error.to_string()))?;
            let request_payload = serde_json::json!({
                "method": "list_models",
                "params": {
                    "provider": provider_runtime_request_config(&provider),
                }
            });
            let response = invoke_provider_method(&entry, &request_payload, &provider)
                .map_err(ApiError::internal)?;
            let extension_models = parse_provider_models_from_response(&response)?;
            if !extension_models.is_empty() {
                models = extension_models;
                source = "extension".to_string();
            }
        }
    }

    Ok(ProviderModelsResponse {
        provider_id: provider.id.clone(),
        source,
        models,
        manual_allowed,
        generation_options,
    })
}

pub(super) async fn provider_create(
    State(state): State<AppState>,
    Json(payload): Json<ProviderConfig>,
) -> Result<Json<ProviderConfig>, ApiError> {
    validate_provider_payload(&state, &payload)?;
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
    validate_provider_payload(&state, &payload)?;
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

fn validate_provider_payload(state: &AppState, payload: &ProviderConfig) -> Result<(), ApiError> {
    let _ = resolve_provider_contribution(state, &payload.kind)?;
    if payload.enabled && payload.default_model.trim().is_empty() {
        return Err(ApiError::bad_request(
            "启用上游渠道前必须配置默认模型；无法发现模型时使用手动输入。",
        ));
    }
    let mut seen = HashSet::new();
    for model in &payload.available_models {
        let model_id = model.id.trim();
        if model_id.is_empty() {
            return Err(ApiError::bad_request("模型列表里不能有空模型 ID。"));
        }
        if !seen.insert(model_id.to_string()) {
            return Err(ApiError::bad_request(format!(
                "模型列表里存在重复模型 ID: '{model_id}'。"
            )));
        }
    }
    if !payload.default_model.trim().is_empty()
        && !payload
            .available_models
            .iter()
            .any(|model| model.id.trim() == payload.default_model.trim())
    {
        return Err(ApiError::bad_request("默认模型必须存在于模型列表里。"));
    }
    Ok(())
}

fn resolve_provider_contribution(
    state: &AppState,
    kind: &str,
) -> Result<Option<ennoia_extension_host::RegisteredProviderContribution>, ApiError> {
    let normalized = kind.trim();
    let matches = state
        .extensions
        .snapshot()
        .providers
        .into_iter()
        .filter(|item| item.provider.kind == normalized || item.provider.id == normalized)
        .collect::<Vec<_>>();

    match matches.len() {
        0 => Err(ApiError::bad_request(format!(
            "接口类型 '{normalized}' 当前没有可用实现扩展。"
        ))),
        1 => Ok(matches.into_iter().next()),
        _ => Err(ApiError::bad_request(format!(
            "接口类型 '{normalized}' 对应多个实现扩展，当前不允许创建渠道。"
        ))),
    }
}

fn parse_provider_models_from_response(
    response: &JsonValue,
) -> Result<Vec<ennoia_kernel::ProviderModelDescriptor>, ApiError> {
    let Some(items) = response
        .get("result")
        .and_then(|item| item.get("models"))
        .and_then(JsonValue::as_array)
    else {
        return Ok(Vec::new());
    };

    items.iter().map(parse_provider_model_descriptor).collect()
}

fn parse_provider_model_descriptor(
    value: &JsonValue,
) -> Result<ennoia_kernel::ProviderModelDescriptor, ApiError> {
    serde_json::from_value::<ennoia_kernel::ProviderModelDescriptor>(value.clone()).map_err(
        |error| {
            ApiError::internal(format!(
                "provider returned invalid model descriptor: {error}"
            ))
        },
    )
}
