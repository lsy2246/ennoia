use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ennoia_server::{bootstrap_app_state, default_app_state, run_server, AppState};

const ENNOIA_HOME_ENV: &str = "ENNOIA_HOME";

const APP_CONFIG_TEMPLATE: &str = include_str!("../templates/config/ennoia.toml");
const SERVER_CONFIG_TEMPLATE: &str = include_str!("../templates/config/server.toml");
const UI_CONFIG_TEMPLATE: &str = include_str!("../templates/config/ui.toml");
const CODER_TEMPLATE: &str = include_str!("../templates/config/agents/coder.toml");
const PLANNER_TEMPLATE: &str = include_str!("../templates/config/agents/planner.toml");
const OBSERVATORY_TEMPLATE: &str = include_str!("../templates/config/extensions/observatory.toml");
const OBSERVATORY_MANIFEST_TEMPLATE: &str =
    include_str!("../templates/global/extensions/observatory/manifest.toml");
const MEMORY_POLICY_TEMPLATE: &str = include_str!("../templates/policies/memory.toml");
const STAGE_POLICY_TEMPLATE: &str = include_str!("../templates/policies/stage.toml");

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("init") => {
            let target = resolve_runtime_path(args.get(2));
            init_home_template(&target)?;
            println!("initialized Ennoia home at {}", target.display());
        }
        Some("print-config") => {
            print_default_config()?;
        }
        Some("dev") => {
            let target = resolve_runtime_path(args.get(2));
            init_home_template(&target)?;
            let state = bootstrap_app_state(&target).await?;
            println!(
                "Ennoia dev runtime ready at {} with {} agents",
                target.display(),
                state.agents.len()
            );
            run_server(&target).await?;
        }
        Some("start") | Some("serve") => {
            let target = resolve_runtime_path(args.get(2));
            init_home_template(&target)?;
            run_server(&target).await?;
        }
        Some("memory") => {
            memory_command(&args[2..]).await?;
        }
        Some("admin") => {
            admin_command(&args[2..]).await?;
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
    println!("  ennoia memory list");
    println!("  ennoia memory remember <owner_kind> <owner_id> <namespace> <content>");
    println!("  ennoia memory recall <owner_kind> <owner_id> [query]");
    println!("  ennoia admin create-admin <username> <password> [display_name]");
    println!("  ennoia admin list-users");
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
    let target = resolve_runtime_path(None);
    init_home_template(&target)?;
    let state = bootstrap_app_state(&target).await?;

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

fn init_home_template(target: &Path) -> io::Result<()> {
    let config_dir = target.join("config");
    let policies_dir = target.join("policies");
    fs::create_dir_all(config_dir.join("agents"))?;
    fs::create_dir_all(config_dir.join("extensions"))?;
    fs::create_dir_all(&policies_dir)?;
    fs::create_dir_all(target.join("state/queue"))?;
    fs::create_dir_all(target.join("state/runs"))?;
    fs::create_dir_all(target.join("state/cache"))?;
    fs::create_dir_all(target.join("state/sqlite"))?;
    fs::create_dir_all(target.join("global/extensions/observatory"))?;
    fs::create_dir_all(target.join("global/skills"))?;
    fs::create_dir_all(target.join("agents"))?;
    fs::create_dir_all(target.join("spaces"))?;
    fs::create_dir_all(target.join("logs"))?;

    write_if_missing(&config_dir.join("ennoia.toml"), APP_CONFIG_TEMPLATE)?;
    write_if_missing(&config_dir.join("server.toml"), SERVER_CONFIG_TEMPLATE)?;
    write_if_missing(&config_dir.join("ui.toml"), UI_CONFIG_TEMPLATE)?;
    write_if_missing(&config_dir.join("agents/coder.toml"), CODER_TEMPLATE)?;
    write_if_missing(&config_dir.join("agents/planner.toml"), PLANNER_TEMPLATE)?;
    write_if_missing(
        &config_dir.join("extensions/observatory.toml"),
        OBSERVATORY_TEMPLATE,
    )?;
    write_if_missing(
        &target.join("global/extensions/observatory/manifest.toml"),
        OBSERVATORY_MANIFEST_TEMPLATE,
    )?;
    write_if_missing(&policies_dir.join("memory.toml"), MEMORY_POLICY_TEMPLATE)?;
    write_if_missing(&policies_dir.join("stage.toml"), STAGE_POLICY_TEMPLATE)?;

    Ok(())
}

fn default_home_template_path() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ennoia")
}

fn resolve_runtime_path(argument: Option<&String>) -> PathBuf {
    argument
        .map(PathBuf::from)
        .or_else(|| env::var_os(ENNOIA_HOME_ENV).map(PathBuf::from))
        .unwrap_or_else(default_home_template_path)
}

fn write_if_missing(path: &Path, contents: &str) -> io::Result<()> {
    if !path.exists() {
        fs::write(path, contents)?;
    }

    Ok(())
}

async fn admin_command(args: &[String]) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let target = resolve_runtime_path(None);
    init_home_template(&target)?;
    let state = ennoia_server::bootstrap_app_state(&target).await?;

    let sub = args.first().map(String::as_str).unwrap_or("help");
    match sub {
        "create-admin" => admin_create_admin(&state, &args[1..]).await,
        "list-users" => admin_list_users(&state).await,
        other => {
            eprintln!("unknown admin subcommand: {other}");
            eprintln!("available: create-admin, list-users");
            std::process::exit(2);
        }
    }
}

async fn admin_create_admin(
    state: &ennoia_server::AppState,
    args: &[String],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if args.len() < 2 {
        eprintln!("usage: ennoia admin create-admin <username> <password> [display_name]");
        std::process::exit(2);
    }
    let username = &args[0];
    let password = &args[1];
    let display_name = args.get(2).cloned();

    let existing = state.user_store.count().await?;
    if existing > 0 {
        eprintln!("note: {existing} user(s) already exist; still creating admin");
    }

    let user = state
        .auth_service
        .register(
            username,
            password,
            display_name,
            None,
            ennoia_kernel::UserRole::Admin,
        )
        .await?;
    println!("created admin user: {}", user.id);
    println!("username: {}", user.username);
    Ok(())
}

async fn admin_list_users(
    state: &ennoia_server::AppState,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let users = state.user_store.list().await?;
    if users.is_empty() {
        println!("(no users)");
        return Ok(());
    }
    for user in users {
        println!(
            "{}  {}  [{}]  created={}",
            user.id,
            user.username,
            user.role.as_str(),
            user.created_at
        );
    }
    Ok(())
}
