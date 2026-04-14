//! Kernel defines Ennoia's shared domain model and contracts.

pub mod config;
pub mod domain;
pub mod extension;
pub mod runtime;

pub use config::{AgentConfig, AppConfig, ServerConfig, UiConfig};
pub use domain::{
    AgentSpec, ArtifactKind, ArtifactSpec, MessageSpec, OwnerKind, OwnerRef, RunSpec, RunStatus,
    SpaceSpec, TaskSpec, TaskStatus, ThreadKind, ThreadSpec,
};
pub use extension::{
    CommandContribution, ContributionSet, ExtensionKind, ExtensionManifest, HookContribution,
    PageContribution, PanelContribution, ProviderContribution, ThemeContribution,
};
pub use runtime::{core_modules, PlatformOverview};

/// Returns the current kernel module name.
pub fn module_name() -> &'static str {
    "kernel"
}

/// Returns the Ennoia platform name.
pub fn platform_name() -> &'static str {
    "Ennoia"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_overview_lists_core_modules() {
        let overview = PlatformOverview::default();
        assert!(overview.modules.contains(&"kernel".to_string()));
        assert_eq!(overview.app_name, "Ennoia");
    }
}
