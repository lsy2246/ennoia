use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ennoia_assets::{builtins, templates};
use ennoia_kernel::{
    ExtensionManifest, ExtensionRegistryEntry, ExtensionRegistryFile, ExtensionSourceMode,
    OwnerKind, OwnerRef, ServerConfig, SkillRegistryEntry, SkillRegistryFile,
};
use ennoia_memory::{MemoryKind, RecallMode, RecallQuery, RememberRequest, Stability};
use ennoia_paths::RuntimePaths;
use ennoia_server::{bootstrap_app_state, default_app_state, run_server, AppState};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};

const WEB_DIR: &str = "web";
const WEB_DEV_HOST: &str = "127.0.0.1";
const WEB_DEV_PORT: u16 = 5173;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("init") => {
            let paths = RuntimePaths::resolve(args.get(2).map(String::as_str));
            init_home_template(&paths)?;
            println!("initialized Ennoia home at {}", paths.home().display());
        }
        Some("print-config") => {
            print_default_config()?;
        }
        Some("dev") => {
            let paths = RuntimePaths::resolve(args.get(2).map(String::as_str));
            init_home_template(&paths)?;
            auto_attach_dev_extensions(&paths)?;
            let server_config: ServerConfig = read_toml_or_default(&paths.server_config_file())?;
            ensure_port_available(server_config.port, "API")?;
            ensure_port_available(WEB_DEV_PORT, "Web")?;
            run_dev_supervisor(paths, server_config).await?;
        }
        Some("start") | Some("serve") => {
            let paths = RuntimePaths::resolve(args.get(2).map(String::as_str));
            init_home_template(&paths)?;
            auto_attach_dev_extensions(&paths)?;
            run_server(paths.home()).await?;
        }
        Some("ext") => {
            extension_command(&args[2..]).await?;
        }
        Some("memory") => {
            memory_command(&args[2..]).await?;
        }
        _ => {
            print_summary();
        }
    }

    Ok(())
}

fn print_summary() {
    let state = default_app_state();
    println!("{} {}", state.overview.app_name, state.app_config.mode);
    println!("modules: {}", state.overview.modules.join(", "));
    println!(
        "server: {}:{}",
        state.server_config.host, state.server_config.port
    );
    println!();
    println!("commands:");
    println!("  ennoia init [home]");
    println!("  ennoia dev [home]");
    println!("  ennoia start [home]");
    println!("  ennoia ext list");
    println!("  ennoia ext inspect <id>");
    println!("  ennoia ext attach <path>");
    println!("  ennoia ext detach <id>");
    println!("  ennoia ext reload <id>");
    println!("  ennoia ext restart <id>");
    println!("  ennoia ext logs [limit]");
    println!("  ennoia ext doctor <id>");
    println!("  ennoia ext graph");
    println!("  ennoia memory list");
    println!("  ennoia memory remember <owner_kind> <owner_id> <namespace> <content>");
    println!("  ennoia memory recall <owner_kind> <owner_id> [query]");
}

fn print_default_config() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = default_app_state();
    println!(
        "[config/ennoia.toml]\n{}",
        toml::to_string_pretty(&state.app_config)?
    );
    println!(
        "\n[config/server.toml]\n{}",
        toml::to_string_pretty(&state.server_config)?
    );
    println!(
        "\n[config/ui.toml]\n{}",
        toml::to_string_pretty(&state.ui_config)?
    );
    Ok(())
}

async fn memory_command(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let paths = RuntimePaths::resolve(None);
    init_home_template(&paths)?;
    auto_attach_dev_extensions(&paths)?;
    let state = bootstrap_app_state(paths.home()).await?;

    let sub = args.first().map(String::as_str).unwrap_or("list");
    match sub {
        "list" => memory_list(&state).await,
        "remember" => memory_remember(&state, &args[1..]).await,
        "recall" => memory_recall(&state, &args[1..]).await,
        other => {
            eprintln!("unknown memory subcommand: {other}");
            std::process::exit(2);
        }
    }
}

async fn memory_list(state: &AppState) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let memories = state.memory_store.list_memories(50).await?;
    for memory in memories {
        println!(
            "{}  {:?}/{}  {}  {}",
            memory.id,
            memory.owner.kind,
            memory.owner.id,
            memory.namespace,
            memory.title.as_deref().unwrap_or(&memory.content)
        );
    }
    Ok(())
}

async fn memory_remember(
    state: &AppState,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.len() < 4 {
        eprintln!("usage: ennoia memory remember <owner_kind> <owner_id> <namespace> <content>");
        std::process::exit(2);
    }
    let owner = OwnerRef {
        kind: match args[0].as_str() {
            "agent" => OwnerKind::Agent,
            "space" => OwnerKind::Space,
            _ => OwnerKind::Global,
        },
        id: args[1].clone(),
    };
    let request = RememberRequest {
        owner,
        namespace: args[2].clone(),
        memory_kind: MemoryKind::Fact,
        stability: Stability::Working,
        title: None,
        content: args[3..].join(" "),
        summary: None,
        confidence: None,
        importance: None,
        valid_from: None,
        valid_to: None,
        sources: Vec::new(),
        tags: Vec::new(),
        entities: Vec::new(),
    };
    let receipt = state.memory_store.remember(request).await?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}

async fn memory_recall(
    state: &AppState,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.len() < 2 {
        eprintln!("usage: ennoia memory recall <owner_kind> <owner_id> [query]");
        std::process::exit(2);
    }
    let owner = OwnerRef {
        kind: match args[0].as_str() {
            "agent" => OwnerKind::Agent,
            "space" => OwnerKind::Space,
            _ => OwnerKind::Global,
        },
        id: args[1].clone(),
    };
    let query_text = if args.len() > 2 {
        Some(args[2..].join(" "))
    } else {
        None
    };
    let mode = if query_text.is_some() {
        RecallMode::Fts
    } else {
        RecallMode::Namespace
    };
    let query = RecallQuery {
        owner,
        conversation_id: None,
        run_id: None,
        query_text,
        namespace_prefix: None,
        memory_kind: None,
        mode,
        limit: 20,
    };
    let result = state.memory_store.recall(query).await?;
    println!("receipt: {}", result.receipt_id);
    println!("mode: {}", result.mode);
    for memory in result.memories {
        println!(
            "- [{}] {}: {}",
            memory.namespace,
            memory.title.as_deref().unwrap_or("(no title)"),
            memory.content
        );
    }
    Ok(())
}

async fn extension_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let paths = RuntimePaths::resolve(None);
    init_home_template(&paths)?;
    let state = bootstrap_app_state(paths.home()).await?;

    match args.first().map(String::as_str).unwrap_or("list") {
        "list" => {
            for extension in state.extensions.snapshot().extensions {
                println!(
                    "{}\t{}\t{:?}\t{:?}\t{}",
                    extension.id,
                    extension.version,
                    extension.source_mode,
                    extension.health,
                    extension.source_root
                );
            }
        }
        "inspect" | "doctor" => {
            let id = args.get(1).ok_or("usage: ennoia ext inspect <id>")?;
            let extension = state
                .extensions
                .get(id)
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "attach" => {
            let path = args.get(1).ok_or("usage: ennoia ext attach <path>")?;
            let attached = state.extensions.attach_dev_source(path)?;
            println!("{}", serde_json::to_string_pretty(&attached)?);
        }
        "detach" => {
            let id = args.get(1).ok_or("usage: ennoia ext detach <id>")?;
            let detached = state.extensions.detach_dev_source(id)?;
            println!("{}", if detached { "detached" } else { "not-found" });
        }
        "reload" => {
            let id = args.get(1).ok_or("usage: ennoia ext reload <id>")?;
            let extension = state
                .extensions
                .reload_extension(id)?
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "restart" => {
            let id = args.get(1).ok_or("usage: ennoia ext restart <id>")?;
            let extension = state
                .extensions
                .restart_extension(id)?
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "logs" => {
            let limit = args
                .get(1)
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(20);
            for event in state.extensions.events(limit) {
                println!(
                    "{}\t{}\t{}\t{}",
                    event.occurred_at, event.generation, event.event, event.summary
                );
            }
        }
        "graph" => {
            println!(
                "{}",
                serde_json::to_string_pretty(&state.extensions.snapshot())?
            );
        }
        other => {
            eprintln!("unknown ext subcommand: {other}");
            std::process::exit(2);
        }
    }

    Ok(())
}

const BACKEND_RELOAD_DEBOUNCE: Duration = Duration::from_millis(800);
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(250);
const API_READY_TIMEOUT: Duration = Duration::from_secs(15);

async fn run_dev_supervisor(
    paths: RuntimePaths,
    server_config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let repo_root = env::current_dir()?;
    let mut dev_processes = DevProcessGroup::new();
    dev_processes.start_web(&paths, &server_config)?;
    dev_processes.start_extension_frontends(&paths)?;

    let mut api = ApiDevProcess::new(repo_root.clone(), paths.clone(), server_config.clone());
    api.start_initial().await?;

    let (watch_tx, watch_rx) = mpsc::channel();
    let _watcher = start_backend_watcher(&repo_root, watch_tx)?;

    println!("Ennoia dev runtime starting at {}", paths.home().display());
    println!("Web: http://{WEB_DEV_HOST}:{WEB_DEV_PORT}");
    println!("API: http://{}:{}", server_config.host, server_config.port);
    println!("Backend hot reload: watching crates/, assets/, Cargo.toml and Cargo.lock.");
    println!("Press Ctrl+C to stop API, Web and extension dev processes.");

    let mut ticker = tokio::time::interval(WATCH_POLL_INTERVAL);
    let mut pending_backend_change: Option<Instant> = None;

    loop {
        tokio::select! {
            signal = tokio::signal::ctrl_c() => {
                signal?;
                println!("stopping Ennoia dev runtime...");
                break;
            }
            _ = ticker.tick() => {
                let mut saw_change = false;
                while watch_rx.try_recv().is_ok() {
                    saw_change = true;
                }
                if saw_change {
                    pending_backend_change = Some(Instant::now());
                }
                if pending_backend_change
                    .map(|changed_at| changed_at.elapsed() >= BACKEND_RELOAD_DEBOUNCE)
                    .unwrap_or(false)
                {
                    pending_backend_change = None;
                    if let Err(error) = api.rebuild_and_restart().await {
                        eprintln!("backend hot reload failed: {error}");
                    }
                }
            }
        }
    }

    api.stop();
    drop(dev_processes);
    Ok(())
}

struct ApiDevProcess {
    repo_root: PathBuf,
    paths: RuntimePaths,
    server_config: ServerConfig,
    target_dir: PathBuf,
    current: Option<ApiChild>,
}

struct ApiChild {
    snapshot_path: PathBuf,
    child: Child,
}

impl ApiDevProcess {
    fn new(repo_root: PathBuf, paths: RuntimePaths, server_config: ServerConfig) -> Self {
        let target_dir = repo_root.join("target").join("ennoia-dev-api");
        Self {
            repo_root,
            paths,
            server_config,
            target_dir,
            current: None,
        }
    }

    async fn start_initial(&mut self) -> io::Result<()> {
        println!("building API dev binary...");
        let built = self.build_api_binary()?;
        let snapshot = self.stage_api_binary(&built)?;
        self.current = Some(self.launch_snapshot(snapshot).await?);
        println!("started api; log={}", self.api_log_path().display());
        Ok(())
    }

    async fn rebuild_and_restart(&mut self) -> io::Result<()> {
        println!("backend change detected; rebuilding API...");
        let built = match self.build_api_binary() {
            Ok(path) => path,
            Err(error) => {
                eprintln!(
                    "backend build failed; keeping previous API process alive; log={}",
                    self.build_log_path().display()
                );
                return Err(error);
            }
        };
        let snapshot = self.stage_api_binary(&built)?;
        let previous_snapshot = self
            .current
            .as_ref()
            .map(|child| child.snapshot_path.clone());

        if let Some(child) = self.current.as_mut() {
            child.stop();
        }
        self.current = None;

        match self.launch_snapshot(snapshot.clone()).await {
            Ok(child) => {
                self.current = Some(child);
                println!("restarted api from {}", snapshot.display());
                Ok(())
            }
            Err(error) => {
                eprintln!("new API process failed; attempting rollback: {error}");
                if let Some(previous_snapshot) = previous_snapshot {
                    self.current = Some(self.launch_snapshot(previous_snapshot).await?);
                    eprintln!("rolled back to previous API binary");
                }
                Err(error)
            }
        }
    }

    fn stop(&mut self) {
        if let Some(child) = self.current.as_mut() {
            child.stop();
        }
        self.current = None;
    }

    fn build_api_binary(&self) -> io::Result<PathBuf> {
        if let Some(parent) = self.build_log_path().parent() {
            fs::create_dir_all(parent)?;
        }
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.build_log_path())?;
        let stderr = stdout.try_clone()?;
        let status = Command::new("cargo")
            .arg("build")
            .arg("-p")
            .arg("ennoia-cli")
            .env("CARGO_TARGET_DIR", &self.target_dir)
            .current_dir(&self.repo_root)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .status()?;

        if !status.success() {
            return Err(io::Error::other(format!(
                "cargo build -p ennoia-cli failed; log={}",
                self.build_log_path().display()
            )));
        }

        let binary = self.target_dir.join("debug").join(if cfg!(windows) {
            "ennoia.exe"
        } else {
            "ennoia"
        });
        if !binary.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("built API binary not found at {}", binary.display()),
            ));
        }
        Ok(binary)
    }

    fn stage_api_binary(&self, built_binary: &Path) -> io::Result<PathBuf> {
        let dir = self.paths.state_cache_dir().join("dev").join("api-bin");
        fs::create_dir_all(&dir)?;
        let filename = if cfg!(windows) {
            format!("ennoia-api-{}.exe", unique_suffix())
        } else {
            format!("ennoia-api-{}", unique_suffix())
        };
        let snapshot = dir.join(filename);
        fs::copy(built_binary, &snapshot)?;
        Ok(snapshot)
    }

    async fn launch_snapshot(&self, snapshot: PathBuf) -> io::Result<ApiChild> {
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.api_log_path())?;
        let stderr = stdout.try_clone()?;
        let mut child = Command::new(&snapshot)
            .arg("start")
            .arg(self.paths.home())
            .current_dir(&self.repo_root)
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?;

        if let Err(error) = wait_for_api_ready(&self.server_config, API_READY_TIMEOUT).await {
            let _ = child.kill();
            let _ = child.wait();
            return Err(error);
        }

        Ok(ApiChild {
            snapshot_path: snapshot,
            child,
        })
    }

    fn api_log_path(&self) -> PathBuf {
        self.paths.server_logs_dir().join("api-dev.log")
    }

    fn build_log_path(&self) -> PathBuf {
        self.paths.server_logs_dir().join("api-build.log")
    }
}

impl ApiChild {
    fn stop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        println!("stopped api");
    }
}

impl Drop for ApiDevProcess {
    fn drop(&mut self) {
        self.stop();
    }
}

struct DevProcessGroup {
    children: Vec<DevChild>,
}

struct DevChild {
    label: String,
    child: Child,
}

impl DevProcessGroup {
    fn new() -> Self {
        Self {
            children: Vec::new(),
        }
    }

    fn start_web(&mut self, paths: &RuntimePaths, server_config: &ServerConfig) -> io::Result<()> {
        let web_dir = env::current_dir()?.join(WEB_DIR);
        if !web_dir.join("package.json").exists() {
            println!("Web dev server skipped: {WEB_DIR}/package.json not found");
            return Ok(());
        }

        let log_path = paths.server_logs_dir().join("web-dev.log");
        let mut command = shell_command(
            &format!("bun run dev --host {WEB_DEV_HOST} --port {WEB_DEV_PORT} --strictPort"),
            &web_dir,
        );
        command.env(
            "VITE_ENNOIA_API_URL",
            format!("http://{}:{}", server_config.host, server_config.port),
        );
        self.spawn("web", command, &log_path)
    }

    fn start_extension_frontends(&mut self, paths: &RuntimePaths) -> io::Result<()> {
        for source_root in attached_dev_source_roots(paths)? {
            let Some(descriptor_path) = descriptor_path(&source_root) else {
                continue;
            };
            let contents = fs::read_to_string(descriptor_path)?;
            let manifest: ExtensionManifest =
                toml::from_str(&contents).map_err(io::Error::other)?;
            if manifest.source.mode != ExtensionSourceMode::Dev {
                continue;
            }
            let Some(dev_command) = manifest.frontend.dev_command.clone() else {
                if let Some(dev_url) = manifest.frontend.dev_url {
                    println!(
                        "extension {} frontend uses external dev_url: {}",
                        manifest.id, dev_url
                    );
                }
                continue;
            };

            let log_path = paths
                .extensions_logs_dir()
                .join(format!("{}.frontend.log", manifest.id));
            let command = shell_command(&dev_command, &source_root);
            self.spawn(&format!("{} frontend", manifest.id), command, &log_path)?;
        }
        Ok(())
    }

    fn spawn(&mut self, label: &str, mut command: Command, log_path: &Path) -> io::Result<()> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let stdout = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        let stderr = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        let child = command
            .stdout(Stdio::from(stdout))
            .stderr(Stdio::from(stderr))
            .spawn()?;
        println!("started {label}; log={}", log_path.display());
        self.children.push(DevChild {
            label: label.to_string(),
            child,
        });
        Ok(())
    }
}

impl Drop for DevProcessGroup {
    fn drop(&mut self) {
        for child in &mut self.children {
            let _ = child.child.kill();
            let _ = child.child.wait();
            println!("stopped {}", child.label);
        }
    }
}

fn start_backend_watcher(repo_root: &Path, tx: mpsc::Sender<()>) -> io::Result<RecommendedWatcher> {
    let filter_root = repo_root.to_path_buf();
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                if event
                    .paths
                    .iter()
                    .any(|path| is_backend_reload_path(&filter_root, path))
                {
                    let _ = tx.send(());
                }
            }
        },
        NotifyConfig::default(),
    )
    .map_err(io::Error::other)?;

    watch_if_exists(
        &mut watcher,
        &repo_root.join("crates"),
        RecursiveMode::Recursive,
    )?;
    watch_if_exists(
        &mut watcher,
        &repo_root.join("assets"),
        RecursiveMode::Recursive,
    )?;
    watch_if_exists(
        &mut watcher,
        &repo_root.join("Cargo.toml"),
        RecursiveMode::NonRecursive,
    )?;
    watch_if_exists(
        &mut watcher,
        &repo_root.join("Cargo.lock"),
        RecursiveMode::NonRecursive,
    )?;

    Ok(watcher)
}

fn watch_if_exists(
    watcher: &mut RecommendedWatcher,
    path: &Path,
    mode: RecursiveMode,
) -> io::Result<()> {
    if path.exists() {
        watcher.watch(path, mode).map_err(io::Error::other)?;
    }
    Ok(())
}

fn is_backend_reload_path(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    if relative.starts_with("target") || relative.starts_with("web") {
        return false;
    }
    if relative == Path::new("Cargo.toml") || relative == Path::new("Cargo.lock") {
        return true;
    }
    if !(relative.starts_with("crates") || relative.starts_with("assets")) {
        return false;
    }
    match path.extension().and_then(|value| value.to_str()) {
        Some("rs" | "toml" | "sql" | "json" | "js" | "ts" | "css" | "html") => true,
        None => true,
        _ => false,
    }
}

async fn wait_for_api_ready(config: &ServerConfig, timeout: Duration) -> io::Result<()> {
    let started = Instant::now();
    loop {
        let host = config.host.clone();
        let port = config.port;
        if tokio::task::spawn_blocking(move || probe_api_health(&host, port))
            .await
            .unwrap_or(false)
        {
            return Ok(());
        }
        if started.elapsed() >= timeout {
            return Err(io::Error::new(
                io::ErrorKind::TimedOut,
                format!("API did not become ready within {}s", timeout.as_secs()),
            ));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

fn probe_api_health(host: &str, port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect((host, port)) else {
        return false;
    };
    let _ = stream.set_read_timeout(Some(Duration::from_millis(800)));
    let _ = stream.set_write_timeout(Some(Duration::from_millis(800)));
    if stream
        .write_all(format!("GET /health HTTP/1.1\r\nHost: {host}:{port}\r\n\r\n").as_bytes())
        .is_err()
    {
        return false;
    }

    let mut buffer = [0_u8; 128];
    match stream.read(&mut buffer) {
        Ok(count) => String::from_utf8_lossy(&buffer[..count]).contains("200 OK"),
        Err(_) => false,
    }
}

fn shell_command(command: &str, cwd: &Path) -> Command {
    if cfg!(windows) {
        let mut item = Command::new("powershell.exe");
        item.arg("-NoProfile")
            .arg("-NonInteractive")
            .arg("-ExecutionPolicy")
            .arg("Bypass")
            .arg("-Command")
            .arg(command)
            .current_dir(cwd);
        item
    } else {
        let mut item = Command::new("sh");
        item.arg("-lc").arg(command).current_dir(cwd);
        item
    }
}

fn unique_suffix() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    millis.to_string()
}

fn ensure_port_available(port: u16, label: &str) -> io::Result<()> {
    TcpListener::bind(("127.0.0.1", port))
        .map(|listener| drop(listener))
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                format!(
                    "{label} port {port} is already in use; stop the existing process and retry"
                ),
            )
        })
}

fn attached_dev_source_roots(paths: &RuntimePaths) -> io::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let registry = read_extension_registry(paths)?;
    for entry in registry
        .extensions
        .into_iter()
        .filter(|item| item.source == "dev" && item.enabled && !item.removed)
    {
        roots.push(PathBuf::from(entry.path));
    }

    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn descriptor_path(root: &Path) -> Option<PathBuf> {
    [
        root.join("ennoia.extension.toml"),
        root.join("manifest.toml"),
    ]
    .into_iter()
    .find(|path| path.exists())
}

fn auto_attach_dev_extensions(paths: &RuntimePaths) -> io::Result<()> {
    let builtins_dir = env::current_dir()?.join("builtins").join("extensions");
    if !builtins_dir.exists() {
        return Ok(());
    }

    let mut registry = read_extension_registry(paths)?;

    for entry in fs::read_dir(builtins_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let root = entry.path();
        if !root.join("ennoia.extension.toml").exists() {
            continue;
        }
        let normalized = root.to_string_lossy().replace('\\', "/");
        let id = entry.file_name().to_string_lossy().to_string();
        let builtin_removed = registry
            .extensions
            .iter()
            .any(|item| item.id == id && item.source == "builtin" && item.removed);
        if builtin_removed
            || registry
                .extensions
                .iter()
                .any(|item| item.id == id && item.source == "dev")
        {
            continue;
        }
        registry.extensions.push(ExtensionRegistryEntry {
            id,
            source: "dev".to_string(),
            enabled: true,
            removed: false,
            path: normalized,
        });
    }

    sort_extension_registry_entries(&mut registry.extensions);
    write_extension_registry(paths, &registry)?;
    Ok(())
}

fn init_home_template(paths: &RuntimePaths) -> io::Result<()> {
    paths.ensure_layout()?;

    write_if_missing(&paths.app_config_file(), &render_app_config(paths))?;
    write_if_missing(&paths.server_config_file(), templates::server_config())?;
    write_if_missing(&paths.ui_config_file(), templates::ui_config())?;
    sync_builtin_registries(paths)?;
    materialize_builtin_packages(paths)?;
    sync_builtin_provider_presets(paths)?;
    Ok(())
}

fn render_app_config(paths: &RuntimePaths) -> String {
    templates::app_config().replace(
        "sqlite://~/.ennoia/data/sqlite/ennoia.db",
        &format!("sqlite://{}", paths.display_for_user(paths.sqlite_db())),
    )
}

fn sync_builtin_registries(paths: &RuntimePaths) -> io::Result<()> {
    let mut extension_registry = read_extension_registry(paths)?;
    for id in builtin_extension_ids() {
        if let Some(entry) = extension_registry
            .extensions
            .iter_mut()
            .find(|item| item.id == id && item.source == "builtin")
        {
            entry.path = paths.display_for_user(paths.extension_dir(&id));
            continue;
        }
        extension_registry.extensions.push(ExtensionRegistryEntry {
            id: id.clone(),
            source: "builtin".to_string(),
            enabled: true,
            removed: false,
            path: paths.display_for_user(paths.extension_dir(&id)),
        });
    }
    sort_extension_registry_entries(&mut extension_registry.extensions);
    write_extension_registry(paths, &extension_registry)?;

    let mut skill_registry = read_skill_registry(paths)?;
    for id in builtin_skill_ids() {
        if let Some(entry) = skill_registry
            .skills
            .iter_mut()
            .find(|item| item.id == id && item.source == "builtin")
        {
            entry.path = paths.display_for_user(paths.skill_dir(&id));
            continue;
        }
        skill_registry.skills.push(SkillRegistryEntry {
            id: id.clone(),
            source: "builtin".to_string(),
            enabled: true,
            removed: false,
            path: paths.display_for_user(paths.skill_dir(&id)),
        });
    }
    sort_skill_registry_entries(&mut skill_registry.skills);
    write_skill_registry(paths, &skill_registry)
}

fn materialize_builtin_packages(paths: &RuntimePaths) -> io::Result<()> {
    let extension_registry = read_extension_registry(paths)?;
    let skill_registry = read_skill_registry(paths)?;

    for asset in builtins::extensions() {
        let Some(id) = builtin_package_id(asset.logical_path) else {
            continue;
        };
        if is_removed_builtin_extension(&extension_registry, id) {
            continue;
        }
        write_text_asset(paths.home(), asset.logical_path, asset.contents)?;
    }

    for asset in builtins::skills() {
        let Some(id) = builtin_package_id(asset.logical_path) else {
            continue;
        };
        if is_removed_builtin_skill(&skill_registry, id) {
            continue;
        }
        write_text_asset(paths.home(), asset.logical_path, asset.contents)?;
    }

    Ok(())
}

fn sync_builtin_provider_presets(paths: &RuntimePaths) -> io::Result<()> {
    let extension_registry = read_extension_registry(paths)?;

    for entry in extension_registry
        .extensions
        .iter()
        .filter(|item| item.source == "builtin" && item.enabled && !item.removed)
    {
        let root = paths.expand_home_token(&entry.path);
        let presets_dir = root.join("provider-presets");
        if !presets_dir.exists() {
            continue;
        }

        for preset in fs::read_dir(presets_dir)? {
            let preset = preset?;
            if !preset.file_type()?.is_file() {
                continue;
            }
            let destination = paths.providers_config_dir().join(preset.file_name());
            let contents = fs::read_to_string(preset.path())?;
            write_if_missing(&destination, &contents)?;
        }
    }

    Ok(())
}

fn builtin_extension_ids() -> Vec<String> {
    builtin_package_ids_from_assets(builtins::extensions(), "ennoia.extension.toml")
}

fn builtin_skill_ids() -> Vec<String> {
    builtin_package_ids_from_assets(builtins::skills(), "skill.toml")
}

fn builtin_package_ids_from_assets(
    assets: Vec<ennoia_assets::TextAsset>,
    descriptor: &str,
) -> Vec<String> {
    let mut ids = assets
        .into_iter()
        .filter(|asset| asset.logical_path.ends_with(descriptor))
        .filter_map(|asset| builtin_package_id(asset.logical_path).map(str::to_string))
        .collect::<Vec<_>>();
    ids.sort();
    ids.dedup();
    ids
}

fn builtin_package_id(logical_path: &str) -> Option<&str> {
    let mut parts = logical_path.split('/');
    let _kind = parts.next()?;
    parts.next()
}

fn is_removed_builtin_extension(registry: &ExtensionRegistryFile, id: &str) -> bool {
    registry
        .extensions
        .iter()
        .any(|entry| entry.id == id && entry.source == "builtin" && entry.removed)
}

fn is_removed_builtin_skill(registry: &SkillRegistryFile, id: &str) -> bool {
    registry
        .skills
        .iter()
        .any(|entry| entry.id == id && entry.source == "builtin" && entry.removed)
}

fn write_text_asset(root: &Path, logical_path: &str, contents: &str) -> io::Result<()> {
    let path = root.join(logical_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn read_extension_registry(paths: &RuntimePaths) -> io::Result<ExtensionRegistryFile> {
    read_toml_file_or_default(&paths.extensions_registry_file())
}

fn write_extension_registry(
    paths: &RuntimePaths,
    registry: &ExtensionRegistryFile,
) -> io::Result<()> {
    if let Some(parent) = paths.extensions_registry_file().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        paths.extensions_registry_file(),
        toml::to_string_pretty(registry).map_err(io::Error::other)?,
    )
}

fn read_skill_registry(paths: &RuntimePaths) -> io::Result<SkillRegistryFile> {
    read_toml_file_or_default(&paths.skills_registry_file())
}

fn read_toml_file_or_default<T>(path: &Path) -> io::Result<T>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    let contents = fs::read_to_string(path)?;
    toml::from_str(&contents).map_err(io::Error::other)
}

fn write_skill_registry(paths: &RuntimePaths, registry: &SkillRegistryFile) -> io::Result<()> {
    if let Some(parent) = paths.skills_registry_file().parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        paths.skills_registry_file(),
        toml::to_string_pretty(registry).map_err(io::Error::other)?,
    )
}

fn sort_extension_registry_entries(entries: &mut [ExtensionRegistryEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn sort_skill_registry_entries(entries: &mut [SkillRegistryEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn write_if_missing(path: &Path, contents: &str) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, contents)?;
    }

    Ok(())
}

fn read_toml_or_default<T>(path: &Path) -> Result<T, Box<dyn std::error::Error + Send + Sync>>
where
    T: serde::de::DeserializeOwned + Default,
{
    if !path.exists() {
        return Ok(T::default());
    }
    Ok(toml::from_str(&fs::read_to_string(path)?)?)
}
