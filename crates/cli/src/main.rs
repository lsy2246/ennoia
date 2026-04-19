use std::env;
use std::fs;
use std::io;
use std::path::Path;

use ennoia_assets::templates;
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
            let state = bootstrap_app_state(paths.home()).await?;
            println!(
                "Ennoia dev runtime ready at {} with {} agents",
                paths.home().display(),
                state.agents.len()
            );
            run_server(paths.home()).await?;
        }
        Some("start") | Some("serve") => {
            let paths = RuntimePaths::resolve(args.get(2).map(String::as_str));
            init_home_template(&paths)?;
            run_server(paths.home()).await?;
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

fn init_home_template(paths: &RuntimePaths) -> io::Result<()> {
    paths.ensure_layout()?;
    fs::create_dir_all(paths.global_extension_dir("observatory"))?;

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
