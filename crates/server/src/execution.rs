use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use ennoia_contract::ApiError;
use ennoia_kernel::{AgentConfig, AgentExecutionEnvironment};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use uuid::Uuid;

use crate::app::AppState;

#[derive(Debug, Clone)]
pub(crate) struct AgentExecutionPaths {
    pub workspace: PathBuf,
    pub artifacts: PathBuf,
    pub temp: PathBuf,
}

#[derive(Debug, Clone)]
pub(crate) struct ResolvedExecutionPath {
    pub host_path: PathBuf,
    pub display_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxRoots {
    pub workspace: String,
    pub artifacts: String,
    pub temp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SandboxOperation {
    FsRead {
        host_path: String,
        display_path: String,
        max_bytes: usize,
    },
    FsWrite {
        host_path: String,
        display_path: String,
        content: String,
        append: bool,
    },
    CommandExec {
        command: String,
        args: Vec<String>,
        cwd_host_path: String,
        cwd_display_path: String,
        timeout_ms: u64,
    },
    NetFetch {
        url: String,
        method: String,
        headers: BTreeMap<String, String>,
        body: Option<String>,
        timeout_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxWorkerRequest {
    pub roots: SandboxRoots,
    pub operation: SandboxOperation,
    #[serde(default)]
    pub allow_network: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxWorkerResponse {
    pub ok: bool,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
}

impl AgentExecutionPaths {
    pub(crate) fn for_agent(state: &AppState, agent: &AgentConfig, run_id: &str) -> Self {
        let workspace = state.runtime_paths.agent_working_dir(&agent.id);
        let artifacts = state.runtime_paths.agent_artifacts_dir(&agent.id);
        let temp = state
            .runtime_paths
            .state_cache_dir()
            .join("sandboxes")
            .join(&agent.id)
            .join(run_id);
        Self {
            workspace,
            artifacts,
            temp,
        }
    }

    pub(crate) fn sandbox_roots(&self) -> SandboxRoots {
        SandboxRoots {
            workspace: self.workspace.to_string_lossy().replace('\\', "/"),
            artifacts: self.artifacts.to_string_lossy().replace('\\', "/"),
            temp: self.temp.to_string_lossy().replace('\\', "/"),
        }
    }
}

pub(crate) fn resolve_agent_tool_path(
    environment: &AgentExecutionEnvironment,
    paths: &AgentExecutionPaths,
    input: &str,
) -> Result<ResolvedExecutionPath, ApiError> {
    let normalized = normalize_tool_path(input);
    let raw = normalized.as_str();
    let mode = environment.normalized_mode();

    if let Some(rest) = raw.strip_prefix("/workspace") {
        return resolve_virtual_root(&paths.workspace, "/workspace", rest);
    }
    if let Some(rest) = raw.strip_prefix("/artifacts") {
        return resolve_virtual_root(&paths.artifacts, "/artifacts", rest);
    }
    if let Some(rest) = raw.strip_prefix("/tmp") {
        return resolve_virtual_root(&paths.temp, "/tmp", rest);
    }

    if mode == "native" && is_probably_host_absolute_path(raw) {
        return Err(ApiError::bad_request(
            "native sandbox only accepts /workspace, /artifacts and /tmp paths".to_string(),
        ));
    }

    if Path::new(raw).is_absolute() {
        return Ok(resolved_path(PathBuf::from(raw), "", raw));
    }

    resolve_virtual_root(&paths.workspace, "/workspace", raw)
}

pub(crate) fn resolve_command_cwd(
    environment: &AgentExecutionEnvironment,
    paths: &AgentExecutionPaths,
    cwd: Option<&str>,
) -> Result<ResolvedExecutionPath, ApiError> {
    match cwd {
        Some(value) => resolve_agent_tool_path(environment, paths, value),
        None => Ok(ResolvedExecutionPath {
            host_path: paths.workspace.clone(),
            display_path: "/workspace".to_string(),
        }),
    }
}

pub(crate) async fn execute_native_operation(
    agent: &AgentConfig,
    paths: &AgentExecutionPaths,
    allow_network: bool,
    operation: SandboxOperation,
) -> Result<String, ApiError> {
    let request_dir = paths.temp.clone();
    fs::create_dir_all(&request_dir)
        .map_err(|error| ApiError::internal(format!("create sandbox temp dir failed: {error}")))?;
    let request_path = request_dir.join(format!("req-{}.json", Uuid::new_v4()));
    let response_path = request_dir.join(format!("res-{}.json", Uuid::new_v4()));
    let request = SandboxWorkerRequest {
        roots: paths.sandbox_roots(),
        operation,
        allow_network,
    };
    fs::write(
        &request_path,
        serde_json::to_vec_pretty(&request).map_err(|error| {
            ApiError::internal(format!("serialize sandbox request failed: {error}"))
        })?,
    )
    .map_err(|error| ApiError::internal(format!("write sandbox request failed: {error}")))?;

    let agent_id = agent.id.clone();
    let request_path_string = request_path.to_string_lossy().to_string();
    let response_path_string = response_path.to_string_lossy().to_string();
    let paths_clone = paths.clone();
    let worker_result = tokio::task::spawn_blocking(move || {
        launch_native_worker(
            &agent_id,
            &paths_clone,
            allow_network,
            &request_path_string,
            &response_path_string,
        )
    })
    .await
    .map_err(|error| ApiError::internal(format!("join sandbox worker failed: {error}")))?
    .map_err(ApiError::internal)?;

    if !worker_result.success {
        return Err(ApiError::internal(worker_result.message));
    }

    let response_bytes = fs::read(&response_path)
        .map_err(|error| ApiError::internal(format!("read sandbox response failed: {error}")))?;
    let response = serde_json::from_slice::<SandboxWorkerResponse>(&response_bytes)
        .map_err(|error| ApiError::internal(format!("parse sandbox response failed: {error}")))?;

    let _ = fs::remove_file(&request_path);
    let _ = fs::remove_file(&response_path);

    if response.ok {
        response
            .content
            .ok_or_else(|| ApiError::internal("sandbox response missing content"))
    } else {
        Err(ApiError::internal(
            response
                .error
                .unwrap_or_else(|| "sandbox worker failed".to_string()),
        ))
    }
}

pub async fn run_sandbox_worker(request_path: &str, response_path: &str) -> Result<(), String> {
    let request_bytes =
        fs::read(request_path).map_err(|error| format!("read sandbox request failed: {error}"))?;
    let request = serde_json::from_slice::<SandboxWorkerRequest>(&request_bytes)
        .map_err(|error| format!("parse sandbox request failed: {error}"))?;

    let response = match execute_worker_request(request).await {
        Ok(content) => SandboxWorkerResponse {
            ok: true,
            content: Some(content),
            error: None,
        },
        Err(error) => SandboxWorkerResponse {
            ok: false,
            content: None,
            error: Some(error),
        },
    };

    let bytes = serde_json::to_vec_pretty(&response)
        .map_err(|error| format!("serialize sandbox response failed: {error}"))?;
    fs::write(response_path, bytes)
        .map_err(|error| format!("write sandbox response failed: {error}"))?;
    Ok(())
}

async fn execute_worker_request(request: SandboxWorkerRequest) -> Result<String, String> {
    match request.operation {
        SandboxOperation::FsRead {
            host_path,
            display_path,
            max_bytes,
        } => execute_worker_fs_read(&request.roots, &host_path, &display_path, max_bytes),
        SandboxOperation::FsWrite {
            host_path,
            display_path,
            content,
            append,
        } => execute_worker_fs_write(&request.roots, &host_path, &display_path, &content, append),
        SandboxOperation::CommandExec {
            command,
            args,
            cwd_host_path,
            cwd_display_path,
            timeout_ms,
        } => {
            execute_worker_command_exec(
                &request.roots,
                &command,
                &args,
                &cwd_host_path,
                &cwd_display_path,
                timeout_ms,
            )
            .await
        }
        SandboxOperation::NetFetch {
            url,
            method,
            headers,
            body,
            timeout_ms,
        } => {
            execute_worker_net_fetch(
                request.allow_network,
                &url,
                &method,
                &headers,
                body.as_deref(),
                timeout_ms,
            )
            .await
        }
    }
}

fn execute_worker_fs_read(
    roots: &SandboxRoots,
    host_path: &str,
    display_path: &str,
    max_bytes: usize,
) -> Result<String, String> {
    ensure_within_roots(roots, host_path)?;
    let bytes = fs::read(host_path).map_err(|error| format!("read file failed: {error}"))?;
    let truncated = bytes.len() > max_bytes;
    let visible = if truncated {
        &bytes[..max_bytes]
    } else {
        &bytes[..]
    };
    Ok(serde_json::json!({
        "ok": true,
        "tool": "fs.read",
        "path": display_path,
        "bytes_read": visible.len(),
        "truncated": truncated,
        "content": String::from_utf8_lossy(visible),
    })
    .to_string())
}

fn execute_worker_fs_write(
    roots: &SandboxRoots,
    host_path: &str,
    display_path: &str,
    content: &str,
    append: bool,
) -> Result<String, String> {
    ensure_within_roots(roots, host_path)?;
    if let Some(parent) = Path::new(host_path).parent() {
        fs::create_dir_all(parent).map_err(|error| format!("create parent dir failed: {error}"))?;
    }
    if append {
        use std::io::Write as _;
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(host_path)
            .map_err(|error| format!("open file for append failed: {error}"))?;
        file.write_all(content.as_bytes())
            .map_err(|error| format!("append file failed: {error}"))?;
    } else {
        fs::write(host_path, content.as_bytes())
            .map_err(|error| format!("write file failed: {error}"))?;
    }
    Ok(serde_json::json!({
        "ok": true,
        "tool": "fs.write",
        "path": display_path,
        "bytes_written": content.len(),
        "append": append,
    })
    .to_string())
}

async fn execute_worker_command_exec(
    roots: &SandboxRoots,
    command: &str,
    args: &[String],
    cwd_host_path: &str,
    cwd_display_path: &str,
    timeout_ms: u64,
) -> Result<String, String> {
    ensure_within_roots(roots, cwd_host_path)?;
    let mut child = tokio::process::Command::new(command);
    child.args(args).current_dir(cwd_host_path);
    let output = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), child.output())
        .await
        .map_err(|_| "command exec timed out".to_string())?
        .map_err(|error| format!("spawn command failed: {error}"))?;
    Ok(serde_json::json!({
        "ok": output.status.success(),
        "tool": "command.exec",
        "command": command,
        "args": args,
        "cwd": cwd_display_path,
        "status": output.status.code(),
        "stdout": String::from_utf8_lossy(&output.stdout),
        "stderr": String::from_utf8_lossy(&output.stderr),
    })
    .to_string())
}

async fn execute_worker_net_fetch(
    allow_network: bool,
    url: &str,
    method: &str,
    headers: &BTreeMap<String, String>,
    body: Option<&str>,
    timeout_ms: u64,
) -> Result<String, String> {
    if !allow_network {
        return Err("sandbox worker network capability is disabled".to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_millis(timeout_ms))
        .build()
        .map_err(|error| format!("build http client failed: {error}"))?;
    let mut request_builder = client.request(
        reqwest::Method::from_bytes(method.as_bytes())
            .map_err(|error| format!("invalid http method: {error}"))?,
        url,
    );
    for (key, value) in headers {
        request_builder = request_builder.header(key, value);
    }
    if let Some(body) = body {
        request_builder = request_builder.body(body.to_string());
    }
    let response = request_builder
        .send()
        .await
        .map_err(|error| format!("http request failed: {error}"))?;
    let status = response.status().as_u16();
    let response_headers = response
        .headers()
        .iter()
        .map(|(key, value)| {
            (
                key.as_str().to_string(),
                JsonValue::String(value.to_str().unwrap_or_default().to_string()),
            )
        })
        .collect::<serde_json::Map<String, JsonValue>>();
    let text = response
        .text()
        .await
        .map_err(|error| format!("read http response failed: {error}"))?;
    Ok(serde_json::json!({
        "ok": true,
        "tool": "net.fetch",
        "url": url,
        "status": status,
        "headers": response_headers,
        "body": text,
    })
    .to_string())
}

fn ensure_within_roots(roots: &SandboxRoots, host_path: &str) -> Result<(), String> {
    let candidate = canonical_like(host_path);
    let workspace = canonical_like(&roots.workspace);
    let artifacts = canonical_like(&roots.artifacts);
    let temp = canonical_like(&roots.temp);
    if candidate.starts_with(&workspace)
        || candidate.starts_with(&artifacts)
        || candidate.starts_with(&temp)
    {
        Ok(())
    } else {
        Err(format!(
            "path escapes sandbox roots: {}",
            candidate.to_string_lossy()
        ))
    }
}

fn canonical_like(value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    path.canonicalize().unwrap_or(path)
}

fn normalize_tool_path(value: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    trimmed.replace('\\', "/")
}

fn resolve_virtual_root(
    root_path: &Path,
    root_label: &str,
    suffix: &str,
) -> Result<ResolvedExecutionPath, ApiError> {
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
) -> ResolvedExecutionPath {
    let suffix = suffix.as_ref().to_string_lossy().replace('\\', "/");
    let display_path = if root.is_empty() {
        host_path.to_string_lossy().replace('\\', "/")
    } else {
        let suffix = suffix.trim_start_matches('/');
        if suffix.is_empty() {
            root.to_string()
        } else {
            format!("{root}/{suffix}")
        }
    };
    ResolvedExecutionPath {
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
                    "path cannot escape the selected execution root".to_string(),
                ));
            }
            Component::RootDir | Component::Prefix(_) => {
                return Err(ApiError::bad_request(
                    "path must stay inside the selected execution root".to_string(),
                ));
            }
        }
    }
    Ok(clean)
}

fn is_probably_host_absolute_path(value: &str) -> bool {
    value.starts_with('/') || value.contains(":\\") || value.contains(":/")
}

#[derive(Debug)]
struct NativeWorkerLaunchResult {
    success: bool,
    message: String,
}

#[cfg(not(windows))]
fn launch_native_worker(
    _agent_id: &str,
    _paths: &AgentExecutionPaths,
    _allow_network: bool,
    _request_path: &str,
    _response_path: &str,
) -> Result<NativeWorkerLaunchResult, String> {
    Err("native sandbox backend is only implemented on Windows in this build".to_string())
}

#[cfg(windows)]
fn launch_native_worker(
    agent_id: &str,
    paths: &AgentExecutionPaths,
    allow_network: bool,
    request_path: &str,
    response_path: &str,
) -> Result<NativeWorkerLaunchResult, String> {
    use std::ptr::{null, null_mut};

    use windows_sys::Win32::Foundation::{CloseHandle, ERROR_SUCCESS, WAIT_TIMEOUT};
    use windows_sys::Win32::Security::Isolation::{
        CreateAppContainerProfile, DeriveAppContainerSidFromAppContainerName,
    };
    use windows_sys::Win32::Security::{PSID, SECURITY_CAPABILITIES};
    use windows_sys::Win32::System::Threading::{
        CreateProcessW, DeleteProcThreadAttributeList, GetExitCodeProcess,
        InitializeProcThreadAttributeList, UpdateProcThreadAttribute, WaitForSingleObject,
        CREATE_NO_WINDOW, EXTENDED_STARTUPINFO_PRESENT, PROCESS_INFORMATION,
        PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES, STARTUPINFOEXW,
    };

    let profile_name = sanitize_profile_name(agent_id);
    let profile_wide = to_wide(&profile_name);
    let mut appcontainer_sid: PSID = null_mut();
    unsafe {
        let _ = CreateAppContainerProfile(
            profile_wide.as_ptr(),
            profile_wide.as_ptr(),
            profile_wide.as_ptr(),
            null(),
            0,
            &mut appcontainer_sid,
        );
        if DeriveAppContainerSidFromAppContainerName(profile_wide.as_ptr(), &mut appcontainer_sid)
            != ERROR_SUCCESS as i32
        {
            return Err("derive AppContainer SID failed".to_string());
        }
    }
    let sid_string = sid_to_string(appcontainer_sid)?;
    ensure_appcontainer_directory_access(&paths.workspace, &sid_string)?;
    ensure_appcontainer_directory_access(&paths.artifacts, &sid_string)?;
    ensure_appcontainer_directory_access(&paths.temp, &sid_string)?;

    let mut capability_buffer = CapabilityBuffer::default();
    if allow_network {
        capability_buffer = CapabilityBuffer::internet_client()?;
    }

    let exe =
        std::env::current_exe().map_err(|error| format!("resolve current exe failed: {error}"))?;
    let current_dir = paths
        .workspace
        .to_str()
        .ok_or_else(|| "workspace path is not valid utf-16".to_string())?;
    let application = to_wide_os(exe.as_os_str());
    let command_line = format!(
        "\"{}\" internal sandbox-worker \"{}\" \"{}\"",
        exe.to_string_lossy(),
        request_path,
        response_path
    );
    let mut command_line_wide = to_wide(&command_line);
    let current_dir_wide = to_wide(current_dir);

    let mut security = SECURITY_CAPABILITIES {
        AppContainerSid: appcontainer_sid,
        Capabilities: capability_buffer.ptr(),
        CapabilityCount: capability_buffer.count(),
        Reserved: 0,
    };

    let mut attr_size = 0usize;
    unsafe {
        let _ = InitializeProcThreadAttributeList(null_mut(), 1, 0, &mut attr_size);
    }
    let mut attr_buffer = vec![0u8; attr_size];
    let attr_list = attr_buffer.as_mut_ptr().cast();
    unsafe {
        if InitializeProcThreadAttributeList(attr_list, 1, 0, &mut attr_size) == 0 {
            return Err(last_os_error("initialize proc thread attribute list"));
        }
        if UpdateProcThreadAttribute(
            attr_list,
            0,
            PROC_THREAD_ATTRIBUTE_SECURITY_CAPABILITIES as usize,
            (&mut security as *mut SECURITY_CAPABILITIES).cast(),
            std::mem::size_of::<SECURITY_CAPABILITIES>(),
            null_mut(),
            null(),
        ) == 0
        {
            DeleteProcThreadAttributeList(attr_list);
            return Err(last_os_error("update proc thread attribute"));
        }
    }

    let mut startup = STARTUPINFOEXW::default();
    startup.StartupInfo.cb = std::mem::size_of::<STARTUPINFOEXW>() as u32;
    startup.lpAttributeList = attr_list;
    let mut process_info = PROCESS_INFORMATION::default();
    let create_result = unsafe {
        CreateProcessW(
            application.as_ptr(),
            command_line_wide.as_mut_ptr(),
            null(),
            null(),
            0,
            EXTENDED_STARTUPINFO_PRESENT | CREATE_NO_WINDOW,
            null(),
            current_dir_wide.as_ptr(),
            (&startup as *const STARTUPINFOEXW).cast(),
            &mut process_info,
        )
    };
    unsafe {
        DeleteProcThreadAttributeList(attr_list);
        let _ = windows_sys::Win32::Security::FreeSid(appcontainer_sid);
    }
    if create_result == 0 {
        return Err(last_os_error("create sandbox worker process"));
    }

    let wait_result = unsafe { WaitForSingleObject(process_info.hProcess, 180_000) };
    if wait_result == WAIT_TIMEOUT {
        unsafe {
            CloseHandle(process_info.hThread);
            CloseHandle(process_info.hProcess);
        }
        return Ok(NativeWorkerLaunchResult {
            success: false,
            message: "sandbox worker timed out".to_string(),
        });
    }

    let mut exit_code = 1u32;
    unsafe {
        let _ = GetExitCodeProcess(process_info.hProcess, &mut exit_code);
        CloseHandle(process_info.hThread);
        CloseHandle(process_info.hProcess);
    }
    Ok(NativeWorkerLaunchResult {
        success: exit_code == 0,
        message: if exit_code == 0 {
            "sandbox worker completed".to_string()
        } else {
            format!("sandbox worker exited with code {exit_code}")
        },
    })
}

#[cfg(windows)]
fn ensure_appcontainer_directory_access(path: &Path, sid: &str) -> Result<(), String> {
    fs::create_dir_all(path)
        .map_err(|error| format!("create sandbox directory failed: {error}"))?;
    let status = std::process::Command::new("icacls")
        .arg(path)
        .arg("/grant")
        .arg(format!("*{sid}:(OI)(CI)F"))
        .arg("/t")
        .arg("/c")
        .arg("/q")
        .status()
        .map_err(|error| format!("spawn icacls failed: {error}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "icacls failed for {} with status {}",
            path.display(),
            status
        ))
    }
}

#[cfg(windows)]
fn sanitize_profile_name(agent_id: &str) -> String {
    let filtered = agent_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("Ennoia.Agent.{filtered}")
}

#[cfg(windows)]
fn to_wide(value: &str) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    std::ffi::OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}

#[cfg(windows)]
fn to_wide_os(value: &std::ffi::OsStr) -> Vec<u16> {
    use std::os::windows::ffi::OsStrExt;

    value.encode_wide().chain(std::iter::once(0)).collect()
}

#[cfg(windows)]
fn sid_to_string(sid: windows_sys::Win32::Security::PSID) -> Result<String, String> {
    use std::slice;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Authorization::ConvertSidToStringSidW;

    let mut raw = std::ptr::null_mut();
    unsafe {
        if ConvertSidToStringSidW(sid, &mut raw) == 0 {
            return Err(last_os_error("convert SID to string"));
        }
        let mut len = 0usize;
        while *raw.add(len) != 0 {
            len += 1;
        }
        let text = String::from_utf16_lossy(slice::from_raw_parts(raw, len));
        let _ = LocalFree(raw.cast());
        Ok(text)
    }
}

#[cfg(windows)]
#[derive(Default)]
struct CapabilityBuffer {
    values: Vec<windows_sys::Win32::Security::SID_AND_ATTRIBUTES>,
    raw_arrays: Vec<*mut std::ffi::c_void>,
}

#[cfg(windows)]
impl CapabilityBuffer {
    fn internet_client() -> Result<Self, String> {
        use std::ptr::null_mut;

        use windows_sys::Win32::Security::{
            DeriveCapabilitySidsFromName, PSID, SID_AND_ATTRIBUTES,
        };

        let capability_name = to_wide("internetClient");
        let mut capability_group_sids: *mut PSID = null_mut();
        let mut capability_group_sid_count = 0u32;
        let mut capability_sids: *mut PSID = null_mut();
        let mut capability_sid_count = 0u32;
        unsafe {
            if DeriveCapabilitySidsFromName(
                capability_name.as_ptr(),
                &mut capability_group_sids,
                &mut capability_group_sid_count,
                &mut capability_sids,
                &mut capability_sid_count,
            ) == 0
            {
                return Err(last_os_error("derive capability SID"));
            }
            let sid_slice = if capability_sids.is_null() || capability_sid_count == 0 {
                &[]
            } else {
                std::slice::from_raw_parts(capability_sids, capability_sid_count as usize)
            };
            let values = sid_slice
                .iter()
                .map(|sid| SID_AND_ATTRIBUTES {
                    Sid: *sid,
                    Attributes: 0,
                })
                .collect::<Vec<_>>();
            let mut raw_arrays = Vec::new();
            if !capability_sids.is_null() {
                raw_arrays.push(capability_sids.cast());
            }
            if !capability_group_sids.is_null() {
                raw_arrays.push(capability_group_sids.cast());
            }
            Ok(Self { values, raw_arrays })
        }
    }

    fn ptr(&mut self) -> *mut windows_sys::Win32::Security::SID_AND_ATTRIBUTES {
        if self.values.is_empty() {
            std::ptr::null_mut()
        } else {
            self.values.as_mut_ptr()
        }
    }

    fn count(&self) -> u32 {
        self.values.len() as u32
    }
}

#[cfg(windows)]
impl Drop for CapabilityBuffer {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::LocalFree;

        unsafe {
            for value in &self.raw_arrays {
                let _ = LocalFree(*value);
            }
        }
    }
}

#[cfg(windows)]
fn last_os_error(context: &str) -> String {
    format!("{context} failed: {}", std::io::Error::last_os_error())
}
