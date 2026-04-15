//! Extension host loads system extensions and skills from disk.

pub mod registry;

pub use registry::{
    ExtensionRegistry, ExtensionRegistrySnapshot, RegisteredCommandContribution,
    RegisteredExtension, RegisteredExtensionSnapshot, RegisteredHookContribution,
    RegisteredPageContribution, RegisteredPanelContribution, RegisteredProviderContribution,
    RegisteredThemeContribution,
};

/// Returns the current extension host module name.
pub fn module_name() -> &'static str {
    "extension-host"
}

#[cfg(test)]
mod tests {
    use ennoia_kernel::{
        CommandContribution, ContributionSet, ExtensionKind, ExtensionManifest, HookContribution,
        PageContribution, PanelContribution, ProviderContribution,
    };

    use crate::ExtensionRegistry;

    #[test]
    fn registry_exposes_items() {
        let registry = ExtensionRegistry::new(vec![ExtensionManifest {
            id: "observatory".to_string(),
            kind: ExtensionKind::SystemExtension,
            version: "0.1.0".to_string(),
            frontend_bundle: Some("frontend/index.js".to_string()),
            backend_entry: Some("backend/index.js".to_string()),
            contributes: ContributionSet {
                pages: vec![PageContribution {
                    id: "observatory.events".to_string(),
                    title: "Observatory".to_string(),
                    route: "/observatory".to_string(),
                    mount: "observatory.events.page".to_string(),
                    icon: Some("activity".to_string()),
                }],
                panels: vec![PanelContribution {
                    id: "observatory.timeline".to_string(),
                    title: "Event Timeline".to_string(),
                    mount: "observatory.timeline.panel".to_string(),
                    slot: "right".to_string(),
                    icon: Some("panel-right".to_string()),
                }],
                commands: vec![CommandContribution {
                    id: "observatory.open".to_string(),
                    title: "Open Observatory".to_string(),
                    action: "open-page".to_string(),
                    shortcut: Some("Ctrl+Shift+O".to_string()),
                }],
                providers: vec![ProviderContribution {
                    id: "observatory.feed".to_string(),
                    kind: "activity-feed".to_string(),
                    entry: Some("backend/providers/activity-feed.js".to_string()),
                }],
                hooks: vec![HookContribution {
                    event: "run.completed".to_string(),
                    handler: Some("backend/hooks/run-completed.js".to_string()),
                }],
                ..ContributionSet::default()
            },
        }]);

        assert_eq!(registry.items().len(), 1);
        assert_eq!(registry.pages().len(), 1);
        assert_eq!(registry.panels().len(), 1);
    }
}
