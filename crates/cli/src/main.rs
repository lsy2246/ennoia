use std::env;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use ennoia_server::{build_router, default_app_state};

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("init") => {
            let target = args
                .get(2)
                .map(PathBuf::from)
                .unwrap_or_else(default_home_template_path);
            init_home_template(&target)?;
            println!("initialized Ennoia home at {}", target.display());
        }
        Some("print-config") => {
            print_default_config()?;
        }
        Some("dev") => {
            let _router = build_router(default_app_state());
            println!("Ennoia dev router prepared");
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

fn print_default_config() -> Result<(), Box<dyn std::error::Error>> {
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
    fs::create_dir_all(target.join("global/extensions"))?;
    fs::create_dir_all(target.join("global/skills"))?;
    fs::create_dir_all(target.join("agents"))?;
    fs::create_dir_all(target.join("spaces"))?;
    fs::create_dir_all(target.join("logs"))?;

    fs::write(config_dir.join("ennoia.toml"), APP_CONFIG_TEMPLATE)?;
    fs::write(config_dir.join("server.toml"), SERVER_CONFIG_TEMPLATE)?;
    fs::write(config_dir.join("ui.toml"), UI_CONFIG_TEMPLATE)?;
    fs::write(config_dir.join("agents/coder.toml"), CODER_TEMPLATE)?;
    fs::write(config_dir.join("agents/planner.toml"), PLANNER_TEMPLATE)?;
    fs::write(
        config_dir.join("extensions/observatory.toml"),
        OBSERVATORY_TEMPLATE,
    )?;
    fs::write(config_dir.join("extensions/github.toml"), GITHUB_TEMPLATE)?;

    Ok(())
}

fn default_home_template_path() -> PathBuf {
    env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ennoia")
}
