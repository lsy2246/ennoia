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

    pub fn coder_agent() -> &'static str {
        get("config/agents/coder.toml").expect("coder template")
    }

    pub fn planner_agent() -> &'static str {
        get("config/agents/planner.toml").expect("planner template")
    }

    pub fn observatory_extension_config() -> &'static str {
        get("config/extensions/observatory.toml").expect("observatory extension template")
    }

    pub fn observatory_manifest() -> &'static str {
        get("global/extensions/observatory/manifest.toml").expect("observatory manifest template")
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
