use std::collections::HashMap;
use std::fs::{self, OpenOptions};
use std::io::{self, BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStderr, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;
use std::time::SystemTime;

use ennoia_kernel::{
    ExtensionHostCapabilityRequest, ExtensionRpcRequest, ExtensionRpcResponse,
    ProcessWorkerControlMessage,
};
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
const DEFAULT_PROCESS_WORKER_POOL_SIZE: usize = 4;

pub trait HostCapabilityDispatcher: Send + Sync {
    fn dispatch(
        &self,
        extension: &ResolvedExtensionSnapshot,
        request: ExtensionHostCapabilityRequest,
    ) -> io::Result<ExtensionRpcResponse>;
}

#[derive(Debug)]
pub struct WorkerRuntime {
    engine: Engine,
    home_dir: PathBuf,
    logs_dir: PathBuf,
    modules: Mutex<HashMap<String, CachedModule>>,
    processes: Mutex<HashMap<String, Arc<ProcessWorkerPool>>>,
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

#[derive(Debug)]
struct ProcessWorkerSlot {
    handle: Mutex<ProcessWorkerHandle>,
    busy: AtomicBool,
}

impl ProcessWorkerSlot {
    fn new(handle: ProcessWorkerHandle) -> Self {
        Self {
            handle: Mutex::new(handle),
            busy: AtomicBool::new(false),
        }
    }

    fn try_acquire(&self) -> bool {
        self.busy
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }

    fn release(&self) {
        self.busy.store(false, Ordering::Release);
    }
}

#[derive(Debug)]
struct ProcessWorkerLease {
    pool: Arc<ProcessWorkerPool>,
    slot: Arc<ProcessWorkerSlot>,
    discarded: bool,
}

impl ProcessWorkerLease {
    fn slot(&self) -> &Arc<ProcessWorkerSlot> {
        &self.slot
    }

    fn discard(mut self) {
        self.pool.discard_slot(&self.slot);
        self.discarded = true;
    }
}

impl Drop for ProcessWorkerLease {
    fn drop(&mut self) {
        if !self.discarded {
            self.slot.release();
        }
    }
}

#[derive(Debug)]
struct ProcessWorkerPool {
    entry: PathBuf,
    fingerprint: WorkerFingerprint,
    protocol: String,
    max_size: usize,
    slots: Mutex<Vec<Arc<ProcessWorkerSlot>>>,
}

impl ProcessWorkerPool {
    fn new(entry: &Path, fingerprint: WorkerFingerprint, protocol: &str, max_size: usize) -> Self {
        Self {
            entry: entry.to_path_buf(),
            fingerprint,
            protocol: protocol.to_string(),
            max_size: max_size.max(1),
            slots: Mutex::new(Vec::new()),
        }
    }

    fn matches(&self, entry: &Path, fingerprint: &WorkerFingerprint, protocol: &str) -> bool {
        self.entry == entry && self.fingerprint == *fingerprint && self.protocol == protocol
    }

    fn acquire_or_spawn<F>(self: &Arc<Self>, mut spawn: F) -> io::Result<ProcessWorkerLease>
    where
        F: FnMut() -> io::Result<ProcessWorkerHandle>,
    {
        loop {
            let slots = self
                .slots
                .lock()
                .map_err(|_| io::Error::other("worker process pool poisoned"))?
                .clone();

            for slot in &slots {
                if !slot.try_acquire() {
                    continue;
                }
                if self.slot_is_alive(slot)? {
                    return Ok(ProcessWorkerLease {
                        pool: self.clone(),
                        slot: slot.clone(),
                        discarded: false,
                    });
                }
                self.discard_slot(slot);
            }

            if slots.len() < self.max_size {
                let slot = Arc::new(ProcessWorkerSlot::new(spawn()?));
                slot.busy.store(true, Ordering::Release);
                self.slots
                    .lock()
                    .map_err(|_| io::Error::other("worker process pool poisoned"))?
                    .push(slot.clone());
                return Ok(ProcessWorkerLease {
                    pool: self.clone(),
                    slot,
                    discarded: false,
                });
            }

            thread::sleep(Duration::from_millis(5));
        }
    }

    fn shutdown_all(&self) {
        let slots = self
            .slots
            .lock()
            .map(|slots| slots.clone())
            .unwrap_or_default();
        for slot in slots {
            if let Ok(mut handle) = slot.handle.lock() {
                handle.shutdown();
            }
        }
    }

    fn discard_slot(&self, slot: &Arc<ProcessWorkerSlot>) {
        let removed = self.slots.lock().ok().and_then(|mut slots| {
            let index = slots
                .iter()
                .position(|candidate| Arc::ptr_eq(candidate, slot))?;
            Some(slots.remove(index))
        });
        if let Some(slot) = removed {
            if let Ok(mut handle) = slot.handle.lock() {
                handle.shutdown();
            }
        }
    }

    fn slot_is_alive(&self, slot: &Arc<ProcessWorkerSlot>) -> io::Result<bool> {
        let mut handle = slot
            .handle
            .lock()
            .map_err(|_| io::Error::other("worker process handle poisoned"))?;
        Ok(handle.child.try_wait()?.is_none())
    }
}

impl Drop for WorkerRuntime {
    fn drop(&mut self) {
        let pools = self
            .processes
            .lock()
            .map(|processes| processes.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for pool in pools {
            pool.shutdown_all();
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
        host_dispatcher: Option<&dyn HostCapabilityDispatcher>,
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
            "process" => {
                self.dispatch_process(extension, worker, method, request, &entry, host_dispatcher)
            }
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

        let current = self
            .processes
            .lock()
            .map(|processes| {
                processes
                    .iter()
                    .map(|(extension_id, pool)| (extension_id.clone(), pool.clone()))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let mut stale = Vec::new();
        for (extension_id, pool) in current {
            let Some(extension) = extensions.iter().find(|item| item.id == extension_id) else {
                stale.push((extension_id, pool));
                continue;
            };
            let Some(worker) = extension.worker.as_ref() else {
                stale.push((extension_id, pool));
                continue;
            };
            if worker.kind != "process" {
                stale.push((extension_id, pool));
                continue;
            }
            let entry = PathBuf::from(&worker.entry);
            let protocol = worker
                .protocol
                .as_deref()
                .unwrap_or(SUPPORTED_PROCESS_PROTOCOL);
            let Ok(fingerprint) = worker_fingerprint(&entry) else {
                stale.push((extension_id, pool));
                continue;
            };
            if !pool.matches(&entry, &fingerprint, protocol) {
                stale.push((extension_id, pool));
            }
        }
        for (extension_id, pool) in stale {
            self.remove_process_if_current(&extension_id, &pool);
        }
    }

    pub fn terminate_extension(&self, extension_id: &str) {
        if let Ok(mut modules) = self.modules.lock() {
            modules.remove(extension_id);
        }
        self.remove_process(extension_id);
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
        host_dispatcher: Option<&dyn HostCapabilityDispatcher>,
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

        let pool = self.process_pool(extension, entry, protocol)?;
        let lease = pool.acquire_or_spawn(|| self.spawn_process(extension, entry, protocol))?;
        match self.invoke_process(extension, lease.slot(), &payload, host_dispatcher) {
            Ok(response) => Ok(response),
            Err(first_error) => {
                lease.discard();
                let lease =
                    pool.acquire_or_spawn(|| self.spawn_process(extension, entry, protocol))?;
                self.invoke_process(extension, lease.slot(), &payload, host_dispatcher)
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

    fn process_pool(
        &self,
        extension: &ResolvedExtensionSnapshot,
        entry: &Path,
        protocol: &str,
    ) -> io::Result<Arc<ProcessWorkerPool>> {
        let fingerprint = worker_fingerprint(entry)?;
        if let Some(existing) = self
            .processes
            .lock()
            .map_err(|_| io::Error::other("worker process cache poisoned"))?
            .get(&extension.id)
            .cloned()
        {
            if existing.matches(entry, &fingerprint, protocol) {
                return Ok(existing);
            }
            self.remove_process_if_current(&extension.id, &existing);
        }

        let spawned = Arc::new(ProcessWorkerPool::new(
            entry,
            fingerprint,
            protocol,
            DEFAULT_PROCESS_WORKER_POOL_SIZE,
        ));
        let mut processes = self
            .processes
            .lock()
            .map_err(|_| io::Error::other("worker process cache poisoned"))?;
        if let Some(existing) = processes.get(&extension.id).cloned() {
            drop(processes);
            if existing.matches(entry, &worker_fingerprint(entry)?, protocol) {
                return Ok(existing);
            }
            self.remove_process_if_current(&extension.id, &existing);
            let mut processes = self
                .processes
                .lock()
                .map_err(|_| io::Error::other("worker process cache poisoned"))?;
            processes.insert(extension.id.clone(), spawned.clone());
            return Ok(spawned);
        }
        processes.insert(extension.id.clone(), spawned.clone());
        Ok(spawned)
    }

    fn spawn_process(
        &self,
        extension: &ResolvedExtensionSnapshot,
        entry: &Path,
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
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn invoke_process(
        &self,
        extension: &ResolvedExtensionSnapshot,
        slot: &Arc<ProcessWorkerSlot>,
        payload: &[u8],
        host_dispatcher: Option<&dyn HostCapabilityDispatcher>,
    ) -> io::Result<ExtensionRpcResponse> {
        let mut handle = slot
            .handle
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

        loop {
            let mut response = String::new();
            let read = handle.stdout.read_line(&mut response)?;
            if read == 0 {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "process worker closed stdout",
                ));
            }
            let response = response.trim_end_matches(['\r', '\n']);
            if let Some(control) = parse_process_worker_control_message(response)? {
                match control {
                    ProcessWorkerControlMessage::HostCall { call_id, request } => {
                        let Some(host_dispatcher) = host_dispatcher else {
                            write_process_worker_control_message(
                                &mut handle.stdin,
                                &ProcessWorkerControlMessage::HostResult {
                                    call_id,
                                    response: ExtensionRpcResponse::failure(
                                        "host_capability_unavailable",
                                        "host capability dispatcher is unavailable",
                                    ),
                                },
                            )?;
                            continue;
                        };
                        let response =
                            host_dispatcher
                                .dispatch(extension, request)
                                .unwrap_or_else(|error| {
                                    ExtensionRpcResponse::failure(
                                        "host_capability_failed",
                                        error.to_string(),
                                    )
                                });
                        write_process_worker_control_message(
                            &mut handle.stdin,
                            &ProcessWorkerControlMessage::HostResult { call_id, response },
                        )?;
                    }
                    ProcessWorkerControlMessage::HostResult { .. } => {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            "process worker emitted unexpected host result message",
                        ));
                    }
                }
                continue;
            }
            return parse_worker_response(response.as_bytes()).map_err(io::Error::other);
        }
    }

    fn remove_process(&self, extension_id: &str) {
        if let Some(pool) = self
            .processes
            .lock()
            .ok()
            .and_then(|mut processes| processes.remove(extension_id))
        {
            pool.shutdown_all();
        }
    }

    fn remove_process_if_current(&self, extension_id: &str, pool: &Arc<ProcessWorkerPool>) {
        let removed = self.processes.lock().ok().and_then(|mut processes| {
            let current = processes.get(extension_id)?;
            if Arc::ptr_eq(current, pool) {
                processes.remove(extension_id)
            } else {
                None
            }
        });
        if let Some(pool) = removed {
            pool.shutdown_all();
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

fn parse_process_worker_control_message(
    line: &str,
) -> io::Result<Option<ProcessWorkerControlMessage>> {
    let value = match serde_json::from_str::<JsonValue>(line) {
        Ok(value) => value,
        Err(_) => return Ok(None),
    };
    let Some(kind) = value.get("kind").and_then(JsonValue::as_str) else {
        return Ok(None);
    };
    if !matches!(kind, "host_call" | "host_result") {
        return Ok(None);
    }
    serde_json::from_value(value)
        .map(Some)
        .map_err(io::Error::other)
}

fn write_process_worker_control_message(
    stdin: &mut ChildStdin,
    message: &ProcessWorkerControlMessage,
) -> io::Result<()> {
    serde_json::to_writer(&mut *stdin, message).map_err(io::Error::other)?;
    stdin.write_all(b"\n")?;
    stdin.flush()
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
    prefixes.extend(extension.actions.iter().map(|item| item.method.clone()));
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
        ExtensionHealth, ExtensionRuntimeSpec, MemoryContribution, ResolvedWorkerEntry,
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
                None,
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
                None,
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
            .dispatch(
                &extension,
                "memory/ping",
                ExtensionRpcRequest::default(),
                None,
            )
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
            .dispatch(
                &extension,
                "memory/ping",
                ExtensionRpcRequest::default(),
                None,
            )
            .expect("dispatch second");
        assert_eq!(second.data["version"], 2);
        fs::remove_dir_all(&root).expect("cleanup");
    }

    fn test_extension(root: &Path, method_prefix: &str) -> ResolvedExtensionSnapshot {
        ResolvedExtensionSnapshot {
            id: "test".to_string(),
            version: None,
            name: "Test".to_string(),
            description: String::new(),
            docs: None,
            compat: ennoia_kernel::ExtensionCompatSpec::default(),
            conversation: ennoia_kernel::ExtensionConversationSpec::default(),
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
            runtime: ExtensionRuntimeSpec {
                timeout_ms: 1_000,
                memory_limit_mb: 16,
                ..ExtensionRuntimeSpec::default()
            },
            views: Vec::new(),
            operations: Vec::new(),
            events: Vec::new(),
            pages: Vec::new(),
            panels: Vec::new(),
            themes: Vec::new(),
            locales: Vec::new(),
            message_renderers: Vec::new(),
            settings: Vec::new(),
            providers: Vec::new(),
            behaviors: Vec::new(),
            memories: vec![MemoryContribution {
                id: "test".to_string(),
                extension_id: Some("test".to_string()),
                interfaces: Vec::new(),
                entry: Some(method_prefix.to_string()),
            }],
            hooks: Vec::new(),
            actions: Vec::new(),
            schedule_actions: Vec::new(),
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
