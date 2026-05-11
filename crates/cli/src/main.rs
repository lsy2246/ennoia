use std::env;
use std::fs;
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::process;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::sync::{mpsc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ennoia_assets::{builtins, templates};
use ennoia_kernel::{
    apply_server_log_env_overrides, ExtensionDevSourceEntry, ExtensionManifest,
    ExtensionRegistryEntry, ExtensionRegistryFile, ExtensionSourceMode, LoggingConfig,
    ServerConfig, SkillRegistryEntry, SkillRegistryFile,
};
use ennoia_paths::RuntimePaths;
use ennoia_server::{bootstrap_app_state, default_app_state, execution, run_server};
use notify::{Config as NotifyConfig, RecommendedWatcher, RecursiveMode, Watcher};

const WEB_DIR: &str = "web";
const ENNOIA_ALLOW_DEV_SOURCES_ENV: &str = "ENNOIA_ALLOW_DEV_SOURCES";
static DEV_CONSOLE_OUTPUT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum ConsoleLogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
struct DevConsoleMirrorConfig {
    enabled: bool,
    min_level: ConsoleLogLevel,
}

impl DevConsoleMirrorConfig {
    fn from_logging(config: &LoggingConfig) -> Self {
        Self {
            enabled: config.dev_console.enabled,
            min_level: ConsoleLogLevel::from_str(&config.dev_console.level),
        }
    }
}

impl ConsoleLogLevel {
    fn from_str(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "debug" => Self::Debug,
            "warn" | "warning" => Self::Warn,
            "error" => Self::Error,
            _ => Self::Info,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Debug => "debug",
            Self::Info => "info",
            Self::Warn => "warn",
            Self::Error => "error",
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None => {
            print_summary();
        }
        Some(flag) if is_help_flag(flag) => {
            print_summary();
        }
        Some("internal") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_internal_usage();
            } else {
                internal_command(&args[1..]).await?;
            }
        }
        Some("init") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_home_command_usage("init");
                return Ok(());
            }
            let paths = RuntimePaths::resolve(parse_optional_home_arg("init", &args[1..])?);
            init_home_template(&paths)?;
            println!("initialized Ennoia home at {}", paths.home().display());
        }
        Some("print-config") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_print_config_usage();
                return Ok(());
            }
            ensure_no_args("print-config", &args[1..], &print_config_usage())?;
            print_default_config()?;
        }
        Some("dev") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_home_command_usage("dev");
                return Ok(());
            }
            let repo_root = env::current_dir()?;
            ensure_no_args("dev", &args[1..], &home_command_usage("dev"))?;
            let paths = RuntimePaths::new(repo_root.join(".dev"));
            init_home_template(&paths)?;
            let mut server_config: ServerConfig =
                read_toml_or_default(&paths.server_config_file())?;
            server_config = server_config.normalize();
            apply_server_log_env_overrides(&mut server_config.logging);
            ensure_port_available(&server_config.host, server_config.port, "API")?;
            ensure_port_available(
                &server_config.web_dev.host,
                server_config.web_dev.port,
                "Web",
            )?;
            ensure_builtin_process_workers(&repo_root)?;
            auto_attach_dev_extensions(&paths)?;
            run_dev_supervisor(paths, server_config).await?;
        }
        Some("start") | Some("serve") => {
            let command = args.first().map(String::as_str).unwrap_or("start");
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_home_command_usage(command);
                return Ok(());
            }
            ensure_no_args(command, &args[1..], &home_command_usage(command))?;
            let paths = RuntimePaths::resolve(None);
            init_home_template(&paths)?;
            let _pid_guard = acquire_pid_file(paths.server_pid_file(), command)?;
            run_server(paths.home()).await?;
        }
        Some("stop") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_stop_usage();
                return Ok(());
            }
            stop_command(&args[1..])?;
        }
        Some("ext") => {
            if args.get(1).is_some_and(|value| is_help_flag(value)) {
                print_extension_usage();
            } else {
                extension_command(&args[1..]).await?;
            }
        }
        Some(other) => {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("unknown command: {other}\n\n{}", summary_text()),
            )
            .into());
        }
    }

    Ok(())
}

fn is_help_flag(value: &str) -> bool {
    matches!(value, "-h" | "--help")
}

fn parse_optional_home_arg<'a>(
    command: &str,
    args: &'a [String],
) -> Result<Option<&'a str>, Box<dyn std::error::Error + Send + Sync>> {
    match args {
        [] => Ok(None),
        [value] if value.starts_with('-') => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "unknown option for 'ennoia {command}': {value}\n\n{}",
                home_command_usage(command)
            ),
        )
        .into()),
        [value] => Ok(Some(value.as_str())),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "too many arguments for 'ennoia {command}'\n\n{}",
                home_command_usage(command)
            ),
        )
        .into()),
    }
}

fn invalid_input_error(message: impl Into<String>) -> Box<dyn std::error::Error + Send + Sync> {
    io::Error::new(io::ErrorKind::InvalidInput, message.into()).into()
}

struct PidFileGuard {
    path: PathBuf,
    pid: u32,
}

impl Drop for PidFileGuard {
    fn drop(&mut self) {
        remove_pid_file_if_matches(&self.path, self.pid);
    }
}

fn acquire_pid_file(path: PathBuf, label: &str) -> io::Result<PidFileGuard> {
    if let Some(existing_pid) = read_pid_file(&path)? {
        if existing_pid != process::id() && is_process_running(existing_pid)? {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!("{label} is already running with pid {existing_pid}"),
            ));
        }
        remove_pid_file_if_matches(&path, existing_pid);
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, format!("{}\n", process::id()))?;
    Ok(PidFileGuard {
        path,
        pid: process::id(),
    })
}

fn stop_command(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match resolve_stop_target(args)? {
        StopTarget::Dev(paths) => {
            stop_process_from_pid_file(&paths.dev_pid_file(), "dev runtime")?;
        }
        StopTarget::Runtime(paths) => {
            stop_process_from_pid_file(&paths.server_pid_file(), "server runtime")?;
        }
    }
    Ok(())
}

enum StopTarget {
    Dev(RuntimePaths),
    Runtime(RuntimePaths),
}

fn resolve_stop_target(
    args: &[String],
) -> Result<StopTarget, Box<dyn std::error::Error + Send + Sync>> {
    match args {
        [] => {
            let cwd = env::current_dir()?;
            let dev_home = cwd.join(".dev");
            if dev_home.exists() {
                Ok(StopTarget::Dev(RuntimePaths::new(dev_home)))
            } else {
                Ok(StopTarget::Runtime(RuntimePaths::resolve(None)))
            }
        }
        [value] if value.starts_with('-') => Err(invalid_input_error(format!(
            "unknown option for 'ennoia stop': {value}\n\n{}",
            stop_usage()
        ))),
        [value] if value == "dev" => {
            let cwd = env::current_dir()?;
            Ok(StopTarget::Dev(RuntimePaths::new(cwd.join(".dev"))))
        }
        [value] => Ok(StopTarget::Runtime(RuntimePaths::new(value))),
        _ => Err(invalid_input_error(format!(
            "too many arguments for 'ennoia stop'\n\n{}",
            stop_usage()
        ))),
    }
}

fn stop_process_from_pid_file(path: &Path, label: &str) -> io::Result<()> {
    let Some(pid) = read_pid_file(path)? else {
        println!("no {label} pid file found at {}", path.display());
        return Ok(());
    };

    if !is_process_running(pid)? {
        remove_pid_file_if_matches(path, pid);
        println!(
            "{label} is not running; removed stale pid file {}",
            path.display()
        );
        return Ok(());
    }

    terminate_process(pid)?;
    wait_for_process_exit(pid, Duration::from_secs(8))?;
    remove_pid_file_if_matches(path, pid);
    println!("stopped {label} (pid {pid})");
    Ok(())
}

fn read_pid_file(path: &Path) -> io::Result<Option<u32>> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = fs::read_to_string(path)?;
    let value = contents
        .trim()
        .parse::<u32>()
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
    Ok(Some(value))
}

fn remove_pid_file_if_matches(path: &Path, pid: u32) {
    match read_pid_file(path) {
        Ok(Some(existing_pid)) if existing_pid == pid => {
            let _ = fs::remove_file(path);
        }
        Ok(None) => {}
        Ok(Some(_)) | Err(_) => {}
    }
}

fn wait_for_process_exit(pid: u32, timeout: Duration) -> io::Result<()> {
    let started = Instant::now();
    while started.elapsed() < timeout {
        if !is_process_running(pid)? {
            return Ok(());
        }
        thread::sleep(Duration::from_millis(120));
    }
    if is_process_running(pid)? {
        #[cfg(not(windows))]
        force_kill_process(pid)?;
        #[cfg(windows)]
        terminate_windows_process_tree(pid)?;
    }
    Ok(())
}

fn terminate_process(pid: u32) -> io::Result<()> {
    #[cfg(windows)]
    {
        terminate_windows_process_tree(pid)
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!("kill -TERM failed for pid {pid}")))
        }
    }
}

#[cfg(not(windows))]
fn force_kill_process(pid: u32) -> io::Result<()> {
    let status = Command::new("kill")
        .args(["-KILL", &pid.to_string()])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!("kill -KILL failed for pid {pid}")))
    }
}

fn is_process_running(pid: u32) -> io::Result<bool> {
    #[cfg(windows)]
    {
        let output = Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/FO", "CSV", "/NH"])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .output()?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        Ok(stdout.contains(&format!("\"{pid}\"")))
    }

    #[cfg(not(windows))]
    {
        let status = Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;
        Ok(status.success())
    }
}

fn ensure_no_args(
    command: &str,
    args: &[String],
    usage: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    match args {
        [] => Ok(()),
        [value] => Err(invalid_input_error(format!(
            "unexpected argument for 'ennoia {command}': {value}\n\n{usage}"
        ))),
        _ => Err(invalid_input_error(format!(
            "too many arguments for 'ennoia {command}'\n\n{usage}"
        ))),
    }
}

fn parse_required_arg<'a>(
    command: &str,
    args: &'a [String],
    usage: &str,
) -> Result<&'a str, Box<dyn std::error::Error + Send + Sync>> {
    match args {
        [value] if value.starts_with('-') => Err(invalid_input_error(format!(
            "unknown option for 'ennoia {command}': {value}\n\n{usage}"
        ))),
        [value] => Ok(value.as_str()),
        [] => Err(invalid_input_error(usage.to_string())),
        _ => Err(invalid_input_error(format!(
            "too many arguments for 'ennoia {command}'\n\n{usage}"
        ))),
    }
}

fn parse_optional_usize_arg(
    command: &str,
    args: &[String],
    usage: &str,
    default: usize,
) -> Result<usize, Box<dyn std::error::Error + Send + Sync>> {
    match args {
        [] => Ok(default),
        [value] if value.starts_with('-') => Err(invalid_input_error(format!(
            "unknown option for 'ennoia {command}': {value}\n\n{usage}"
        ))),
        [value] => value.parse::<usize>().map_err(|_| {
            invalid_input_error(format!(
                "invalid numeric argument for 'ennoia {command}': {value}\n\n{usage}"
            ))
        }),
        _ => Err(invalid_input_error(format!(
            "too many arguments for 'ennoia {command}'\n\n{usage}"
        ))),
    }
}

fn summary_text() -> String {
    let state = default_app_state();
    [
        state.overview.app_name,
        format!("modules: {}", state.overview.modules.join(", ")),
        format!(
            "server: {}:{}",
            state.server_config.host, state.server_config.port
        ),
        String::new(),
        "commands:".to_string(),
        "  ennoia init [home]".to_string(),
        "  ennoia dev".to_string(),
        "  ennoia start".to_string(),
        "  ennoia serve".to_string(),
        "  ennoia stop [home|dev]".to_string(),
        "  ennoia print-config".to_string(),
        "  ennoia ext list".to_string(),
        "  ennoia ext inspect <id>".to_string(),
        "  ennoia ext attach <path>".to_string(),
        "  ennoia ext detach <id>".to_string(),
        "  ennoia ext reload <id>".to_string(),
        "  ennoia ext restart <id>".to_string(),
        "  ennoia ext logs [limit]".to_string(),
        "  ennoia ext doctor <id>".to_string(),
        "  ennoia ext graph".to_string(),
        String::new(),
    ]
    .join("\n")
}

fn print_summary() {
    print!("{}", summary_text());
}

fn home_command_usage(command: &str) -> String {
    if command == "dev" {
        return "usage: ennoia dev\n\nennoia dev always uses the repository-local .dev directory and does not read ENNOIA_HOME.".to_string();
    }
    if command == "start" || command == "serve" {
        return format!(
            "usage: ennoia {command}\n\nennoia {command} resolves the runtime home from ENNOIA_HOME or the default ~/.ennoia directory and does not accept a path argument."
        );
    }
    format!(
        "usage: ennoia {command} [home]\n\nhome is optional. If omitted, ENNOIA_HOME or the default ~/.ennoia directory is used."
    )
}

fn print_home_command_usage(command: &str) {
    println!("{}", home_command_usage(command));
}

fn print_extension_usage() {
    println!("{}", extension_usage());
}

fn print_internal_usage() {
    println!("{}", internal_usage());
}

fn print_print_config_usage() {
    println!("{}", print_config_usage());
}

fn print_stop_usage() {
    println!("{}", stop_usage());
}

fn print_config_usage() -> String {
    "usage: ennoia print-config".to_string()
}

fn stop_usage() -> String {
    [
        "usage: ennoia stop [home|dev]".to_string(),
        String::new(),
        "Without arguments, Ennoia stops the repository-local dev runtime if ./.dev exists;"
            .to_string(),
        "otherwise it stops the runtime resolved from ENNOIA_HOME or ~/.ennoia.".to_string(),
        String::new(),
        "Examples:".to_string(),
        "  ennoia stop".to_string(),
        "  ennoia stop dev".to_string(),
        "  ennoia stop C:/Users/Administrator/.ennoia".to_string(),
    ]
    .join("\n")
}

fn extension_usage() -> String {
    [
        "usage: ennoia ext <subcommand> [args]".to_string(),
        String::new(),
        "subcommands:".to_string(),
        "  list".to_string(),
        "  inspect <id>".to_string(),
        "  attach <path>".to_string(),
        "  detach <id>".to_string(),
        "  reload <id>".to_string(),
        "  restart <id>".to_string(),
        "  logs [limit]".to_string(),
        "  doctor <id>".to_string(),
        "  graph".to_string(),
    ]
    .join("\n")
}

fn extension_subcommand_usage(subcommand: &str) -> String {
    match subcommand {
        "list" => "usage: ennoia ext list".to_string(),
        "inspect" => "usage: ennoia ext inspect <id>".to_string(),
        "attach" => "usage: ennoia ext attach <path>".to_string(),
        "detach" => "usage: ennoia ext detach <id>".to_string(),
        "reload" => "usage: ennoia ext reload <id>".to_string(),
        "restart" => "usage: ennoia ext restart <id>".to_string(),
        "logs" => "usage: ennoia ext logs [limit]".to_string(),
        "doctor" => "usage: ennoia ext doctor <id>".to_string(),
        "graph" => "usage: ennoia ext graph".to_string(),
        _ => extension_usage(),
    }
}

fn internal_usage() -> String {
    [
        "usage: ennoia internal <subcommand> [args]".to_string(),
        String::new(),
        "subcommands:".to_string(),
        "  sandbox-worker <request.json> <response.json>".to_string(),
    ]
    .join("\n")
}

fn internal_subcommand_usage(subcommand: &str) -> String {
    match subcommand {
        "sandbox-worker" => {
            "usage: ennoia internal sandbox-worker <request.json> <response.json>".to_string()
        }
        _ => internal_usage(),
    }
}

fn print_default_config() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut state = default_app_state();
    apply_server_log_env_overrides(&mut state.server_config.logging);
    println!(
        "[config/server.toml]\n{}",
        toml::to_string_pretty(&state.server_config)?
    );
    println!(
        "\n[config/ui.toml]\n{}",
        toml::to_string_pretty(&state.ui_config)?
    );
    Ok(())
}

async fn extension_command(
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.is_empty() {
        print_extension_usage();
        return Ok(());
    }

    let subcommand = args.first().map(String::as_str).unwrap_or_default();
    let subcommand_args = &args[1..];
    if subcommand_args
        .first()
        .is_some_and(|value| is_help_flag(value))
    {
        println!("{}", extension_subcommand_usage(subcommand));
        return Ok(());
    }

    let paths = RuntimePaths::resolve(None);
    init_home_template(&paths)?;
    let state = bootstrap_app_state(paths.home()).await?;

    match subcommand {
        "list" => {
            ensure_no_args(
                "ext list",
                subcommand_args,
                &extension_subcommand_usage("list"),
            )?;
            for extension in state.extensions.snapshot().extensions {
                println!(
                    "{}\t{:?}\t{:?}\t{}",
                    extension.id, extension.source_mode, extension.health, extension.source_root
                );
            }
        }
        "inspect" | "doctor" => {
            let id = parse_required_arg(
                &format!("ext {subcommand}"),
                subcommand_args,
                &extension_subcommand_usage(subcommand),
            )?;
            let extension = state
                .extensions
                .get(id)
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "attach" => {
            let path = parse_required_arg(
                "ext attach",
                subcommand_args,
                &extension_subcommand_usage("attach"),
            )?;
            let attached = state.extensions.attach_dev_source(path)?;
            println!("{}", serde_json::to_string_pretty(&attached)?);
        }
        "detach" => {
            let id = parse_required_arg(
                "ext detach",
                subcommand_args,
                &extension_subcommand_usage("detach"),
            )?;
            let detached = state.extensions.detach_dev_source(id)?;
            println!("{}", if detached { "detached" } else { "not-found" });
        }
        "reload" => {
            let id = parse_required_arg(
                "ext reload",
                subcommand_args,
                &extension_subcommand_usage("reload"),
            )?;
            let extension = state
                .extensions
                .reload_extension(id)?
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "restart" => {
            let id = parse_required_arg(
                "ext restart",
                subcommand_args,
                &extension_subcommand_usage("restart"),
            )?;
            let extension = state
                .extensions
                .restart_extension(id)?
                .ok_or_else(|| format!("extension '{id}' not found"))?;
            println!("{}", serde_json::to_string_pretty(&extension)?);
        }
        "logs" => {
            let limit = parse_optional_usize_arg(
                "ext logs",
                subcommand_args,
                &extension_subcommand_usage("logs"),
                20,
            )?;
            for event in state.extensions.events(limit) {
                println!(
                    "{}\t{}\t{}\t{}",
                    event.occurred_at, event.generation, event.event, event.summary
                );
            }
        }
        "graph" => {
            ensure_no_args(
                "ext graph",
                subcommand_args,
                &extension_subcommand_usage("graph"),
            )?;
            println!(
                "{}",
                serde_json::to_string_pretty(&state.extensions.snapshot())?
            );
        }
        other => {
            return Err(invalid_input_error(format!(
                "unknown ext subcommand: {other}\n\n{}",
                extension_usage()
            )));
        }
    }

    Ok(())
}

async fn internal_command(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.is_empty() {
        print_internal_usage();
        return Ok(());
    }

    let subcommand = args.first().map(String::as_str).unwrap_or_default();
    let subcommand_args = &args[1..];
    if subcommand_args
        .first()
        .is_some_and(|value| is_help_flag(value))
    {
        println!("{}", internal_subcommand_usage(subcommand));
        return Ok(());
    }

    match subcommand {
        "sandbox-worker" => {
            let usage = internal_subcommand_usage("sandbox-worker");
            let (request_path, response_path) = match subcommand_args {
                [request_path, response_path] => (request_path.as_str(), response_path.as_str()),
                [] | [_] => return Err(invalid_input_error(usage)),
                [value, ..] if value.starts_with('-') => {
                    return Err(invalid_input_error(format!(
                        "unknown option for 'ennoia internal sandbox-worker': {value}\n\n{usage}"
                    )))
                }
                _ => {
                    return Err(invalid_input_error(format!(
                        "too many arguments for 'ennoia internal sandbox-worker'\n\n{usage}"
                    )))
                }
            };
            execution::run_sandbox_worker(request_path, response_path)
                .await
                .map_err(io::Error::other)?;
        }
        other => {
            return Err(invalid_input_error(format!(
                "unknown internal subcommand: {other}\n\n{}",
                internal_usage()
            )));
        }
    }
    Ok(())
}

const DEV_CHILD_LOG_TAIL_LINES: usize = 20;

async fn run_dev_supervisor(
    paths: RuntimePaths,
    server_config: ServerConfig,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let _pid_guard = acquire_pid_file(paths.dev_pid_file(), "dev")?;
    let repo_root = env::current_dir()?;
    let console_config = DevConsoleMirrorConfig::from_logging(&server_config.logging);
    let mut dev_processes = DevProcessGroup::new(
        console_config.clone(),
        server_config.dev_supervisor.child_startup_grace_ms,
    );
    dev_processes.build_extension_ui_once(&repo_root, &paths)?;

    let mut api = ApiDevProcess::new(
        repo_root.clone(),
        paths.clone(),
        server_config.clone(),
        console_config.clone(),
    );
    api.start_initial().await?;
    if let Err(error) = dev_processes.start_web(&paths, &server_config) {
        api.stop();
        return Err(error.into());
    }
    if let Err(error) = dev_processes.start_extension_ui_watch(&repo_root, &paths) {
        api.stop();
        return Err(error.into());
    }
    if let Err(error) = dev_processes.report_extension_ui_sources(&paths) {
        api.stop();
        return Err(error.into());
    }

    let (host_watch_tx, host_watch_rx) = mpsc::channel();
    let _host_watcher = start_host_watcher(&repo_root, host_watch_tx)?;
    let (builtin_watch_tx, builtin_watch_rx) = mpsc::channel();
    let _builtin_worker_watcher = start_builtin_worker_watcher(&repo_root, builtin_watch_tx)?;

    println!("Ennoia dev runtime starting at {}", paths.home().display());
    println!(
        "Web: http://{}:{}",
        server_config.web_dev.host, server_config.web_dev.port
    );
    println!("API: http://{}:{}", server_config.host, server_config.port);
    println!("Host hot reload: watching crates/, assets/, Cargo.toml and Cargo.lock.");
    println!("Builtin worker hot reload: watching assets/extensions/*/(data|plugins|worker)/.");
    println!(
        "Dev console logs: {} (level >= {}).",
        if console_config.enabled {
            "enabled"
        } else {
            "disabled"
        },
        console_config.min_level.as_str()
    );
    println!("Press Ctrl+C to stop API and Web processes.");

    let mut ticker = tokio::time::interval(Duration::from_millis(
        server_config.dev_supervisor.watch_poll_ms,
    ));
    let mut pending_host_change: Option<Instant> = None;
    let mut pending_builtin_worker_change: Option<Instant> = None;

    loop {
        tokio::select! {
            signal = wait_for_dev_stop_signal() => {
                signal?;
                println!("stopping Ennoia dev runtime...");
                break;
            }
            _ = ticker.tick() => {
                dev_processes.ensure_children_alive()?;
                if let Err(error) = api.ensure_healthy().await {
                    eprintln!("api health recovery failed: {error}");
                }
                let mut saw_change = false;
                while host_watch_rx.try_recv().is_ok() {
                    saw_change = true;
                }
                if saw_change {
                    pending_host_change = Some(Instant::now());
                }
                let mut saw_builtin_worker_change = false;
                while builtin_watch_rx.try_recv().is_ok() {
                    saw_builtin_worker_change = true;
                }
                if saw_builtin_worker_change {
                    pending_builtin_worker_change = Some(Instant::now());
                }
                if pending_host_change
                    .map(|changed_at| {
                        changed_at.elapsed()
                            >= Duration::from_millis(
                                server_config.dev_supervisor.host_reload_debounce_ms,
                            )
                    })
                    .unwrap_or(false)
                {
                    pending_host_change = None;
                    if let Err(error) = api.rebuild_and_restart().await {
                        eprintln!("host hot reload failed: {error}");
                    }
                }
                if pending_builtin_worker_change
                    .map(|changed_at| {
                        changed_at.elapsed()
                            >= Duration::from_millis(
                                server_config.dev_supervisor.host_reload_debounce_ms,
                            )
                    })
                    .unwrap_or(false)
                {
                    pending_builtin_worker_change = None;
                    if let Err(error) = ensure_builtin_process_workers(&repo_root) {
                        eprintln!("builtin worker hot reload failed: {error}");
                    }
                }
            }
        }
    }

    api.stop();
    drop(dev_processes);
    Ok(())
}

async fn wait_for_dev_stop_signal() -> io::Result<()> {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        tokio::select! {
            result = tokio::signal::ctrl_c() => result.map_err(io::Error::other),
            _ = terminate.recv() => Ok(()),
        }
    }

    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await.map_err(io::Error::other)
    }
}

struct ApiDevProcess {
    repo_root: PathBuf,
    paths: RuntimePaths,
    server_config: ServerConfig,
    console_config: DevConsoleMirrorConfig,
    target_dir: PathBuf,
    current: Option<ApiChild>,
    next_health_probe_at: Instant,
    unhealthy_since: Option<Instant>,
    unhealthy_reported: bool,
}

struct ApiChild {
    snapshot_path: PathBuf,
    child: Child,
}

impl ApiDevProcess {
    fn new(
        repo_root: PathBuf,
        paths: RuntimePaths,
        server_config: ServerConfig,
        console_config: DevConsoleMirrorConfig,
    ) -> Self {
        let target_dir = repo_root.join("target").join("ennoia-dev-api");
        let healthcheck_interval =
            Duration::from_millis(server_config.dev_supervisor.api_healthcheck_interval_ms);
        Self {
            repo_root,
            paths,
            server_config,
            console_config,
            target_dir,
            current: None,
            next_health_probe_at: Instant::now() + healthcheck_interval,
            unhealthy_since: None,
            unhealthy_reported: false,
        }
    }

    async fn start_initial(&mut self) -> io::Result<()> {
        println!("building API dev binary...");
        let built = self.build_api_binary()?;
        let snapshot = self.stage_api_binary(&built)?;
        self.current = Some(self.launch_snapshot(snapshot).await?);
        self.reset_health_watch();
        println!("started api; log={}", self.api_log_path().display());
        Ok(())
    }

    async fn rebuild_and_restart(&mut self) -> io::Result<()> {
        println!("host change detected; rebuilding API...");
        ensure_builtin_process_workers(&self.repo_root)?;
        let built = match self.build_api_binary() {
            Ok(path) => path,
            Err(error) => {
                eprintln!(
                    "host build failed; keeping previous API process alive; log={}",
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
        self.wait_for_api_port_release().await?;

        match self.launch_snapshot(snapshot.clone()).await {
            Ok(child) => {
                self.current = Some(child);
                self.reset_health_watch();
                println!("restarted api from {}", snapshot.display());
                Ok(())
            }
            Err(error) => {
                eprintln!("new API process failed; attempting rollback: {error}");
                let _ = self.wait_for_api_port_release().await;
                if let Some(previous_snapshot) = previous_snapshot {
                    self.current = Some(self.launch_snapshot(previous_snapshot).await?);
                    self.reset_health_watch();
                    eprintln!("rolled back to previous API binary");
                }
                Err(error)
            }
        }
    }

    async fn ensure_healthy(&mut self) -> io::Result<()> {
        let now = Instant::now();
        if now < self.next_health_probe_at {
            return Ok(());
        }
        self.next_health_probe_at = now
            + Duration::from_millis(
                self.server_config
                    .dev_supervisor
                    .api_healthcheck_interval_ms,
            );

        let exited_snapshot = {
            let Some(child) = self.current.as_mut() else {
                return Ok(());
            };
            child.child.try_wait()?.map(|status| {
                eprintln!("api exited unexpectedly: {status}; restarting current snapshot");
                child.snapshot_path.clone()
            })
        };
        if let Some(snapshot) = exited_snapshot {
            self.current = Some(self.launch_snapshot(snapshot).await?);
            self.reset_health_watch();
            return Ok(());
        }

        let host = self.server_config.host.clone();
        let port = self.server_config.port;
        let probe_timeout_ms = self.server_config.dev_supervisor.probe_socket_timeout_ms;
        let healthy =
            tokio::task::spawn_blocking(move || probe_api_health(&host, port, probe_timeout_ms))
                .await
                .unwrap_or(false);
        if healthy {
            if self.unhealthy_reported {
                println!("api health probe recovered");
            }
            self.unhealthy_since = None;
            self.unhealthy_reported = false;
            return Ok(());
        }

        let first_failed_at = *self.unhealthy_since.get_or_insert(now);
        if now.duration_since(first_failed_at)
            < Duration::from_millis(self.server_config.dev_supervisor.api_healthcheck_grace_ms)
        {
            return Ok(());
        }

        if !self.unhealthy_reported {
            eprintln!(
                "api health probe timed out for more than {}s; restarting current snapshot",
                Duration::from_millis(self.server_config.dev_supervisor.api_healthcheck_grace_ms)
                    .as_secs()
            );
            self.unhealthy_reported = true;
        }
        let snapshot = self
            .current
            .as_ref()
            .map(|child| child.snapshot_path.clone())
            .ok_or_else(|| {
                io::Error::other("current API snapshot missing during health recovery")
            })?;
        if let Some(child) = self.current.as_mut() {
            child.stop();
        }
        self.current = None;
        self.wait_for_api_port_release().await?;
        self.current = Some(self.launch_snapshot(snapshot.clone()).await?);
        self.reset_health_watch();
        println!("restarted unhealthy api from {}", snapshot.display());
        Ok(())
    }

    fn stop(&mut self) {
        if let Some(child) = self.current.as_mut() {
            child.stop();
        }
        self.current = None;
        self.unhealthy_since = None;
    }

    fn reset_health_watch(&mut self) {
        self.unhealthy_since = None;
        self.unhealthy_reported = false;
        self.next_health_probe_at = Instant::now()
            + Duration::from_millis(
                self.server_config
                    .dev_supervisor
                    .api_healthcheck_interval_ms,
            );
    }

    async fn wait_for_api_port_release(&self) -> io::Result<()> {
        let started = Instant::now();
        loop {
            if ensure_port_available(&self.server_config.host, self.server_config.port, "API")
                .is_ok()
            {
                return Ok(());
            }
            if started.elapsed()
                >= Duration::from_millis(
                    self.server_config
                        .dev_supervisor
                        .api_port_release_timeout_ms,
                )
            {
                return Err(io::Error::new(
                    io::ErrorKind::AddrInUse,
                    format!(
                        "API address {}:{} was not released within {}s",
                        self.server_config.host,
                        self.server_config.port,
                        Duration::from_millis(
                            self.server_config
                                .dev_supervisor
                                .api_port_release_timeout_ms,
                        )
                        .as_secs()
                    ),
                ));
            }
            tokio::time::sleep(Duration::from_millis(120)).await;
        }
    }

    fn build_api_binary(&self) -> io::Result<PathBuf> {
        self.prune_dev_api_server_artifacts()?;
        let mut command = Command::new("cargo");
        command
            .arg("build")
            .arg("-p")
            .arg("ennoia-cli")
            .env("CARGO_TARGET_DIR", &self.target_dir)
            .env("CARGO_INCREMENTAL", "0")
            .current_dir(&self.repo_root);
        let status = run_logged_command(
            "api-build",
            command,
            &self.build_log_path(),
            &self.console_config,
        )?;

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

    fn prune_dev_api_server_artifacts(&self) -> io::Result<()> {
        let deps_dir = self.target_dir.join("debug").join("deps");
        if !deps_dir.exists() {
            return Ok(());
        }

        for entry in fs::read_dir(&deps_dir)? {
            let entry = entry?;
            if !entry.file_type()?.is_file() {
                continue;
            }
            let file_name = entry.file_name();
            let file_name = file_name.to_string_lossy();
            if file_name.starts_with("libennoia_server-")
                && matches!(
                    entry.path().extension().and_then(|value| value.to_str()),
                    Some("rlib" | "rmeta")
                )
            {
                let _ = fs::remove_file(entry.path());
            }
        }

        Ok(())
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
        let mut child = Command::new(&snapshot)
            .arg("start")
            .env(ENNOIA_ALLOW_DEV_SOURCES_ENV, "1")
            .env("ENNOIA_HOME", self.paths.home())
            .current_dir(&self.repo_root)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        attach_child_log_pumps(
            "api",
            &mut child,
            &self.api_log_path(),
            &self.console_config,
        )?;

        if let Err(error) = wait_for_api_ready(&self.server_config).await {
            stop_child_process(&mut child);
            let _ = wait_for_port_release(
                &self.server_config.host,
                self.server_config.port,
                self.server_config
                    .dev_supervisor
                    .api_port_release_timeout_ms,
            )
            .await;
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
        stop_child_process(&mut self.child);
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
    console_config: DevConsoleMirrorConfig,
    child_startup_grace_ms: u64,
}

struct DevChild {
    label: String,
    log_path: PathBuf,
    child: Child,
}

impl DevProcessGroup {
    fn new(console_config: DevConsoleMirrorConfig, child_startup_grace_ms: u64) -> Self {
        Self {
            children: Vec::new(),
            console_config,
            child_startup_grace_ms,
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
            &format!(
                "bun run dev --host {} --port {} --strictPort",
                server_config.web_dev.host, server_config.web_dev.port
            ),
            &web_dir,
        );
        command.env("ENNOIA_WEB_DEV_HOST", &server_config.web_dev.host);
        command.env(
            "ENNOIA_WEB_DEV_PORT",
            server_config.web_dev.port.to_string(),
        );
        command.env(
            "VITE_ENNOIA_API_URL",
            format!("http://{}:{}", server_config.host, server_config.port),
        );
        self.spawn("web", command, &log_path)
    }

    fn start_extension_ui_watch(
        &mut self,
        repo_root: &Path,
        paths: &RuntimePaths,
    ) -> io::Result<()> {
        let script_path = repo_root.join("scripts").join("build-extension-ui.mjs");
        if !script_path.exists() {
            println!("Extension UI watcher skipped: scripts/build-extension-ui.mjs not found");
            return Ok(());
        }
        let log_path = paths.server_logs_dir().join("extension-ui-dev.log");
        let mut command = shell_command("node scripts/build-extension-ui.mjs --watch", repo_root);
        attach_extension_ui_roots_env(&mut command, paths)?;
        self.spawn("extension-ui", command, &log_path)
    }

    fn build_extension_ui_once(&self, repo_root: &Path, paths: &RuntimePaths) -> io::Result<()> {
        let script_path = repo_root.join("scripts").join("build-extension-ui.mjs");
        if !script_path.exists() {
            println!("Extension UI prebuild skipped: scripts/build-extension-ui.mjs not found");
            return Ok(());
        }
        let log_path = paths.server_logs_dir().join("extension-ui-dev.log");
        let mut command = shell_command("node scripts/build-extension-ui.mjs", repo_root);
        attach_extension_ui_roots_env(&mut command, paths)?;
        let status = run_logged_command(
            "extension-ui-build",
            command,
            &log_path,
            &self.console_config,
        )?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other(format!(
                "extension UI prebuild failed; log={}",
                log_path.display()
            )))
        }
    }

    fn report_extension_ui_sources(&mut self, paths: &RuntimePaths) -> io::Result<()> {
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
            if let Some(dev_url) = manifest.ui.dev_url {
                println!(
                    "extension {} ui uses external dev_url: {}",
                    manifest.id, dev_url
                );
            }
        }
        Ok(())
    }

    fn spawn(&mut self, label: &str, mut command: Command, log_path: &Path) -> io::Result<()> {
        if let Some(parent) = log_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut child = command
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        attach_child_log_pumps(label, &mut child, log_path, &self.console_config)?;
        println!("started {label}; log={}", log_path.display());
        ensure_dev_child_stable_start(
            label,
            &mut child,
            log_path,
            Duration::from_millis(self.child_startup_grace_ms),
        )?;
        self.children.push(DevChild {
            label: label.to_string(),
            log_path: log_path.to_path_buf(),
            child,
        });
        Ok(())
    }

    fn ensure_children_alive(&mut self) -> io::Result<()> {
        for child in &mut self.children {
            if let Some(status) = child.child.try_wait()? {
                return Err(dev_child_exit_error(&child.label, &child.log_path, status));
            }
        }
        Ok(())
    }
}

impl Drop for DevProcessGroup {
    fn drop(&mut self) {
        for child in &mut self.children {
            stop_child_process(&mut child.child);
            println!("stopped {}", child.label);
        }
    }
}

fn run_logged_command(
    label: &str,
    mut command: Command,
    log_path: &Path,
    console_config: &DevConsoleMirrorConfig,
) -> io::Result<ExitStatus> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut child = command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    attach_child_log_pumps(label, &mut child, log_path, console_config)?;
    child.wait()
}

fn attach_child_log_pumps(
    label: &str,
    child: &mut Child,
    log_path: &Path,
    console_config: &DevConsoleMirrorConfig,
) -> io::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }
    if let Some(stdout) = child.stdout.take() {
        spawn_log_pump(
            stdout,
            log_path.to_path_buf(),
            label.to_string(),
            console_config.clone(),
            false,
        );
    }
    if let Some(stderr) = child.stderr.take() {
        spawn_log_pump(
            stderr,
            log_path.to_path_buf(),
            label.to_string(),
            console_config.clone(),
            true,
        );
    }
    Ok(())
}

fn ensure_dev_child_stable_start(
    label: &str,
    child: &mut Child,
    log_path: &Path,
    grace_period: Duration,
) -> io::Result<()> {
    thread::sleep(grace_period);
    if let Some(status) = child.try_wait()? {
        thread::sleep(Duration::from_millis(120));
        return Err(dev_child_exit_error(label, log_path, status));
    }
    Ok(())
}

fn dev_child_exit_error(label: &str, log_path: &Path, status: ExitStatus) -> io::Error {
    let exit_summary = match status.code() {
        Some(code) => format!("exit code {code}"),
        None => "terminated by signal".to_string(),
    };
    let log_tail = tail_log_file(log_path, DEV_CHILD_LOG_TAIL_LINES);
    let detail = if log_tail.is_empty() {
        "日志尾部为空。".to_string()
    } else {
        format!("最近日志：\n{}", log_tail.join("\n"))
    };
    io::Error::other(format!(
        "开发子进程 '{label}' 已退出（{exit_summary}）。\n日志文件：{}\n{detail}",
        log_path.display()
    ))
}

fn tail_log_file(path: &Path, max_lines: usize) -> Vec<String> {
    if max_lines == 0 {
        return Vec::new();
    }
    let Ok(contents) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut lines = contents
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if lines.len() > max_lines {
        let drain = lines.len() - max_lines;
        lines.drain(0..drain);
    }
    lines
}

fn spawn_log_pump<R>(
    reader: R,
    log_path: PathBuf,
    label: String,
    console_config: DevConsoleMirrorConfig,
    is_stderr: bool,
) where
    R: Read + Send + 'static,
{
    thread::spawn(move || {
        let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) else {
            return;
        };
        let mut pending_block = Vec::new();
        for line in BufReader::new(reader).lines() {
            let Ok(line) = line else {
                break;
            };
            let _ = writeln!(file, "{line}");
            if pending_block.is_empty() {
                pending_block.push(line);
                continue;
            }

            if is_console_block_continuation(&line) {
                let is_blank = line.trim().is_empty();
                pending_block.push(line);
                if is_blank {
                    mirror_dev_console_block(&label, &pending_block, is_stderr, &console_config);
                    pending_block.clear();
                }
                continue;
            }

            mirror_dev_console_block(&label, &pending_block, is_stderr, &console_config);
            pending_block.clear();
            pending_block.push(line);
        }

        if !pending_block.is_empty() {
            mirror_dev_console_block(&label, &pending_block, is_stderr, &console_config);
        }
    });
}

fn is_console_block_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    line.trim().is_empty()
        || line.starts_with(' ')
        || line.starts_with('\t')
        || trimmed.starts_with("-->")
        || trimmed.starts_with('|')
        || trimmed.starts_with(":::")
        || trimmed.starts_with('=')
        || trimmed.starts_with("help:")
        || trimmed.starts_with("note:")
}

fn mirror_dev_console_block(
    label: &str,
    lines: &[String],
    is_stderr: bool,
    console_config: &DevConsoleMirrorConfig,
) {
    if lines.is_empty() {
        return;
    }
    if !console_config.enabled {
        return;
    }
    let level = detect_console_log_level(&lines[0], is_stderr);
    if level < console_config.min_level {
        return;
    }

    let formatted = if lines.len() == 1 {
        format!("[{label}] {}", lines[0])
    } else {
        format!("[{label}] {}\n{}", lines[0], lines[1..].join("\n"))
    };
    let lock = DEV_CONSOLE_OUTPUT_LOCK.get_or_init(|| Mutex::new(()));
    let _guard = lock.lock().ok();
    if is_stderr || matches!(level, ConsoleLogLevel::Warn | ConsoleLogLevel::Error) {
        eprintln!("{formatted}");
    } else {
        println!("{formatted}");
    }
}

fn detect_console_log_level(line: &str, is_stderr: bool) -> ConsoleLogLevel {
    if is_stderr {
        return ConsoleLogLevel::Error;
    }
    let lower = line.to_ascii_lowercase();
    if has_level_token(&lower, "error") {
        return ConsoleLogLevel::Error;
    }
    if has_level_token(&lower, "warn") || has_level_token(&lower, "warning") {
        return ConsoleLogLevel::Warn;
    }
    if has_level_token(&lower, "debug") {
        return ConsoleLogLevel::Debug;
    }
    ConsoleLogLevel::Info
}

fn has_level_token(line: &str, level: &str) -> bool {
    line.contains(&format!("level={level}"))
        || line.contains(&format!("[{level}]"))
        || line
            .split(|item: char| !item.is_ascii_alphabetic())
            .any(|token| token == level)
}

fn start_host_watcher(repo_root: &Path, tx: mpsc::Sender<()>) -> io::Result<RecommendedWatcher> {
    let filter_root = repo_root.to_path_buf();
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                if event
                    .paths
                    .iter()
                    .any(|path| is_host_reload_path(&filter_root, path))
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

fn start_builtin_worker_watcher(
    repo_root: &Path,
    tx: mpsc::Sender<()>,
) -> io::Result<RecommendedWatcher> {
    let filter_root = repo_root.to_path_buf();
    let mut watcher = RecommendedWatcher::new(
        move |result: Result<notify::Event, notify::Error>| {
            if let Ok(event) = result {
                if event
                    .paths
                    .iter()
                    .any(|path| is_builtin_worker_reload_path(&filter_root, path))
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
        &repo_root.join("assets").join("extensions"),
        RecursiveMode::Recursive,
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

fn is_host_reload_path(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    if relative.starts_with("target") || relative.starts_with("web") {
        return false;
    }
    if relative == Path::new("Cargo.toml") || relative == Path::new("Cargo.lock") {
        return true;
    }
    if relative.starts_with("assets") {
        return !relative.starts_with(Path::new("assets").join("extensions"))
            && !relative.starts_with(Path::new("assets").join("skills"))
            && has_host_reload_extension(path);
    }
    if !relative.starts_with("crates") {
        return false;
    }
    has_host_reload_extension(path)
}

fn is_builtin_worker_reload_path(repo_root: &Path, path: &Path) -> bool {
    let Ok(relative) = path.strip_prefix(repo_root) else {
        return false;
    };
    if relative.starts_with("target") || relative.starts_with("web") {
        return false;
    }
    if !relative.starts_with(Path::new("assets").join("extensions")) {
        return false;
    }
    is_builtin_worker_reload_relative_path(relative)
}

fn is_builtin_worker_reload_relative_path(relative: &Path) -> bool {
    let mut components = relative.components();
    let _ = components.next();
    let _ = components.next();
    let _ = components.next();
    let Some(scope) = components.next().and_then(|item| item.as_os_str().to_str()) else {
        return false;
    };
    if !matches!(scope, "data" | "plugins" | "worker") {
        return false;
    }
    has_host_reload_extension(relative)
}

fn has_host_reload_extension(path: &Path) -> bool {
    match path.extension().and_then(|value| value.to_str()) {
        Some("rs" | "toml" | "sql" | "json" | "js" | "ts" | "css" | "html" | "wasm") => true,
        None => true,
        _ => false,
    }
}

async fn wait_for_api_ready(config: &ServerConfig) -> io::Result<()> {
    let timeout = Duration::from_millis(config.dev_supervisor.api_ready_timeout_ms);
    let started = Instant::now();
    loop {
        let host = config.host.clone();
        let port = config.port;
        let probe_timeout_ms = config.dev_supervisor.probe_socket_timeout_ms;
        if tokio::task::spawn_blocking(move || probe_api_health(&host, port, probe_timeout_ms))
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
        tokio::time::sleep(Duration::from_millis(
            config.dev_supervisor.watch_poll_ms.max(100),
        ))
        .await;
    }
}

async fn wait_for_port_release(host: &str, port: u16, timeout_ms: u64) -> io::Result<()> {
    let started = Instant::now();
    loop {
        if ensure_port_available(host, port, "API").is_ok() {
            return Ok(());
        }
        if started.elapsed() >= Duration::from_millis(timeout_ms) {
            return Err(io::Error::new(
                io::ErrorKind::AddrInUse,
                format!(
                    "API address {host}:{port} was not released within {}s",
                    Duration::from_millis(timeout_ms).as_secs()
                ),
            ));
        }
        tokio::time::sleep(Duration::from_millis(120)).await;
    }
}

fn probe_api_health(host: &str, port: u16, timeout_ms: u64) -> bool {
    let Ok(mut stream) = TcpStream::connect((host, port)) else {
        return false;
    };
    let timeout = Duration::from_millis(timeout_ms);
    let _ = stream.set_read_timeout(Some(timeout));
    let _ = stream.set_write_timeout(Some(timeout));
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

fn ensure_port_available(host: &str, port: u16, label: &str) -> io::Result<()> {
    TcpListener::bind((host, port))
        .map(|listener| drop(listener))
        .map_err(|_| {
            io::Error::new(
                io::ErrorKind::AddrInUse,
                format!(
                    "{label} address {host}:{port} is already in use; stop the existing process and retry"
                ),
            )
        })
}

fn stop_child_process(child: &mut Child) {
    if child.try_wait().ok().flatten().is_some() {
        let _ = child.wait();
        return;
    }

    #[cfg(windows)]
    {
        let _ = terminate_windows_process_tree(child.id());
    }

    #[cfg(not(windows))]
    {
        let _ = child.kill();
    }

    let started = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let _ = child.wait();
                break;
            }
            Ok(None) if started.elapsed() < Duration::from_secs(5) => {
                thread::sleep(Duration::from_millis(80));
            }
            Ok(None) => {
                let _ = child.kill();
                let _ = child.wait();
                break;
            }
            Err(_) => break,
        }
    }
}

#[cfg(windows)]
fn terminate_windows_process_tree(pid: u32) -> io::Result<()> {
    let status = Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(io::Error::other(format!(
            "taskkill failed for process tree {pid}"
        )))
    }
}

fn ensure_builtin_process_workers(repo_root: &Path) -> io::Result<()> {
    let cargo = env::var_os("CARGO").unwrap_or_else(|| {
        if cfg!(windows) {
            "cargo.exe".into()
        } else {
            "cargo".into()
        }
    });
    let status = Command::new(cargo)
        .arg("build")
        .arg("-p")
        .arg("ennoia-conversation-service")
        .arg("-p")
        .arg("ennoia-memory")
        .arg("-p")
        .arg("ennoia-workflow")
        .current_dir(repo_root)
        .status()?;
    if !status.success() {
        return Err(io::Error::other("failed to build builtin process workers"));
    }

    let conversation_root = repo_root
        .join("assets")
        .join("extensions")
        .join("conversation");
    if conversation_root.join("extension.toml").exists() {
        let built_binary = repo_root
            .join("target")
            .join("debug")
            .join(if cfg!(windows) {
                "ennoia-conversation-service.exe"
            } else {
                "ennoia-conversation-service"
            });
        if !built_binary.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "conversation process worker not found at {}",
                    built_binary.display()
                ),
            ));
        }

        let destination = conversation_root
            .join("bin")
            .join(conversation_service_name());
        copy_builtin_process_worker(&built_binary, &destination)?;
    }

    let memory_root = repo_root.join("assets").join("extensions").join("memory");
    if memory_root.join("extension.toml").exists() {
        let built_binary = repo_root
            .join("target")
            .join("debug")
            .join(if cfg!(windows) {
                "ennoia-memory-extension.exe"
            } else {
                "ennoia-memory-extension"
            });
        if !built_binary.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "memory process worker not found at {}",
                    built_binary.display()
                ),
            ));
        }

        let destination = memory_root.join("bin").join(memory_service_name());
        copy_builtin_process_worker(&built_binary, &destination)?;
    }

    let workflow_root = repo_root.join("assets").join("extensions").join("workflow");
    if workflow_root.join("extension.toml").exists() {
        let built_binary = repo_root
            .join("target")
            .join("debug")
            .join(if cfg!(windows) {
                "ennoia-workflow-extension.exe"
            } else {
                "ennoia-workflow-extension"
            });
        if !built_binary.exists() {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "workflow process worker not found at {}",
                    built_binary.display()
                ),
            ));
        }

        let destination = workflow_root.join("bin").join(workflow_service_name());
        copy_builtin_process_worker(&built_binary, &destination)?;
    }

    Ok(())
}

fn copy_builtin_process_worker(source: &Path, destination: &Path) -> io::Result<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(destination)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(destination, permissions)?;
    }
    Ok(())
}

fn conversation_service_name() -> &'static str {
    if cfg!(windows) {
        "conversation-service.exe"
    } else {
        "conversation-service"
    }
}

fn memory_service_name() -> &'static str {
    if cfg!(windows) {
        "memory-service.exe"
    } else {
        "memory-service"
    }
}

fn workflow_service_name() -> &'static str {
    if cfg!(windows) {
        "workflow-service.exe"
    } else {
        "workflow-service"
    }
}

fn attached_dev_source_roots(paths: &RuntimePaths) -> io::Result<Vec<PathBuf>> {
    let mut roots = Vec::new();
    let registry = read_extension_registry(paths)?;
    for entry in registry.dev_sources.into_iter().filter(|item| item.enabled) {
        roots.push(PathBuf::from(entry.path));
    }

    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn attach_extension_ui_roots_env(command: &mut Command, paths: &RuntimePaths) -> io::Result<()> {
    let roots = attached_dev_source_roots(paths)?
        .into_iter()
        .map(|root| root.to_string_lossy().replace('\\', "/"))
        .collect::<Vec<_>>();
    if !roots.is_empty() {
        let payload = serde_json::to_string(&roots).map_err(io::Error::other)?;
        command.env("ENNOIA_EXTENSION_UI_ROOTS", payload);
    }
    Ok(())
}

fn descriptor_path(root: &Path) -> Option<PathBuf> {
    let path = root.join("extension.toml");
    path.exists().then_some(path)
}

fn auto_attach_dev_extensions(paths: &RuntimePaths) -> io::Result<()> {
    let builtin_extensions_dir = env::current_dir()?.join("assets").join("extensions");
    if !builtin_extensions_dir.exists() {
        return Ok(());
    }

    let mut registry = read_extension_registry(paths)?;

    for entry in fs::read_dir(builtin_extensions_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let root = entry.path();
        if !root.join("extension.toml").exists() {
            continue;
        }
        let normalized = root.to_string_lossy().replace('\\', "/");
        let id = entry.file_name().to_string_lossy().to_string();
        if registry.blocked_builtin_sync.iter().any(|item| item == &id)
            || registry.dev_sources.iter().any(|item| item.id == id)
        {
            continue;
        }
        registry.dev_sources.push(ExtensionDevSourceEntry {
            id,
            path: normalized,
            enabled: true,
        });
    }

    sort_extension_dev_sources(&mut registry.dev_sources);
    write_extension_registry(paths, &registry)?;
    Ok(())
}

fn init_home_template(paths: &RuntimePaths) -> io::Result<()> {
    paths.ensure_layout()?;

    write_if_missing(&paths.server_config_file(), templates::server_config())?;
    write_if_missing(&paths.ui_config_file(), templates::ui_config())?;
    migrate_ui_config(&paths.ui_config_file())?;
    sync_builtin_registries(paths)?;
    materialize_builtin_packages(paths)?;
    sync_builtin_provider_presets(paths)?;
    Ok(())
}

fn sync_builtin_registries(paths: &RuntimePaths) -> io::Result<()> {
    let mut extension_registry = read_extension_registry(paths)?;
    for id in builtin_extension_ids() {
        if extension_registry
            .extensions
            .iter_mut()
            .find(|item| item.id == id)
            .is_some()
        {
            continue;
        }
        extension_registry.extensions.push(ExtensionRegistryEntry {
            id: id.clone(),
            enabled: true,
        });
    }
    sort_extension_registry_entries(&mut extension_registry.extensions);
    write_extension_registry(paths, &extension_registry)?;

    let mut skill_registry = read_skill_registry(paths)?;
    for id in builtin_skill_ids() {
        if skill_registry
            .skills
            .iter_mut()
            .find(|item| item.id == id)
            .is_some()
        {
            continue;
        }
        skill_registry.skills.push(SkillRegistryEntry {
            id: id.clone(),
            enabled: true,
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
        if is_blocked_builtin_extension(&extension_registry, id) {
            continue;
        }
        write_text_asset(paths.home(), asset.logical_path, asset.contents)?;
    }
    for asset in builtins::extension_binaries() {
        let Some(id) = builtin_package_id(asset.logical_path) else {
            continue;
        };
        if is_blocked_builtin_extension(&extension_registry, id) {
            continue;
        }
        write_binary_asset(paths.home(), asset.logical_path, asset.contents)?;
    }

    for asset in builtins::skills() {
        let Some(id) = builtin_package_id(asset.logical_path) else {
            continue;
        };
        if is_blocked_builtin_skill(&skill_registry, id) {
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
        .filter(|item| item.enabled)
    {
        if is_blocked_builtin_extension(&extension_registry, &entry.id) {
            continue;
        }
        let root = paths.extension_dir(&entry.id);
        let presets_dir = root.join("model-endpoint-presets");
        if !presets_dir.exists() {
            continue;
        }

        for preset in fs::read_dir(presets_dir)? {
            let preset = preset?;
            if !preset.file_type()?.is_file() {
                continue;
            }
            let destination = paths.model_endpoints_config_dir().join(preset.file_name());
            let contents = fs::read_to_string(preset.path())?;
            write_if_missing(&destination, &contents)?;
        }
    }

    Ok(())
}

fn builtin_extension_ids() -> Vec<String> {
    builtin_package_ids_from_assets(builtins::extensions(), "extension.toml")
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

fn is_blocked_builtin_extension(registry: &ExtensionRegistryFile, id: &str) -> bool {
    registry
        .blocked_builtin_sync
        .iter()
        .any(|entry| entry == id)
}

fn is_blocked_builtin_skill(registry: &SkillRegistryFile, id: &str) -> bool {
    registry
        .blocked_builtin_sync
        .iter()
        .any(|entry| entry == id)
}

fn write_text_asset(root: &Path, logical_path: &str, contents: &str) -> io::Result<()> {
    let path = root.join(logical_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)
}

fn write_binary_asset(root: &Path, logical_path: &str, contents: &[u8]) -> io::Result<()> {
    let path = root.join(logical_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, contents)?;
    ensure_binary_asset_permissions(&path, logical_path)
}

fn ensure_binary_asset_permissions(path: &Path, logical_path: &str) -> io::Result<()> {
    #[cfg(unix)]
    if should_mark_binary_asset_executable(logical_path) {
        use std::os::unix::fs::PermissionsExt;

        let mut permissions = fs::metadata(path)?.permissions();
        permissions.set_mode(0o755);
        fs::set_permissions(path, permissions)?;
    }

    #[cfg(not(unix))]
    let _ = (path, logical_path);

    Ok(())
}

#[cfg(any(test, unix))]
fn should_mark_binary_asset_executable(logical_path: &str) -> bool {
    matches!(
        logical_path.split('/').collect::<Vec<_>>().as_slice(),
        ["extensions", _, "bin", ..]
    )
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        ensure_no_args, extension_subcommand_usage, extension_usage, home_command_usage,
        internal_subcommand_usage, is_builtin_worker_reload_path, is_host_reload_path,
        parse_optional_home_arg, parse_optional_usize_arg, parse_required_arg, print_config_usage,
        should_mark_binary_asset_executable, summary_text, tail_log_file, unique_suffix,
    };

    fn as_args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    #[test]
    fn marks_builtin_process_worker_paths_as_executable() {
        assert!(should_mark_binary_asset_executable(
            "extensions/conversation/bin/conversation-service"
        ));
        assert!(should_mark_binary_asset_executable(
            "extensions/memory/bin/memory-service.exe"
        ));
    }

    #[test]
    fn ignores_non_worker_asset_paths() {
        assert!(!should_mark_binary_asset_executable(
            "extensions/workflow/worker/workflow.wasm"
        ));
        assert!(!should_mark_binary_asset_executable(
            "skills/example/skill.toml"
        ));
    }

    #[test]
    fn tail_log_file_returns_last_non_empty_lines() {
        let path = std::env::temp_dir().join(format!("ennoia-log-tail-{}.log", unique_suffix()));
        fs::write(&path, "line-1\n\nline-2\nline-3\nline-4\n").expect("write log file");

        let lines = tail_log_file(&path, 2);

        assert_eq!(lines, vec!["line-3".to_string(), "line-4".to_string()]);
        let _ = fs::remove_file(path);
    }

    #[test]
    fn host_reload_ignores_builtin_extension_sources() {
        let repo_root = Path::new("C:/repo");
        assert!(!is_host_reload_path(
            repo_root,
            &repo_root.join(
                "assets/extensions/workflow/plugins/workflow-service/src/conversation_hooks.rs"
            )
        ));
    }

    #[test]
    fn builtin_worker_reload_includes_backend_sources() {
        let repo_root = Path::new("C:/repo");
        assert!(is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join(
                "assets/extensions/workflow/plugins/workflow-service/src/conversation_hooks.rs"
            )
        ));
        assert!(is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/data/schema.sql")
        ));
        assert!(is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/worker/src/lib.rs")
        ));
    }

    #[test]
    fn builtin_worker_reload_ignores_ui_outputs_and_manifests() {
        let repo_root = Path::new("C:/repo");
        assert!(!is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/bin/workflow-service.exe")
        ));
        assert!(!is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/ui/page/Page.tsx")
        ));
        assert!(!is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/install/data/system/sqlite/logs.db")
        ));
        assert!(!is_builtin_worker_reload_path(
            repo_root,
            &repo_root.join("assets/extensions/workflow/extension.toml")
        ));
    }

    #[test]
    fn dev_command_rejects_custom_home_argument() {
        let args = as_args(&["C:/ennoia-home"]);
        let usage = home_command_usage("dev");
        let error = ensure_no_args("dev", &args, &usage).expect_err("dev should reject args");
        assert!(error
            .to_string()
            .contains("unexpected argument for 'ennoia dev': C:/ennoia-home"));
    }

    #[test]
    fn start_command_rejects_custom_home_argument() {
        let args = as_args(&["C:/ennoia-home"]);
        let usage = home_command_usage("start");
        let error = ensure_no_args("start", &args, &usage).expect_err("start should reject args");
        assert!(error
            .to_string()
            .contains("unexpected argument for 'ennoia start': C:/ennoia-home"));
    }

    #[test]
    fn parse_optional_home_arg_rejects_flags() {
        let args = as_args(&["--bad"]);
        let error = parse_optional_home_arg("dev", &args).expect_err("flag should fail");
        assert!(error
            .to_string()
            .contains("unknown option for 'ennoia dev': --bad"));
    }

    #[test]
    fn parse_required_arg_rejects_extra_values() {
        let args = as_args(&["alpha", "beta"]);
        let usage = extension_subcommand_usage("attach");
        let error =
            parse_required_arg("ext attach", &args, &usage).expect_err("extra args should fail");
        assert!(error
            .to_string()
            .contains("too many arguments for 'ennoia ext attach'"));
    }

    #[test]
    fn parse_optional_usize_arg_rejects_invalid_numbers() {
        let args = as_args(&["oops"]);
        let usage = extension_subcommand_usage("logs");
        let error = parse_optional_usize_arg("ext logs", &args, &usage, 20)
            .expect_err("invalid numeric input should fail");
        assert!(error
            .to_string()
            .contains("invalid numeric argument for 'ennoia ext logs': oops"));
    }

    #[test]
    fn ensure_no_args_rejects_unexpected_argument() {
        let args = as_args(&["unexpected"]);
        let usage = print_config_usage();
        let error =
            ensure_no_args("print-config", &args, &usage).expect_err("unexpected arg should fail");
        assert!(error
            .to_string()
            .contains("unexpected argument for 'ennoia print-config': unexpected"));
    }

    #[test]
    fn usage_texts_include_new_commands() {
        let summary = summary_text();
        let dev_usage = home_command_usage("dev");
        let start_usage = home_command_usage("start");
        assert!(summary.contains("ennoia dev\n"));
        assert!(!summary.contains("ennoia dev [home]"));
        assert!(summary.contains("ennoia serve\n"));
        assert!(!summary.contains("ennoia start [home]"));
        assert!(summary.contains("ennoia print-config"));
        assert!(dev_usage.contains("repository-local .dev directory"));
        assert!(start_usage.contains("does not accept a path argument"));
        assert!(extension_usage().contains("usage: ennoia ext <subcommand> [args]"));
        assert!(internal_subcommand_usage("sandbox-worker")
            .contains("sandbox-worker <request.json> <response.json>"));
    }
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
    entries.sort_by(|left, right| left.id.cmp(&right.id));
}

fn sort_skill_registry_entries(entries: &mut [SkillRegistryEntry]) {
    entries.sort_by(|left, right| left.id.cmp(&right.id));
}

fn sort_extension_dev_sources(entries: &mut [ExtensionDevSourceEntry]) {
    entries.sort_by(|left, right| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.path.cmp(&right.path))
    });
}

fn write_if_missing(path: &Path, contents: &str) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, contents)?;
    }

    Ok(())
}

fn migrate_ui_config(path: &Path) -> io::Result<()> {
    if !path.exists() {
        return Ok(());
    }

    let contents = fs::read_to_string(path)?;
    let migrated = contents
        .replace("shell_title", "web_title")
        .replace("shell.title", "web.title");
    if migrated != contents {
        fs::write(path, migrated)?;
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
