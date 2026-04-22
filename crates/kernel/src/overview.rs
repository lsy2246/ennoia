use serde::{Deserialize, Serialize};

use crate::OwnerKind;

/// PlatformOverview powers the CLI summary and the server overview endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlatformOverview {
    pub app_name: String,
    pub modules: Vec<String>,
    pub default_owner_kind: OwnerKind,
}

impl Default for PlatformOverview {
    fn default() -> Self {
        Self {
            app_name: "Ennoia".to_string(),
            modules: core_modules(),
            default_owner_kind: OwnerKind::Space,
        }
    }
}

/// Returns the stable core module list.
pub fn core_modules() -> Vec<String> {
    vec![
        "kernel".to_string(),
        "policy".to_string(),
        "extension-host".to_string(),
        "server".to_string(),
        "cli".to_string(),
    ]
}
