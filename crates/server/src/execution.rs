use std::path::{Component, Path, PathBuf};

use ennoia_contract::ApiError;
use ennoia_kernel::{AgentConfig, AgentFileAccessProfile};

use crate::app::AppState;

#[derive(Debug, Clone)]
pub(crate) struct AgentFileAccessPaths {
    pub workspace: PathBuf,
    pub artifacts: PathBuf,
    pub temp: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedFileAccessPath {
    pub host_path: PathBuf,
    pub display_path: String,
}

impl AgentFileAccessPaths {
    pub(crate) fn for_agent(state: &AppState, agent: &AgentConfig, run_id: &str) -> Self {
        let workspace = state.runtime_paths.agent_working_dir(&agent.id);
        let artifacts = state.runtime_paths.agent_artifacts_dir(&agent.id);
        let temp = state
            .runtime_paths
            .state_cache_dir()
            .join("file-access")
            .join(&agent.id)
            .join(run_id);
        Self {
            workspace,
            artifacts,
            temp,
        }
    }

    fn host_root(&self, root_id: &str) -> Option<&PathBuf> {
        match root_id {
            "workspace" => Some(&self.workspace),
            "artifacts" => Some(&self.artifacts),
            "temp" => Some(&self.temp),
            _ => None,
        }
    }
}

pub(crate) fn resolve_agent_file_path(
    profile: &AgentFileAccessProfile,
    paths: &AgentFileAccessPaths,
    input: &str,
) -> Result<ResolvedFileAccessPath, ApiError> {
    let normalized = normalize_file_access_path(input);
    let raw = normalized.as_str();
    if raw.is_empty() {
        return resolve_agent_file_path(profile, paths, default_root(profile).as_str());
    }

    if is_probably_host_absolute_path(raw) && !raw.starts_with('/') {
        return Err(file_access_root_error(profile));
    }

    if raw.starts_with('/') {
        return resolve_virtual_path(profile, paths, raw)
            .ok_or_else(|| file_access_root_error(profile))?;
    }

    let default_root = default_root(profile);
    resolve_virtual_path(profile, paths, &format!("{default_root}/{raw}"))
        .ok_or_else(|| file_access_root_error(profile))?
}

pub(crate) fn resolve_command_cwd(
    profile: &AgentFileAccessProfile,
    paths: &AgentFileAccessPaths,
    cwd: Option<&str>,
) -> Result<ResolvedFileAccessPath, ApiError> {
    match cwd {
        Some(value) => resolve_agent_file_path(profile, paths, value),
        None => resolve_agent_file_path(profile, paths, default_root(profile).as_str()),
    }
}

fn resolve_virtual_path(
    profile: &AgentFileAccessProfile,
    paths: &AgentFileAccessPaths,
    raw: &str,
) -> Option<Result<ResolvedFileAccessPath, ApiError>> {
    let mut roots = profile.roots.iter().collect::<Vec<_>>();
    roots.sort_by(|left, right| right.path.len().cmp(&left.path.len()));
    for root in roots {
        let root_path = normalize_virtual_root(&root.path);
        let suffix = if raw == root_path {
            Some("")
        } else {
            raw.strip_prefix(&format!("{root_path}/"))
        };
        let Some(suffix) = suffix else {
            continue;
        };
        let Some(host_root) = paths.host_root(root.id.as_str()) else {
            return Some(Err(ApiError::bad_request(format!(
                "file access root '{}' is not available in this runtime",
                root.id
            ))));
        };
        return Some(resolve_virtual_root(host_root, &root_path, suffix));
    }
    None
}

fn default_root(profile: &AgentFileAccessProfile) -> String {
    let configured = normalize_virtual_root(&profile.default_root);
    if profile
        .roots
        .iter()
        .any(|root| normalize_virtual_root(&root.path) == configured)
    {
        configured
    } else {
        "/workspace".to_string()
    }
}

fn normalize_virtual_root(value: &str) -> String {
    let normalized = normalize_file_access_path(value);
    let trimmed = normalized.trim_end_matches('/');
    if trimmed.is_empty() {
        "/workspace".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn normalize_file_access_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.replace('\\', "/")
}

fn file_access_root_error(profile: &AgentFileAccessProfile) -> ApiError {
    let roots = profile
        .roots
        .iter()
        .map(|root| normalize_virtual_root(&root.path))
        .collect::<Vec<_>>();
    ApiError::bad_request(format!(
        "file access only accepts configured virtual roots: {}",
        roots.join(", ")
    ))
}

fn resolve_virtual_root(
    root_path: &Path,
    root_label: &str,
    suffix: &str,
) -> Result<ResolvedFileAccessPath, ApiError> {
    let trimmed_suffix = suffix.trim_start_matches('/');
    let safe_suffix = sanitize_relative_suffix(trimmed_suffix)?;
    let host_path = if safe_suffix.as_os_str().is_empty() {
        root_path.to_path_buf()
    } else {
        root_path.join(&safe_suffix)
    };
    Ok(resolved_path(host_path, root_label, &safe_suffix))
}

fn resolved_path(
    host_path: PathBuf,
    root: &str,
    suffix: impl AsRef<Path>,
) -> ResolvedFileAccessPath {
    let suffix = suffix.as_ref().to_string_lossy().replace('\\', "/");
    let suffix = suffix.trim_start_matches('/');
    let display_path = if suffix.is_empty() {
        root.to_string()
    } else {
        format!("{root}/{suffix}")
    };
    ResolvedFileAccessPath {
        host_path,
        display_path,
    }
}

fn sanitize_relative_suffix(value: &str) -> Result<PathBuf, ApiError> {
    let mut clean = PathBuf::new();
    for component in Path::new(value).components() {
        match component {
            Component::Normal(part) => clean.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(ApiError::bad_request(
                    "path cannot escape the selected file access root".to_string(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ApiError::bad_request(
                    "path must stay inside the selected file access root".to_string(),
                ));
            }
        }
    }
    Ok(clean)
}

fn is_probably_host_absolute_path(value: &str) -> bool {
    value.contains(":\\") || value.contains(":/")
}
