use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use ennoia_kernel::{ExtensionRpcRequest, ExtensionRpcResponse};
use serde::Serialize;
use serde_json::Value as JsonValue;
use wasmtime::{
    Config, Engine, Instance, Linker, Memory, Module, Store, StoreLimits, StoreLimitsBuilder,
    TypedFunc,
};

use crate::registry::ResolvedExtensionSnapshot;

const SUPPORTED_WORKER_ABI: &str = "ennoia.worker";
const SUPPORTED_PROCESS_PROTOCOL: &str = "jsonrpc-stdio";
const MAX_RPC_BYTES: usize = 4 * 1024 * 1024;
const FUEL_PER_TIMEOUT_MS: u64 = 10_000;

#[derive(Debug)]
pub struct WorkerRuntime {
    engine: Engine,
    home_dir: PathBuf,
    logs_dir: PathBuf,
    modules: Mutex<HashMap<String, CachedModule>>,
    processes: Mutex<HashMap<String, Arc<Mutex<ProcessWorkerHandle>>>>,
}

#[derive(Debug, Clone)]
struct CachedModule {
    entry: PathBuf,
    fingerprint: WorkerFingerprint,
    module: Module,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkerFingerprint {
    modified: Option<SystemTime>,
    len: u64,
}

#[derive(Debug)]
struct WorkerStore {
    limits: StoreLimits,
}

#[derive(Debug, Serialize)]
struct WorkerInvocation<'a> {
    method: &'a str,
    params: JsonValue,
    context: JsonValue,
}

struct WorkerInstance {
    store: Store<WorkerStore>,
    instance: Instance,
}

#[derive(Debug)]
struct ProcessWorkerHandle {
    entry: PathBuf,
    fingerprint: WorkerFingerprint,
    protocol: String,
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl ProcessWorkerHandle {
    fn shutdown(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
    }
}

impl Drop for WorkerRuntime {
    fn drop(&mut self) {
        if let Ok(processes) = self.processes.lock() {
            for handle in processes.values() {
                if let Ok(mut handle) = handle.lock() {
                    handle.shutdown();
                }
            }
        }
    }
}

impl WorkerRuntime {
    pub fn new(home_dir: PathBuf, logs_dir: PathBuf) -> io::Result<Self> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_backtrace_details(wasmtime::WasmBacktraceDetails::Disable);
        let engine = Engine::new(&config).map_err(io::Error::other)?;
        fs::create_dir_all(&logs_dir)?;

        Ok(Self {
            engine,
            home_dir,
            logs_dir,
            modules: Mutex::new(HashMap::new()),
            processes: Mutex::new(HashMap::new()),
        })
    }

    pub fn dispatch(
        &self,
        extension: &ResolvedExtensionSnapshot,
        method: &str,
        request: ExtensionRpcRequest,
    ) -> io::Result<ExtensionRpcResponse> {
        let Some(worker) = extension.worker.as_ref() else {
            return Ok(ExtensionRpcResponse::failure(
                "worker_missing",
                format!("extension '{}' does not declare a worker", extension.id),
            ));
        };

        if !is_safe_method_name(method) {
            return Ok(ExtensionRpcResponse::failure(
                "rpc_method_invalid",
                "RPC method contains unsupported characters",
            ));
        }

        if !is_method_authorized(extension, method) {
            return Ok(ExtensionRpcResponse::failure(
                "rpc_method_forbidden",
                format!(
                    "method '{method}' is not declared by extension '{}'",
                    extension.id
                ),
            ));
        }

        let entry = PathBuf::from(&worker.entry);
        if let Err(error) = ensure_worker_path(extension, &entry) {
            return Ok(ExtensionRpcResponse::failure(
                "worker_path_forbidden",
                error.to_string(),
            ));
        }

        match worker.kind.as_str() {
            "wasm" => self.dispatch_wasm(extension, worker, method, request, &entry),
            "process" => self.dispatch_process(extension, worker, method, request, &entry),
            _ => Ok(ExtensionRpcResponse::failure(
                "worker_kind_unsupported",
                format!("unsupported worker kind '{}'", worker.kind),
            )),
        }
    }

    pub fn invalidate_missing_or_changed(&self, extensions: &[ResolvedExtensionSnapshot]) {
        if let Ok(mut modules) = self.modules.lock() {
            modules.retain(|extension_id, cached| {
                let Some(extension) = extensions.iter().find(|item| &item.id == extension_id)
                else {
                    return false;
                };
                let Some(worker) = extension.worker.as_ref() else {
                    return false;
                };
                if worker.kind != "wasm" {
                    return false;
                }
                let entry = PathBuf::from(&worker.entry);
                entry == cached.entry
                    && worker_fingerprint(&entry)
                        .map(|fingerprint| fingerprint == cached.fingerprint)
                        .unwrap_or(false)
            });
        }

        if let Ok(mut processes) = self.processes.lock() {
            processes.retain(|extension_id, handle| {
                let Some(extension) = extensions.iter().find(|item| &item.id == extension_id)
                else {
                    if let Ok(mut handle) = handle.lock() {
                        handle.shutdown();
                    }
                    return false;
                };
                let Some(worker) = extension.worker.as_ref() else {
                    if let Ok(mut handle) = handle.lock() {
                        handle.shutdown();
                    }
                    return false;
                };
                if worker.kind != "process" {
                    if let Ok(mut handle) = handle.lock() {
                        handle.shutdown();
                    }
                    return false;
                }
                let entry = PathBuf::from(&worker.entry);
                let protocol = worker
                    .protocol
                    .as_deref()
                    .unwrap_or(SUPPORTED_PROCESS_PROTOCOL)
                    .to_string();
                let Ok(fingerprint) = worker_fingerprint(&entry) else {
                    if let Ok(mut handle) = handle.lock() {
                        handle.shutdown();
                    }
                    return false;
                };
                let Ok(mut process) = handle.lock() else {
                    return false;
                };
                let alive = process
                    .child
                    .try_wait()
                    .map(|status| status.is_none())
                    .unwrap_or(false);
                let keep = alive
                    && process.entry == entry
                    && process.fingerprint == fingerprint
                    && process.protocol == protocol;
                if !keep {
                    process.shutdown();
                }
                keep
            });
        }
    }

    fn dispatch_wasm(
        &self,
        extension: &ResolvedExtensionSnapshot,
        worker: &ennoia_kernel::ResolvedWorkerEntry,
        method: &str,
        request: ExtensionRpcRequest,
        entry: &Path,
    ) -> io::Result<ExtensionRpcResponse> {
        if worker.abi != SUPPORTED_WORKER_ABI {
            return Ok(ExtensionRpcResponse::failure(
                "worker_abi_unsupported",
                format!("unsupported worker ABI '{}'", worker.abi),
            ));
        }

        let module = match self.load_module(&extension.id, entry) {
            Ok(module) => module,
            Err(error) => {
                return Ok(ExtensionRpcResponse::failure(
                    "worker_module_unavailable",
                    error.to_string(),
                ))
            }
        };

        let payload = WorkerInvocation {
            method,
            params: request.params,
            context: request.context,
        };
        let payload = match serde_json::to_vec(&payload) {
            Ok(payload) if payload.len() <= MAX_RPC_BYTES => payload,
            Ok(_) => {
                return Ok(ExtensionRpcResponse::failure(
                    "rpc_payload_too_large",
                    format!("RPC payload exceeds {} bytes", MAX_RPC_BYTES),
                ))
            }
            Err(error) => return Err(io::Error::other(error)),
        };

        match self.invoke_wasm(extension, &module, &payload) {
            Ok(response) => Ok(response),
            Err(error) => Ok(ExtensionRpcResponse::failure(
                "worker_execution_failed",
                error.to_string(),
            )),
        }
    }

    fn dispatch_process(
        &self,
        extension: &ResolvedExtensionSnapshot,
        worker: &ennoia_kernel::ResolvedWorkerEntry,
        method: &str,
        request: ExtensionRpcRequest,
        entry: &Path,
    ) -> io::Result<ExtensionRpcResponse> {
        let protocol = worker
            .protocol
            .as_deref()
            .unwrap_or(SUPPORTED_PROCESS_PROTOCOL);
        if protocol != SUPPORTED_PROCESS_PROTOCOL {
            return Ok(ExtensionRpcResponse::failure(
                "worker_protocol_unsupported",
                format!("unsupported process worker protocol '{protocol}'"),
            ));
        }

        let payload = WorkerInvocation {
            method,
            params: request.params,
            context: request.context,
        };
        let payload = match serde_json::to_vec(&payload) {
            Ok(payload) if payload.len() <= MAX_RPC_BYTES => payload,
            Ok(_) => {
                return Ok(ExtensionRpcResponse::failure(
                    "rpc_payload_too_large",
                    format!("RPC payload exceeds {} bytes", MAX_RPC_BYTES),
                ))
            }
            Err(error) => return Err(io::Error::other(error)),
        };

        let handle = self.process_handle(extension, entry, protocol)?;
        match self.invoke_process(&handle, &payload) {
            Ok(response) => Ok(response),
            Err(first_error) => {
                self.remove_process(&extension.id);
                let handle = self.process_handle(extension, entry, protocol)?;
                self.invoke_process(&handle, &payload)
                    .map_err(|second_error| {
                        io::Error::other(format!(
                        "process worker failed after restart; first error: {}; second error: {}",
                        first_error, second_error
                    ))
                    })
            }
        }
    }

    fn load_module(&self, extension_id: &str, entry: &Path) -> io::Result<Module> {
        let fingerprint = worker_fingerprint(entry)?;
        let mut modules = self
            .modules
            .lock()
            .map_err(|_| io::Error::other("worker module cache poisoned"))?;

        if let Some(cached) = modules.get(extension_id) {
            if cached.entry == entry && cached.fingerprint == fingerprint {
                return Ok(cached.module.clone());
            }
        }

        let bytes = fs::read(entry)?;
        let module = Module::from_binary(&self.engine, &bytes).map_err(io::Error::other)?;
        if let Some(import) = module.imports().next() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                format!(
                    "worker import denied: {}.{}",
                    import.module(),
                    import.name()
                ),
            ));
        }

        modules.insert(
            extension_id.to_string(),
            CachedModule {
                entry: entry.to_path_buf(),
                fingerprint,
                module: module.clone(),
            },
        );
        Ok(module)
    }

    fn invoke_wasm(
        &self,
        extension: &ResolvedExtensionSnapshot,
        module: &Module,
        payload: &[u8],
    ) -> anyhow::Result<ExtensionRpcResponse> {
        let mut instance = self.instantiate(extension, module)?;
        let memory = required_memory(&mut instance)?;
        let alloc = required_func::<i32, i32>(&mut instance, "ennoia_worker_alloc")?;
        let dealloc = instance
            .instance
            .get_typed_func::<(i32, i32), ()>(&mut instance.store, "ennoia_worker_dealloc")?;
        let handle = required_func::<(i32, i32), i64>(&mut instance, "ennoia_worker_handle")?;

        let request_ptr = alloc.call(&mut instance.store, payload.len() as i32)?;
        if request_ptr < 0 {
            anyhow::bail!("worker allocator returned a negative pointer");
        }
        memory.write(&mut instance.store, request_ptr as usize, payload)?;

        let packed = handle.call(&mut instance.store, (request_ptr, payload.len() as i32))?;
        let _ = dealloc.call(&mut instance.store, (request_ptr, payload.len() as i32));

        let (response_ptr, response_len) = unpack_ptr_len(packed)?;
        if response_len > MAX_RPC_BYTES {
            anyhow::bail!("worker response exceeds {MAX_RPC_BYTES} bytes");
        }

        let mut response = vec![0_u8; response_len];
        memory.read(&instance.store, response_ptr, &mut response)?;
        let _ = dealloc.call(
            &mut instance.store,
            (response_ptr as i32, response_len as i32),
        );

        parse_worker_response(&response)
    }

    fn instantiate(
        &self,
        extension: &ResolvedExtensionSnapshot,
        module: &Module,
    ) -> anyhow::Result<WorkerInstance> {
        let limit_bytes = extension.runtime.memory_limit_mb as usize * 1024 * 1024;
        let store_data = WorkerStore {
            limits: StoreLimitsBuilder::new().memory_size(limit_bytes).build(),
        };
        let mut store = Store::new(&self.engine, store_data);
        store.limiter(|data| &mut data.limits);
        let fuel = extension
            .runtime
            .timeout_ms
            .saturating_mul(FUEL_PER_TIMEOUT_MS)
            .max(FUEL_PER_TIMEOUT_MS);
        store.set_fuel(fuel)?;

        let linker = Linker::new(&self.engine);
        let instance = linker.instantiate(&mut store, module)?;
        Ok(WorkerInstance { store, instance })
    }

    fn process_handle(
        &self,
        extension: &ResolvedExtensionSnapshot,
        entry: &Path,
        protocol: &str,
    ) -> io::Result<Arc<Mutex<ProcessWorkerHandle>>> {
        let fingerprint = worker_fingerprint(entry)?;
        let mut processes = self
            .processes
            .lock()
            .map_err(|_| io::Error::other("worker process cache poisoned"))?;

        if let Some(existing) = processes.get(&extension.id) {
            let existing = existing.clone();
            let mut handle = existing
                .lock()
                .map_err(|_| io::Error::other("worker process handle poisoned"))?;
            let alive = handle.child.try_wait()?.is_none();
            if alive
                && handle.entry == entry
                && handle.fingerprint == fingerprint
                && handle.protocol == protocol
            {
                drop(handle);
                return Ok(existing);
            }
            handle.shutdown();
        }

        let spawned = Arc::new(Mutex::new(self.spawn_process(
            extension,
            entry,
            fingerprint,
            protocol,
        )?));
        processes.insert(extension.id.clone(), spawned.clone());
        Ok(spawned)
    }

    fn spawn_process(
        &self,
        extension: &ResolvedExtensionSnapshot,
        entry: &Path,
        fingerprint: WorkerFingerprint,
        protocol: &str,
    ) -> io::Result<ProcessWorkerHandle> {
        let log_path = self
            .logs_dir
            .join(format!("{}.process.log", extension.id.replace('/', "_")));
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let mut command = Command::new(entry);
        command
            .current_dir(PathBuf::from(&extension.source_root))
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .env("ENNOIA_HOME", &self.home_dir)
            .env("ENNOIA_EXTENSION_ID", &extension.id)
            .env("ENNOIA_EXTENSION_ROOT", &extension.source_root)
            .env("ENNOIA_EXTENSION_INSTALL_DIR", &extension.install_dir)
            .env(
                "ENNOIA_EXTENSION_DATA_DIR",
                self.home_dir
                    .join("data")
                    .join("extensions")
                    .join(&extension.id),
            )
            .env("ENNOIA_EXTENSION_LOG_DIR", &self.logs_dir)
            .env("ENNOIA_WORKER_PROTOCOL", protocol);

        let mut child = command.spawn()?;
        let stdin = child
            .stdin
            .take()
            .ok_or_else(|| io::Error::other("process worker missing stdin"))?;
        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| io::Error::other("process worker missing stdout"))?;
        let stderr = child
            .stderr
            .take()
            .ok_or_else(|| io::Error::other("process worker missing stderr"))?;
        spawn_process_log_pump(stderr, log_path);

        Ok(ProcessWorkerHandle {
            entry: entry.to_path_buf(),
            fingerprint,
            protocol: protocol.to_string(),
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn invoke_process(
        &self,
        handle: &Arc<Mutex<ProcessWorkerHandle>>,
        payload: &[u8],
    ) -> io::Result<ExtensionRpcResponse> {
        let mut handle = handle
            .lock()
            .map_err(|_| io::Error::other("worker process handle poisoned"))?;
        if handle.child.try_wait()?.is_some() {
            return Err(io::Error::new(
                io::ErrorKind::BrokenPipe,
                "process worker exited before handling the request",
            ));
        }

        handle.stdin.write_all(payload)?;
        handle.stdin.write_all(b"\n")?;
        handle.stdin.flush()?;

        let mut response = String::new();
        let read = handle.stdout.read_line(&mut response)?;
        if read == 0 {
            return Err(io::Error::new(
                io::ErrorKind::UnexpectedEof,
                "process worker closed stdout",
            ));
        }
        parse_worker_response(response.trim_end_matches(['\r', '\n']).as_bytes())
            .map_err(io::Error::other)
    }

    fn remove_process(&self, extension_id: &str) {
        if let Ok(mut processes) = self.processes.lock() {
            if let Some(handle) = processes.remove(extension_id) {
                if let Ok(mut handle) = handle.lock() {
                    handle.shutdown();
                }
            }
        }
    }
}

fn required_memory(instance: &mut WorkerInstance) -> anyhow::Result<Memory> {
    instance
        .instance
        .get_memory(&mut instance.store, "memory")
        .ok_or_else(|| anyhow::anyhow!("worker must export memory"))
}

fn required_func<Params, Results>(
    instance: &mut WorkerInstance,
    name: &str,
) -> anyhow::Result<TypedFunc<Params, Results>>
where
    Params: wasmtime::WasmParams,
    Results: wasmtime::WasmResults,
{
    instance
        .instance
        .get_typed_func::<Params, Results>(&mut instance.store, name)
        .map_err(Into::into)
}

fn parse_worker_response(bytes: &[u8]) -> anyhow::Result<ExtensionRpcResponse> {
    if let Ok(response) = serde_json::from_slice::<ExtensionRpcResponse>(bytes) {
        return Ok(response);
    }
    let data = serde_json::from_slice::<JsonValue>(bytes)?;
    Ok(ExtensionRpcResponse::success(data))
}

fn unpack_ptr_len(packed: i64) -> anyhow::Result<(usize, usize)> {
    let packed = packed as u64;
    let ptr = (packed >> 32) as usize;
    let len = (packed & 0xffff_ffff) as usize;
    if len == 0 {
        anyhow::bail!("worker returned an empty response");
    }
    Ok((ptr, len))
}

fn worker_fingerprint(entry: &Path) -> io::Result<WorkerFingerprint> {
    let metadata = fs::metadata(entry)?;
    Ok(WorkerFingerprint {
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

fn ensure_worker_path(extension: &ResolvedExtensionSnapshot, entry: &Path) -> io::Result<()> {
    let root = canonicalize_or_original(Path::new(&extension.source_root));
    let entry = canonicalize_or_original(entry);
    if !entry.starts_with(&root) {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "worker entry must stay inside the extension root",
        ));
    }
    Ok(())
}

fn canonicalize_or_original(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn is_safe_method_name(method: &str) -> bool {
    !method.is_empty()
        && method.len() <= 256
        && !method.contains("..")
        && method
            .chars()
            .all(|item| item.is_ascii_alphanumeric() || matches!(item, '_' | '-' | '.' | '/' | ':'))
}

fn is_method_authorized(extension: &ResolvedExtensionSnapshot, method: &str) -> bool {
    let prefixes = method_prefixes(extension);
    prefixes.is_empty()
        || prefixes
            .iter()
            .any(|prefix| method == prefix || method.starts_with(&format!("{prefix}/")))
}

fn method_prefixes(extension: &ResolvedExtensionSnapshot) -> Vec<String> {
    let mut prefixes = Vec::new();
    prefixes.extend(
        extension
            .providers
            .iter()
            .filter_map(|item| item.entry.clone()),
    );
    prefixes.extend(
        extension
            .behaviors
            .iter()
            .filter_map(|item| item.entry.clone()),
    );
    prefixes.extend(
        extension
            .memories
            .iter()
            .filter_map(|item| item.entry.clone()),
    );
    prefixes.extend(
        extension
            .hooks
            .iter()
            .filter_map(|item| item.handler.clone()),
    );
    prefixes.extend(extension.interfaces.iter().map(|item| item.method.clone()));
    prefixes.extend(
        extension
            .schedule_actions
            .iter()
            .map(|item| item.method.clone()),
    );
    prefixes.sort();
    prefixes.dedup();
    prefixes
}

fn spawn_process_log_pump(stderr: ChildStderr, log_path: PathBuf) {
    thread::spawn(move || {
        let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) else {
            return;
        };
        for line in BufReader::new(stderr).lines() {
            let Ok(line) = line else {
                break;
            };
            let _ = writeln!(file, "{line}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::{
        ExtensionCapabilities, ExtensionHealth, ExtensionKind, ExtensionPermissionSpec,
        ExtensionRuntimeSpec, MemoryContribution, ResolvedWorkerEntry,
    };

    #[test]
    fn dispatches_wasm_worker_rpc() {
        let root = unique_test_dir("wasm-dispatch");
        fs::create_dir_all(root.join("worker")).expect("create worker dir");
        let wasm = wat::parse_str(echo_worker_wat(r#"{"ok":true,"data":{"pong":true}}"#))
            .expect("compile wat");
        fs::write(root.join("worker/plugin.wasm"), wasm).expect("write wasm");

        let runtime = WorkerRuntime::new(root.join("home"), root.join("logs")).expect("runtime");
        let response = runtime
            .dispatch(
                &test_extension(&root, "memory"),
                "memory/ping",
                ExtensionRpcRequest::default(),
            )
            .expect("dispatch");

        assert!(response.ok);
        assert_eq!(response.data["pong"], true);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn rejects_undeclared_rpc_method() {
        let root = unique_test_dir("wasm-forbidden");
        fs::create_dir_all(root.join("worker")).expect("create worker dir");
        fs::write(
            root.join("worker/plugin.wasm"),
            wat::parse_str(echo_worker_wat(r#"{"ok":true,"data":{}}"#)).expect("compile wat"),
        )
        .expect("write wasm");

        let runtime = WorkerRuntime::new(root.join("home"), root.join("logs")).expect("runtime");
        let response = runtime
            .dispatch(
                &test_extension(&root, "memory"),
                "other/ping",
                ExtensionRpcRequest::default(),
            )
            .expect("dispatch");

        assert!(!response.ok);
        assert_eq!(response.error.expect("error").code, "rpc_method_forbidden");
        fs::remove_dir_all(&root).expect("cleanup");
    }

    #[test]
    fn recompiles_worker_when_wasm_changes() {
        let root = unique_test_dir("wasm-reload");
        fs::create_dir_all(root.join("worker")).expect("create worker dir");
        let wasm_path = root.join("worker/plugin.wasm");
        fs::write(
            &wasm_path,
            wat::parse_str(echo_worker_wat(r#"{"ok":true,"data":{"version":1}}"#))
                .expect("compile wat"),
        )
        .expect("write wasm");

        let runtime = WorkerRuntime::new(root.join("home"), root.join("logs")).expect("runtime");
        let extension = test_extension(&root, "memory");
        let first = runtime
            .dispatch(&extension, "memory/ping", ExtensionRpcRequest::default())
            .expect("dispatch first");
        assert_eq!(first.data["version"], 1);

        std::thread::sleep(std::time::Duration::from_millis(20));
        fs::write(
            &wasm_path,
            wat::parse_str(echo_worker_wat(r#"{"ok":true,"data":{"version":2}}"#))
                .expect("compile wat"),
        )
        .expect("rewrite wasm");

        let second = runtime
            .dispatch(&extension, "memory/ping", ExtensionRpcRequest::default())
            .expect("dispatch second");
        assert_eq!(second.data["version"], 2);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    fn test_extension(root: &Path, method_prefix: &str) -> ResolvedExtensionSnapshot {
        ResolvedExtensionSnapshot {
            id: "test".to_string(),
            name: "Test".to_string(),
            description: String::new(),
            docs: None,
            conversation: ennoia_kernel::ExtensionConversationSpec::default(),
            kind: ExtensionKind::SystemExtension,
            source_mode: ennoia_kernel::ExtensionSourceMode::Dev,
            source_root: root.to_string_lossy().replace('\\', "/"),
            install_dir: root.to_string_lossy().replace('\\', "/"),
            generation: 1,
            health: ExtensionHealth::Ready,
            ui: None,
            worker: Some(ResolvedWorkerEntry {
                kind: "wasm".to_string(),
                entry: root
                    .join("worker/plugin.wasm")
                    .to_string_lossy()
                    .replace('\\', "/"),
                abi: SUPPORTED_WORKER_ABI.to_string(),
                protocol: None,
                status: "ready".to_string(),
            }),
            permissions: ExtensionPermissionSpec::default(),
            runtime: ExtensionRuntimeSpec {
                timeout_ms: 1_000,
                memory_limit_mb: 16,
                ..ExtensionRuntimeSpec::default()
            },
            capabilities: ExtensionCapabilities::default(),
            resource_types: Vec::new(),
            capability_rows: Vec::new(),
            surfaces: Vec::new(),
            pages: Vec::new(),
            panels: Vec::new(),
            themes: Vec::new(),
            locales: Vec::new(),
            commands: Vec::new(),
            providers: Vec::new(),
            behaviors: Vec::new(),
            memories: vec![MemoryContribution {
                id: "test".to_string(),
                extension_id: Some("test".to_string()),
                interfaces: Vec::new(),
                entry: Some(method_prefix.to_string()),
            }],
            hooks: Vec::new(),
            interfaces: Vec::new(),
            schedule_actions: Vec::new(),
            subscriptions: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    fn echo_worker_wat(response: &str) -> String {
        format!(
            r#"
(module
  (memory (export "memory") 1)
  (global $heap (mut i32) (i32.const 2048))
  (data (i32.const 1024) "{escaped}")
  (func (export "ennoia_worker_alloc") (param $len i32) (result i32)
    (local $ptr i32)
    global.get $heap
    local.set $ptr
    global.get $heap
    local.get $len
    i32.add
    global.set $heap
    local.get $ptr)
  (func (export "ennoia_worker_dealloc") (param i32) (param i32))
  (func (export "ennoia_worker_handle") (param i32) (param i32) (result i64)
    i64.const 1024
    i64.const 32
    i64.shl
    i64.const {len}
    i64.or))
"#,
            escaped = response.replace('\\', "\\\\").replace('"', "\\\""),
            len = response.len(),
        )
    }

    fn unique_test_dir(prefix: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "ennoia-{prefix}-{}",
            SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("time")
                .as_nanos()
        ))
    }
}
