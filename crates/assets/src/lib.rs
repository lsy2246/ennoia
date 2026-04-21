//! Ennoia compile-time assets registry.
//!
//! This crate is the single source of truth for built-in templates and
//! migrations. Runtime crates must consume assets through these APIs instead of
//! referencing repository paths directly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAsset {
    pub logical_path: &'static str,
    pub contents: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/generated_assets.rs"));

fn lookup(assets: &'static [(&'static str, &'static str)], path: &str) -> Option<&'static str> {
    assets
        .iter()
        .find_map(|(logical_path, contents)| (*logical_path == path).then_some(*contents))
}

fn wrap_assets(assets: &'static [(&'static str, &'static str)]) -> Vec<TextAsset> {
    assets
        .iter()
        .map(|(logical_path, contents)| TextAsset {
            logical_path,
            contents,
        })
        .collect()
}

pub mod templates {
    use super::{lookup, wrap_assets, TextAsset, TEMPLATE_ASSETS};

    pub fn all() -> Vec<TextAsset> {
        wrap_assets(TEMPLATE_ASSETS)
    }

    pub fn get(path: &str) -> Option<&'static str> {
        lookup(TEMPLATE_ASSETS, path)
    }

    pub fn app_config() -> &'static str {
        get("config/ennoia.toml").expect("app config template")
    }

    pub fn server_config() -> &'static str {
        get("config/server.toml").expect("server config template")
    }

    pub fn ui_config() -> &'static str {
        get("config/ui.toml").expect("ui config template")
    }

    pub fn implementation_skill() -> &'static str {
        get("config/skills/implementation.toml").expect("implementation skill template")
    }

    pub fn task_planning_skill() -> &'static str {
        get("config/skills/task-planning.toml").expect("task planning skill template")
    }

    pub fn frontend_design_skill() -> &'static str {
        get("config/skills/frontend-design.toml").expect("frontend design skill template")
    }

    pub fn openai_provider() -> &'static str {
        get("config/providers/openai.toml").expect("openai provider template")
    }

    pub fn attached_workspaces() -> &'static str {
        get("extensions/attached/workspaces.toml").expect("attached workspaces template")
    }

    pub fn memory_policy() -> &'static str {
        get("policies/memory.toml").expect("memory policy template")
    }

    pub fn stage_policy() -> &'static str {
        get("policies/stage.toml").expect("stage policy template")
    }
}

pub mod migrations {
    use super::{lookup, wrap_assets, TextAsset, MIGRATION_ASSETS};

    pub fn all() -> Vec<TextAsset> {
        wrap_assets(MIGRATION_ASSETS)
    }

    pub fn get(path: &str) -> Option<&'static str> {
        lookup(MIGRATION_ASSETS, path)
    }
}
