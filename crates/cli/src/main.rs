use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use ennoia_assets::templates;
use ennoia_kernel::{ExtensionManifest, ExtensionSourceMode, ServerConfig};
use ennoia_paths::RuntimePaths;
use ennoia_server::{bootstrap_app_state, default_app_state, run_server, AppState};

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
            auto_attach_workspace_extensions(&paths)?;
            let server_config: ServerConfig = read_toml_or_default(&paths.server_config_file())?;
            ensure_port_available(server_config.port, "API")?;
            ensure_port_available(5173, "Shell")?;
            let mut dev_processes = DevProcessGroup::new();
            dev_processes.start_shell(&paths, &server_config)?;
            dev_processes.start_extension_frontends(&paths)?;
            println!("Ennoia dev runtime starting at {}", paths.home().display());
            println!("Shell: http://127.0.0.1:5173");
            println!("API: http://{}:{}", server_config.host, server_config.port);
            println!("Press Ctrl+C to stop API, Shell and extension dev processes.");
            tokio::select! {
                result = run_server(paths.home()) => {
                    result?;
                }
                signal = tokio::signal::ctrl_c() => {
                    signal?;
                    println!("stopping Ennoia dev runtime...");
                }
            }
        }
        Some("start") | Some("serve") => {
            let paths = RuntimePaths::resolve(args.get(2).map(String::as_str));
            init_home_template(&paths)?;
            auto_attach_workspace_extensions(&paths)?;
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
    auto_attach_workspace_extensions(&paths)?;
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
    let owner = ennoia_kernel::OwnerRef {
        kind: match args[0].as_str() {
            "agent" => ennoia_kernel::OwnerKind::Agent,
            "space" => ennoia_kernel::OwnerKind::Space,
            _ => ennoia_kernel::OwnerKind::Global,
        },
        id: args[1].clone(),
    };
    let request = ennoia_kernel::RememberRequest {
        owner,
        namespace: args[2].clone(),
        memory_kind: ennoia_kernel::MemoryKind::Fact,
        stability: ennoia_kernel::Stability::Working,
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
    let owner = ennoia_kernel::OwnerRef {
        kind: match args[0].as_str() {
            "agent" => ennoia_kernel::OwnerKind::Agent,
            "space" => ennoia_kernel::OwnerKind::Space,
            _ => ennoia_kernel::OwnerKind::Global,
        },
        id: args[1].clone(),
    };
    let query_text = if args.len() > 2 {
        Some(args[2..].join(" "))
    } else {
        None
    };
    let mode = if query_text.is_some() {
        ennoia_kernel::RecallMode::Fts
    } else {
        ennoia_kernel::RecallMode::Namespace
    };
    let query = ennoia_kernel::RecallQuery {
        owner,
        thread_id: None,
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
            let attached = state.extensions.attach_workspace(path)?;
            println!("{}", serde_json::to_string_pretty(&attached)?);
        }
        "detach" => {
            let id = args.get(1).ok_or("usage: ennoia ext detach <id>")?;
            let detached = state.extensions.detach_workspace(id)?;
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

    fn start_shell(
        &mut self,
        paths: &RuntimePaths,
        server_config: &ServerConfig,
    ) -> io::Result<()> {
        let shell_dir = env::current_dir()?.join("web/apps/shell");
        if !shell_dir.join("package.json").exists() {
            println!("Shell dev server skipped: web/apps/shell/package.json not found");
            return Ok(());
        }

        let log_path = paths.server_logs_dir().join("shell-dev.log");
        let mut command = shell_command(
            "bun run dev --host 127.0.0.1 --port 5173 --strictPort",
            &shell_dir,
        );
        command.env(
            "VITE_ENNOIA_API_URL",
            format!("http://{}:{}", server_config.host, server_config.port),
        );
        self.spawn("shell", command, &log_path)
    }

    fn start_extension_frontends(&mut self, paths: &RuntimePaths) -> io::Result<()> {
        for workspace in attached_workspace_roots(paths)? {
            let Some(descriptor_path) = descriptor_path(&workspace) else {
                continue;
            };
            let contents = fs::read_to_string(descriptor_path)?;
            let manifest: ExtensionManifest =
                toml::from_str(&contents).map_err(io::Error::other)?;
            if manifest.source.mode != ExtensionSourceMode::Workspace {
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
            let command = shell_command(&dev_command, &workspace);
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

fn attached_workspace_roots(paths: &RuntimePaths) -> io::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    if paths.attached_workspaces_file().exists() {
        let contents = fs::read_to_string(paths.attached_workspaces_file())?;
        let value: toml::Value = toml::from_str(&contents).map_err(io::Error::other)?;
        if let Some(items) = value.get("workspaces").and_then(toml::Value::as_array) {
            roots.extend(items.iter().filter_map(|item| {
                item.get("path")
                    .and_then(toml::Value::as_str)
                    .map(PathBuf::from)
            }));
        }
    }

    let repo_extensions = env::current_dir()?.join("extensions");
    if repo_extensions.exists() {
        for entry in fs::read_dir(repo_extensions)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() && entry.path().join("ennoia.extension.toml").exists() {
                roots.push(entry.path());
            }
        }
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

fn auto_attach_workspace_extensions(paths: &RuntimePaths) -> io::Result<()> {
    let cwd = env::current_dir()?;
    let extensions_dir = cwd.join("extensions");
    if !extensions_dir.exists() {
        return Ok(());
    }

    let attached_path = paths.attached_workspaces_file();
    let mut current = if attached_path.exists() {
        fs::read_to_string(&attached_path)?
    } else {
        "workspaces = []\n".to_string()
    };

    for entry in fs::read_dir(extensions_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let root = entry.path();
        if !root.join("ennoia.extension.toml").exists() {
            continue;
        }
        let normalized = root.to_string_lossy().replace('\\', "/");
        if current.contains(&normalized) {
            continue;
        }

        let block = format!(
            "workspaces = [{{ id = \"{}\", path = \"{}\" }}]\n",
            entry.file_name().to_string_lossy(),
            normalized
        );
        if current.trim() == "workspaces = []" {
            current = block;
        } else {
            let mut records: toml::Value =
                toml::from_str(&current).unwrap_or_else(|_| toml::Value::Table(Default::default()));
            let table = records
                .as_table_mut()
                .expect("attached workspaces must be table");
            let array = table
                .entry("workspaces")
                .or_insert_with(|| toml::Value::Array(Vec::new()))
                .as_array_mut()
                .expect("workspaces should be array");
            let item = toml::Value::Table(
                [
                    (
                        "id".to_string(),
                        toml::Value::String(entry.file_name().to_string_lossy().to_string()),
                    ),
                    ("path".to_string(), toml::Value::String(normalized)),
                ]
                .into_iter()
                .collect(),
            );
            array.push(item);
            current = toml::to_string_pretty(&records).unwrap_or(current);
        }
    }

    write_if_missing(&attached_path, &current)?;
    if attached_path.exists() {
        fs::write(attached_path, current)?;
    }
    Ok(())
}

fn init_home_template(paths: &RuntimePaths) -> io::Result<()> {
    paths.ensure_layout()?;
    fs::create_dir_all(paths.global_extension_dir("observatory"))?;
    fs::create_dir_all(paths.package_extension_dir("observatory"))?;
    fs::create_dir_all(paths.package_extension_dir("observatory").join("frontend"))?;
    fs::create_dir_all(paths.package_extension_dir("observatory").join("backend"))?;

    write_if_missing(&paths.app_config_file(), templates::app_config())?;
    write_if_missing(&paths.server_config_file(), templates::server_config())?;
    write_if_missing(&paths.ui_config_file(), templates::ui_config())?;
    write_if_missing(
        &paths.agents_config_dir().join("coder.toml"),
        templates::coder_agent(),
    )?;
    write_if_missing(
        &paths.agents_config_dir().join("planner.toml"),
        templates::planner_agent(),
    )?;
    write_if_missing(
        &paths.extensions_config_dir().join("observatory.toml"),
        templates::observatory_extension_config(),
    )?;
    write_if_missing(
        &paths
            .global_extension_dir("observatory")
            .join("manifest.toml"),
        templates::observatory_manifest(),
    )?;
    write_if_missing(
        &paths
            .package_extension_dir("observatory")
            .join("ennoia.extension.toml"),
        templates::observatory_package_descriptor(),
    )?;
    write_if_missing(
        &paths
            .package_extension_dir("observatory")
            .join("frontend")
            .join("index.js"),
        templates::observatory_package_frontend(),
    )?;
    write_if_missing(
        &paths
            .package_extension_dir("observatory")
            .join("backend")
            .join("index.js"),
        templates::observatory_package_backend(),
    )?;
    write_if_missing(
        &paths.attached_workspaces_file(),
        templates::attached_workspaces(),
    )?;
    write_if_missing(
        &paths.policies_dir().join("memory.toml"),
        templates::memory_policy(),
    )?;
    write_if_missing(
        &paths.policies_dir().join("stage.toml"),
        templates::stage_policy(),
    )?;

    Ok(())
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
