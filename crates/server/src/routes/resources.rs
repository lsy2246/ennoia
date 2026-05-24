use super::*;
use crate::app::{
    delete_agent_config, load_agent_document, load_agent_documents, normalize_agent_document,
    write_agent_document,
};
use crate::execution::{resolve_agent_file_path, AgentFileAccessPaths};
use crate::realtime::RealtimeEvent;
use crate::skills::{
    load_skill_manifest, load_skill_settings, load_skill_status, run_skill_check,
    run_skill_prepare, save_skill_settings, validate_skill_settings_payload,
};
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    #[serde(flatten)]
    pub profile: AgentConfig,
    #[serde(default)]
    pub permission_profile: ennoia_kernel::AgentPermissionProfile,
    #[serde(default)]
    pub file_access: ennoia_kernel::AgentFileAccessProfile,
}

impl From<ennoia_kernel::AgentDocument> for AgentRecord {
    fn from(value: ennoia_kernel::AgentDocument) -> Self {
        Self {
            profile: value.profile,
            permission_profile: value.permission_profile,
            file_access: value.file_access,
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

pub(super) async fn agent_artifact_download(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Query(query): Query<AgentArtifactQuery>,
    Path((agent_id, artifact_path)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    let document = load_agent_document(&state.runtime_paths, &agent_id)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?
        .ok_or_else(|| {
            scoped(
                ApiError::not_found(format!("agent '{agent_id}' not found")),
                &request,
            )
        })?;
    let paths = AgentFileAccessPaths::for_agent(&state, &document.profile, "artifact-download");
    let resolved = resolve_agent_file_path(
        &document.file_access,
        &paths,
        &format!("/artifacts/{artifact_path}"),
    )
    .map_err(|error| scoped(error, &request))?;
    if !resolved.host_path.is_file() {
        return Err(scoped(
            ApiError::not_found(format!("artifact '{}' not found", resolved.display_path)),
            &request,
        ));
    }
    let body = fs::read(&resolved.host_path)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))?;
    let content_type = mime_guess::from_path(&resolved.host_path)
        .first_or_octet_stream()
        .to_string();
    let disposition = if query.download.as_deref() == Some("1") {
        "attachment"
    } else {
        "inline"
    };
    let filename = artifact_download_filename(&resolved.host_path);

    Ok((
        [
            (header::CONTENT_TYPE, content_type),
            (header::CACHE_CONTROL, "no-cache".to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!(
                    "{disposition}; filename=\"{}\"; filename*=UTF-8''{}",
                    filename.ascii_fallback, filename.encoded
                ),
            ),
        ],
        body,
    ))
}

#[derive(Debug, Default, Deserialize)]
pub(super) struct AgentArtifactQuery {
    download: Option<String>,
}

struct ArtifactDownloadFilename {
    ascii_fallback: String,
    encoded: String,
}

fn artifact_download_filename(path: &StdPath) -> ArtifactDownloadFilename {
    let filename = path
        .file_name()
        .and_then(|item| item.to_str())
        .filter(|item| !item.is_empty())
        .unwrap_or("artifact");
    let ascii_fallback = filename
        .chars()
        .map(|item| match item {
            '"' | '\\' | '\r' | '\n' => '_',
            '\u{20}'..='\u{7e}' => item,
            _ => '_',
        })
        .collect::<String>();
    let ascii_fallback = if ascii_fallback.trim().is_empty() {
        "artifact".to_string()
    } else {
        ascii_fallback
    };
    ArtifactDownloadFilename {
        ascii_fallback,
        encoded: encode_header_filename(filename),
    }
}

fn encode_header_filename(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        let item = *byte;
        if item.is_ascii_alphanumeric() || matches!(item, b'.' | b'-' | b'_') {
            encoded.push(item as char);
        } else {
            encoded.push_str(&format!("%{item:02X}"));
        }
    }
    encoded
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
            file_access: payload.file_access.clone(),
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
            file_access: payload.file_access.clone(),
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
    Json(load_skill_configs(&state.runtime_paths, state.allow_dev_sources).unwrap_or_default())
}

pub(super) async fn skill_detail(
    State(state): State<AppState>,
    Path(skill_id): Path<String>,
) -> Result<Json<SkillConfig>, ApiError> {
    let skills = load_skill_configs(&state.runtime_paths, state.allow_dev_sources)
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
    let skills = load_skill_configs(&state.runtime_paths, state.allow_dev_sources)
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
    let skills = load_skill_configs(&state.runtime_paths, state.allow_dev_sources)
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
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id, state.allow_dev_sources)
        .map_err(|error| {
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
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id, state.allow_dev_sources)
        .map_err(|error| {
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
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id, state.allow_dev_sources)
        .map_err(|error| {
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
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id, state.allow_dev_sources)
        .map_err(|error| {
            scoped(
                ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
                &request,
            )
        })?;
    run_skill_check(&state.runtime_paths, &manifest, state.allow_dev_sources)
        .await
        .map(Json)
        .map_err(|error| scoped(ApiError::internal(error.to_string()), &request))
}

pub(super) async fn skill_prepare(
    State(state): State<AppState>,
    Extension(request): Extension<RequestContext>,
    Path(skill_id): Path<String>,
) -> ApiResult<SkillCheckResult> {
    let manifest = load_skill_manifest(&state.runtime_paths, &skill_id, state.allow_dev_sources)
        .map_err(|error| {
            scoped(
                ApiError::not_found(format!("skill '{skill_id}' not found: {error}")),
                &request,
            )
        })?;
    run_skill_prepare(&state.runtime_paths, &manifest, state.allow_dev_sources)
        .await
        .map(Json)
        .map_err(|error| {
            let api_error = if error.kind() == std::io::ErrorKind::InvalidInput {
                ApiError::bad_request(error.to_string())
            } else {
                ApiError::internal(error.to_string())
            };
            scoped(api_error, &request)
        })
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
    use super::{
        agent_artifact_download, skill_prepare, validate_model_endpoint_request_timeout_ms,
        AgentArtifactQuery,
    };
    use crate::app::{default_app_state, write_agent_document};
    use axum::extract::{Path, Query, State};
    use axum::response::IntoResponse;
    use axum::Extension;
    use ennoia_kernel::{AgentConfig, AgentDocument, AgentPermissionProfile};
    use ennoia_logs::RequestContext;
    use ennoia_paths::RuntimePaths;
    use std::fs;
    use std::sync::Arc;
    use tempfile::tempdir;

    fn test_request_context() -> RequestContext {
        RequestContext {
            request_id: "req_test".to_string(),
            trace_id: "trace_test".to_string(),
            span_id: "span_test".to_string(),
            parent_span_id: None,
            sampled: true,
            source: "test".to_string(),
        }
    }

    fn write_skill_package_with_prepare(root: &std::path::Path, include_prepare: bool) {
        fs::create_dir_all(root.join("scripts")).expect("create scripts dir");
        fs::write(
            root.join("SKILL.md"),
            r#"---
name: sample
description: sample skill
---

# Sample
"#,
        )
        .expect("write skill markdown");
        let prepare = if include_prepare {
            r#"
[prepare]
runner = "node"
entry = "scripts/prepare.js"
timeout_ms = 10000
"#
        } else {
            ""
        };
        fs::write(
            root.join("config.toml"),
            format!(
                r#"
version = "1.0.0"

[mount]
mode = "auto"

[diagnostics]
manual_check = true

[diagnostics.check]
runner = "node"
entry = "scripts/doctor.js"
timeout_ms = 10000

{prepare}
"#
            ),
        )
        .expect("write skill config");
        fs::write(
            root.join("scripts").join("prepare.js"),
            r#"const fs = require("node:fs"); const path = require("node:path"); fs.writeFileSync(path.join(process.env.ENNOIA_SKILL_DATA_DIR, "prepared.txt"), "ready");"#,
        )
        .expect("write prepare script");
        fs::write(
            root.join("scripts").join("doctor.js"),
            r#"console.log(JSON.stringify({ status: "ready", summary: "prepared", items: [], actions: [] }));"#,
        )
        .expect("write doctor script");
    }

    fn state_for_paths(paths: RuntimePaths) -> crate::app::AppState {
        let mut state = default_app_state();
        state.runtime_paths = Arc::new(paths);
        state.allow_dev_sources = false;
        state
    }

    fn sample_agent_document(agent_id: &str) -> AgentDocument {
        AgentDocument {
            profile: AgentConfig {
                id: agent_id.to_string(),
                display_name: agent_id.to_string(),
                description: String::new(),
                system_prompt: String::new(),
                model_endpoint_id: String::new(),
                model_id: String::new(),
                generation_options: Default::default(),
                skills: Vec::new(),
                enabled: true,
                kind: "agent".to_string(),
                default_model: String::new(),
                skills_dir: String::new(),
                working_dir: String::new(),
                artifacts_dir: String::new(),
            },
            permission_profile: AgentPermissionProfile::default_profile(),
            file_access: Default::default(),
        }
    }

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

    #[tokio::test]
    async fn skill_prepare_handler_runs_declared_prepare_and_returns_latest_status() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path().join("home"));
        paths.ensure_layout().expect("layout");
        write_skill_package_with_prepare(&paths.skill_dir("sample"), true);
        let state = state_for_paths(paths.clone());

        let response = skill_prepare(
            State(state),
            Extension(test_request_context()),
            Path("sample".to_string()),
        )
        .await
        .expect("prepare response")
        .0;

        assert_eq!(response.status, ennoia_kernel::SkillRuntimeStatus::Ready);
        assert_eq!(response.summary, "prepared");
        assert!(paths
            .skill_state_dir("sample")
            .join("prepared.txt")
            .exists());
    }

    #[tokio::test]
    async fn skill_prepare_handler_rejects_skill_without_prepare() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path().join("home"));
        paths.ensure_layout().expect("layout");
        write_skill_package_with_prepare(&paths.skill_dir("sample"), false);
        let state = state_for_paths(paths);

        let error = skill_prepare(
            State(state),
            Extension(test_request_context()),
            Path("sample".to_string()),
        )
        .await
        .expect_err("prepare should fail");

        assert_eq!(error.code(), ennoia_contract::ErrorCode::BadRequest);
        assert!(error
            .message()
            .contains("does not define a prepare workflow"));
    }

    #[tokio::test]
    async fn agent_artifact_download_serves_file_from_agent_artifacts_root() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path().join("home"));
        paths.ensure_layout().expect("layout");
        write_agent_document(&paths, &sample_agent_document("lsy")).expect("write agent");
        fs::create_dir_all(paths.agent_artifacts_dir("lsy")).expect("create artifacts");
        fs::write(
            paths.agent_artifacts_dir("lsy").join("bilibili.png"),
            b"png",
        )
        .expect("write artifact");
        let state = state_for_paths(paths);

        let response = agent_artifact_download(
            State(state),
            Extension(test_request_context()),
            Query(AgentArtifactQuery::default()),
            Path(("lsy".to_string(), "bilibili.png".to_string())),
        )
        .await
        .expect("artifact response")
        .into_response();

        assert_eq!(response.status(), axum::http::StatusCode::OK);
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_TYPE)
                .and_then(|value| value.to_str().ok()),
            Some("image/png")
        );
        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some("inline; filename=\"bilibili.png\"; filename*=UTF-8''bilibili.png")
        );
    }

    #[tokio::test]
    async fn agent_artifact_download_can_force_attachment_disposition() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path().join("home"));
        paths.ensure_layout().expect("layout");
        write_agent_document(&paths, &sample_agent_document("lsy")).expect("write agent");
        fs::create_dir_all(paths.agent_artifacts_dir("lsy")).expect("create artifacts");
        fs::write(
            paths.agent_artifacts_dir("lsy").join("Bilibili 官网.png"),
            b"png",
        )
        .expect("write artifact");
        let state = state_for_paths(paths);

        let response = agent_artifact_download(
            State(state),
            Extension(test_request_context()),
            Query(AgentArtifactQuery {
                download: Some("1".to_string()),
            }),
            Path(("lsy".to_string(), "Bilibili 官网.png".to_string())),
        )
        .await
        .expect("artifact response")
        .into_response();

        assert_eq!(
            response
                .headers()
                .get(axum::http::header::CONTENT_DISPOSITION)
                .and_then(|value| value.to_str().ok()),
            Some(
                "attachment; filename=\"Bilibili __.png\"; filename*=UTF-8''Bilibili%20%E5%AE%98%E7%BD%91.png"
            )
        );
    }

    #[tokio::test]
    async fn agent_artifact_download_rejects_paths_outside_artifacts_root() {
        let temp = tempdir().expect("temp dir");
        let paths = RuntimePaths::new(temp.path().join("home"));
        paths.ensure_layout().expect("layout");
        write_agent_document(&paths, &sample_agent_document("lsy")).expect("write agent");
        let state = state_for_paths(paths);

        let error = match agent_artifact_download(
            State(state),
            Extension(test_request_context()),
            Query(AgentArtifactQuery::default()),
            Path(("lsy".to_string(), "../agent.toml".to_string())),
        )
        .await
        {
            Ok(_) => panic!("path escape should fail"),
            Err(error) => error,
        };

        assert_eq!(error.code(), ennoia_contract::ErrorCode::BadRequest);
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
