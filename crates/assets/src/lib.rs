//! Ennoia compile-time assets registry.
//!
//! This crate is the single source of truth for built-in templates,
//! migrations and database snapshots. Runtime crates must consume assets through these APIs instead of
//! referencing repository paths directly.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextAsset {
    pub logical_path: &'static str,
    pub contents: &'static str,
}

include!(concat!(env!("OUT_DIR"), "/generated_assets.rs"));

pub fn db_sql() -> &'static str {
    DB_SQL
}

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

    pub fn openai_provider() -> &'static str {
        get("config/providers/openai.toml").expect("openai provider template")
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

pub mod builtins {
    use super::{lookup, wrap_assets, TextAsset, BUILTIN_ASSETS};

    pub fn all() -> Vec<TextAsset> {
        wrap_assets(BUILTIN_ASSETS)
    }

    pub fn get(path: &str) -> Option<&'static str> {
        lookup(BUILTIN_ASSETS, path)
    }

    pub fn extensions() -> Vec<TextAsset> {
        filter_prefix("extensions/")
    }

    pub fn skills() -> Vec<TextAsset> {
        filter_prefix("skills/")
    }

    fn filter_prefix(prefix: &str) -> Vec<TextAsset> {
        BUILTIN_ASSETS
            .iter()
            .filter(|(logical_path, _)| logical_path.starts_with(prefix))
            .map(|(logical_path, contents)| TextAsset {
                logical_path,
                contents,
            })
            .collect()
    }
}
