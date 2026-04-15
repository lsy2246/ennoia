use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ennoia_server::{bootstrap_app_state, default_app_state, run_server};

const ENNOIA_HOME_ENV: &str = "ENNOIA_HOME";

const APP_CONFIG_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/ennoia.toml");
const SERVER_CONFIG_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/server.toml");
const UI_CONFIG_TEMPLATE: &str = include_str!("../../../packaging/home-template/config/ui.toml");
const CODER_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/agents/coder.toml");
const PLANNER_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/agents/planner.toml");
const OBSERVATORY_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/extensions/observatory.toml");
const GITHUB_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/config/extensions/github.toml");
const OBSERVATORY_MANIFEST_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/global/extensions/observatory/manifest.toml");
const GITHUB_MANIFEST_TEMPLATE: &str =
    include_str!("../../../packaging/home-template/global/extensions/github/manifest.toml");

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

fn init_home_template(target: &Path) -> io::Result<()> {
    let config_dir = target.join("config");
    fs::create_dir_all(config_dir.join("agents"))?;
    fs::create_dir_all(config_dir.join("extensions"))?;
    fs::create_dir_all(target.join("state/queue"))?;
    fs::create_dir_all(target.join("state/runs"))?;
    fs::create_dir_all(target.join("state/cache"))?;
    fs::create_dir_all(target.join("state/sqlite"))?;
    fs::create_dir_all(target.join("global/extensions/observatory"))?;
    fs::create_dir_all(target.join("global/extensions/github"))?;
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
    write_if_missing(&config_dir.join("extensions/github.toml"), GITHUB_TEMPLATE)?;
    write_if_missing(
        &target.join("global/extensions/observatory/manifest.toml"),
        OBSERVATORY_MANIFEST_TEMPLATE,
    )?;
    write_if_missing(
        &target.join("global/extensions/github/manifest.toml"),
        GITHUB_MANIFEST_TEMPLATE,
    )?;

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
