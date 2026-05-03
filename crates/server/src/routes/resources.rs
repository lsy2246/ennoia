use super::*;
use crate::app::{
    delete_agent_config, load_agent_document, load_agent_documents, normalize_agent_document,
    write_agent_document,
};
use crate::logs_store::{LogEntryWrite, LOGS_COMPONENT_PROXY};
use crate::pipeline::{
    invoke_provider_method, model_endpoint_runtime_request_config, resolve_provider_entry_path,
};
use ennoia_logs::RequestContext;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    #[serde(flatten)]
    pub profile: AgentConfig,
    #[serde(default)]
    pub permission_profile: ennoia_kernel::AgentPermissionProfile,
}

impl From<ennoia_kernel::AgentDocument> for AgentRecord {
    fn from(value: ennoia_kernel::AgentDocument) -> Self {
        Self {
            profile: value.profile,
            permission_profile: value.permission_profile,
        }
    }
}

pub(super) async fn agents(State(state): State<AppState>) -> Json<Vec<AgentRecord>> {
    Json(
        load_agent_documents(&state.runtime_paths)
            .unwrap_or_default()
            .into_iter()
            .map(|document| normalize_agent_document(&state.runtime_paths, document))
            .map(AgentRecord::from)
            .collect(),
    )
}

pub(super) async fn agent_detail(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<Json<AgentRecord>, ApiError> {
    let document = load_agent_document(&state.runtime_paths, &agent_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    document
        .map(|document| normalize_agent_document(&state.runtime_paths, document))
        .map(AgentRecord::from)
        .map(Json)
        .ok_or_else(|| ApiError::not_found(format!("agent '{agent_id}' not found")))
}

pub(super) async fn agent_create(
    State(state): State<AppState>,
    Json(mut payload): Json<AgentRecord>,
) -> Result<Json<AgentRecord>, ApiError> {
    payload.profile.id = payload.profile.id.trim().to_string();
    write_agent_document(
        &state.runtime_paths,
        &ennoia_kernel::AgentDocument {
            profile: payload.profile.clone(),
            permission_profile: payload.permission_profile.clone(),
        },
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let document = load_agent_document(&state.runtime_paths, &payload.profile.id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    document
        .map(|document| normalize_agent_document(&state.runtime_paths, document))
        .map(AgentRecord::from)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created agent"))
}

pub(super) async fn agent_update(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
    Json(mut payload): Json<AgentRecord>,
) -> Result<Json<AgentRecord>, ApiError> {
    payload.profile.id = agent_id.clone();
    write_agent_document(
        &state.runtime_paths,
        &ennoia_kernel::AgentDocument {
            profile: payload.profile.clone(),
            permission_profile: payload.permission_profile.clone(),
        },
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let document = load_agent_document(&state.runtime_paths, &agent_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    document
        .map(|document| normalize_agent_document(&state.runtime_paths, document))
        .map(AgentRecord::from)
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

pub(super) async fn model_endpoints(
    State(state): State<AppState>,
) -> Json<Vec<ModelEndpointConfig>> {
    Json(load_model_endpoint_configs(&state.runtime_paths).unwrap_or_default())
}

pub(super) async fn model_endpoint_detail(
    State(state): State<AppState>,
    Path(model_endpoint_id): Path<String>,
) -> Result<Json<ModelEndpointConfig>, ApiError> {
    let model_endpoints = load_model_endpoint_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    model_endpoints
        .into_iter()
        .find(|model_endpoint| model_endpoint.id == model_endpoint_id)
        .map(Json)
        .ok_or_else(|| {
            ApiError::not_found(format!("model endpoint '{model_endpoint_id}' not found"))
        })
}

pub(super) async fn model_endpoint_models(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(model_endpoint_id): Path<String>,
) -> Result<Json<ModelEndpointModelsResponse>, ApiError> {
    let model_endpoints = load_model_endpoint_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    let model_endpoint = model_endpoints
        .into_iter()
        .find(|item| item.id == model_endpoint_id)
        .ok_or_else(|| {
            ApiError::not_found(format!("model endpoint '{model_endpoint_id}' not found"))
        })?;

    model_endpoint_models_response(&state, &model_endpoint, Some(&request)).map(Json)
}

pub(super) async fn model_endpoint_discover_models(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Json(payload): Json<ModelEndpointConfig>,
) -> Result<Json<ModelEndpointModelsResponse>, ApiError> {
    model_endpoint_models_response(&state, &payload, Some(&request)).map(Json)
}

fn model_endpoint_models_response(
    state: &AppState,
    model_endpoint: &ModelEndpointConfig,
    request: Option<&RequestContext>,
) -> Result<ModelEndpointModelsResponse, ApiError> {
    let contribution = resolve_provider_contribution(&state, &model_endpoint.kind)?;
    let mut models = model_endpoint.available_models.clone();
    let mut source = if models.is_empty() {
        "manual".to_string()
    } else {
        "configured".to_string()
    };
    let mut manual_allowed = model_endpoint.model_discovery.manual_allowed;
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
                        "model_endpoint": model_endpoint_runtime_request_config(&model_endpoint),
                }
            });
            let response = invoke_provider_method(&entry, &request_payload, &model_endpoint)
                .map_err(|error| {
                    let error_message = error.clone();
                    let trace = request.map(RequestContext::trace_context);
                    let _ = state.logs.append_log_scoped(
                        LogEntryWrite {
                            event: "runtime.model_endpoint.discovery_failed".to_string(),
                            level: "error".to_string(),
                            component: LOGS_COMPONENT_PROXY.to_string(),
                            source_kind: "interface".to_string(),
                            source_id: Some(model_endpoint.id.clone()),
                            message: "获取上游模型失败".to_string(),
                            attributes: serde_json::json!({
                                "error": error_message,
                                "operation": "list_models",
                                "model_endpoint_id": model_endpoint.id,
                                "display_name": model_endpoint.display_name,
                                "provider_kind": model_endpoint.kind,
                                "base_url": model_endpoint.base_url,
                                "api_key_env": model_endpoint.api_key_env,
                            }),
                            created_at: None,
                        },
                        trace.as_ref(),
                    );
                    ApiError::internal(error)
                })?;
            let extension_models = parse_provider_models_from_response(&response)?;
            if !extension_models.is_empty() {
                models = extension_models;
                source = "extension".to_string();
            }
        }
    }

    Ok(ModelEndpointModelsResponse {
        model_endpoint_id: model_endpoint.id.clone(),
        source,
        models,
        manual_allowed,
        generation_options,
    })
}

pub(super) async fn model_endpoint_create(
    State(state): State<AppState>,
    Json(payload): Json<ModelEndpointConfig>,
) -> Result<Json<ModelEndpointConfig>, ApiError> {
    validate_model_endpoint_payload(&state, &payload)?;
    write_config_to_dir(
        state.runtime_paths.model_endpoints_config_dir(),
        &payload.id,
        &payload,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let model_endpoints = load_model_endpoint_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    model_endpoints
        .into_iter()
        .find(|model_endpoint| model_endpoint.id == payload.id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created model endpoint"))
}

pub(super) async fn model_endpoint_update(
    State(state): State<AppState>,
    Path(model_endpoint_id): Path<String>,
    Json(mut payload): Json<ModelEndpointConfig>,
) -> Result<Json<ModelEndpointConfig>, ApiError> {
    payload.id = model_endpoint_id.clone();
    validate_model_endpoint_payload(&state, &payload)?;
    write_config_to_dir(
        state.runtime_paths.model_endpoints_config_dir(),
        &model_endpoint_id,
        &payload,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    let model_endpoints = load_model_endpoint_configs(&state.runtime_paths)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    model_endpoints
        .into_iter()
        .find(|model_endpoint| model_endpoint.id == model_endpoint_id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated model endpoint"))
}

pub(super) async fn model_endpoint_delete(
    State(state): State<AppState>,
    Path(model_endpoint_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_config_from_dir(
        state.runtime_paths.model_endpoints_config_dir(),
        &model_endpoint_id,
    )
    .map_err(|error| ApiError::internal(error.to_string()))?;
    if deleted {
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(ApiError::not_found(format!(
            "model endpoint '{model_endpoint_id}' not found"
        )))
    }
}

pub(super) async fn spaces(State(state): State<AppState>) -> Json<Vec<ennoia_kernel::SpaceSpec>> {
    Json(state.spaces)
}

fn validate_model_endpoint_payload(
    state: &AppState,
    payload: &ModelEndpointConfig,
) -> Result<(), ApiError> {
    let _ = resolve_provider_contribution(state, &payload.kind)?;
    if payload.enabled && payload.default_model.trim().is_empty() {
        return Err(ApiError::bad_request(
            "启用模型接入前必须配置默认模型；无法发现模型时使用手动输入。",
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
            "接口类型 '{normalized}' 对应多个实现扩展，当前不允许创建模型接入。"
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
