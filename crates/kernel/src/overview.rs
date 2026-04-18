use serde::{Deserialize, Serialize};

use crate::OwnerKind;

/// PlatformOverview powers the CLI summary and the server overview endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformOverview {
    pub app_name: String,
    pub modules: Vec<String>,
    pub primary_database: String,
    pub default_owner_kind: OwnerKind,
    pub supports_private_threads: bool,
    pub supports_space_threads: bool,
}

impl Default for PlatformOverview {
    fn default() -> Self {
        Self {
            app_name: "Ennoia".to_string(),
            modules: core_modules(),
            primary_database: "sqlite".to_string(),
            default_owner_kind: OwnerKind::Space,
            supports_private_threads: true,
            supports_space_threads: true,
        }
    }
}

/// Returns the stable core module list.
pub fn core_modules() -> Vec<String> {
    vec![
        "kernel".to_string(),
        "policy".to_string(),
        "memory".to_string(),
        "runtime".to_string(),
        "orchestrator".to_string(),
        "scheduler".to_string(),
        "extension-host".to_string(),
        "server".to_string(),
        "cli".to_string(),
    ]
}
