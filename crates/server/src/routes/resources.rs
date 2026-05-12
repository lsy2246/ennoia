use super::*;
use crate::app::{
    delete_agent_config, load_agent_document, load_agent_documents, normalize_agent_document,
    write_agent_document,
};
use crate::realtime::RealtimeEvent;
use crate::skills::{
    load_skill_manifest, load_skill_settings, load_skill_status, run_skill_check,
    save_skill_settings, validate_skill_settings_payload,
};
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
    let created = document
        .map(|document| normalize_agent_document(&state.runtime_paths, document))
        .map(AgentRecord::from)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created agent"))?;
    state.realtime.publish(RealtimeEvent::ModelEndpointsChanged);
    Ok(created)
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
    let updated = document
        .map(|document| normalize_agent_document(&state.runtime_paths, document))
        .map(AgentRecord::from)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated agent"))?;
    state.realtime.publish(RealtimeEvent::ModelEndpointsChanged);
    Ok(updated)
}

pub(super) async fn agent_delete(
    State(state): State<AppState>,
    Path(agent_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    let deleted = delete_agent_config(&state.runtime_paths, &agent_id)
        .map_err(|error| ApiError::internal(error.to_string()))?;
    if deleted {
        state.realtime.publish(RealtimeEvent::ModelEndpointsChanged);
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

pub(super) async fn skill_settings(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(skill_id): Path<String>,
) -> ApiResult<SkillSettingsRecord> {
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id).map_err(|error| {
        scoped(
            ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
            &request,
        )
    })?;
    Ok(Json(load_skill_settings(&state.runtime_paths, &manifest)))
}

pub(super) async fn skill_settings_put(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(skill_id): Path<String>,
    Json(payload): Json<SkillSettingsPayload>,
) -> ApiResult<SkillSettingsRecord> {
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id).map_err(|error| {
        scoped(
            ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
            &request,
        )
    })?;
    validate_skill_settings_payload(&manifest, &payload)
        .map_err(|error| scoped(ApiError::bad_request(error), &request))?;
    save_skill_settings(&state.runtime_paths, &manifest, &payload)
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn skill_status(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(skill_id): Path<String>,
) -> ApiResult<SkillCheckResult> {
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id).map_err(|error| {
        scoped(
            ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
            &request,
        )
    })?;
    Ok(Json(load_skill_status(&state.runtime_paths, &manifest)))
}

pub(super) async fn skill_check(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(skill_id): Path<String>,
) -> ApiResult<SkillCheckResult> {
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id).map_err(|error| {
        scoped(
            ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
            &request,
        )
    })?;
    run_skill_check(&state.runtime_paths, &manifest)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
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
    let created = model_endpoints
        .into_iter()
        .find(|model_endpoint| model_endpoint.id == payload.id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload created model endpoint"))?;
    state.realtime.publish(RealtimeEvent::AgentsChanged);
    Ok(created)
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
    let updated = model_endpoints
        .into_iter()
        .find(|model_endpoint| model_endpoint.id == model_endpoint_id)
        .map(Json)
        .ok_or_else(|| ApiError::internal("failed to reload updated model endpoint"))?;
    state.realtime.publish(RealtimeEvent::AgentsChanged);
    Ok(updated)
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
        state.realtime.publish(RealtimeEvent::AgentsChanged);
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
    validate_model_endpoint_request_timeout_ms(payload.request_timeout_ms)?;
    Ok(())
}

pub(super) fn validate_model_endpoint_request_timeout_ms(
    request_timeout_ms: Option<u64>,
) -> Result<(), ApiError> {
    if let Some(timeout_ms) = request_timeout_ms {
        if timeout_ms > 0 && timeout_ms < 1_000 {
            return Err(ApiError::bad_request(
                "模型接入超时只能填写 0（不限制）或不小于 1000ms 的值。",
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::validate_model_endpoint_request_timeout_ms;

    #[test]
    fn allows_unlimited_model_endpoint_timeout() {
        assert!(validate_model_endpoint_request_timeout_ms(Some(0)).is_ok());
    }

    #[test]
    fn rejects_model_endpoint_timeout_below_minimum_when_non_zero() {
        assert!(validate_model_endpoint_request_timeout_ms(Some(999)).is_err());
    }

    #[test]
    fn allows_model_endpoint_timeout_at_minimum() {
        assert!(validate_model_endpoint_request_timeout_ms(Some(1_000)).is_ok());
    }
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
