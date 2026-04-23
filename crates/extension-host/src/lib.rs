//! Extension host loads system extensions and skills from disk.

pub mod registry;
pub mod worker;

pub use registry::{
    read_registry_file, write_registry_file, ExtensionRuntime, ExtensionRuntimeConfig,
    ExtensionRuntimeSnapshot, RegisteredBehaviorContribution, RegisteredCommandContribution,
    RegisteredHookContribution, RegisteredLocaleContribution, RegisteredMemoryContribution,
    RegisteredPageContribution, RegisteredPanelContribution, RegisteredProviderContribution,
    RegisteredThemeContribution, ResolvedExtensionSnapshot,
};

/// Returns the current extension host module name.
pub fn module_name() -> &'static str {
    "extension-host"
}

#[cfg(test)]
mod tests {
    use ennoia_kernel::{ExtensionHealth, ExtensionRuntimeEvent};

    use crate::ExtensionRuntimeSnapshot;

    #[test]
    fn runtime_module_exports_types() {
        let snapshot = ExtensionRuntimeSnapshot {
            generation: 1,
            updated_at: "1".to_string(),
            extensions: Vec::new(),
            pages: Vec::new(),
            panels: Vec::new(),
            themes: Vec::new(),
            locales: Vec::new(),
            commands: Vec::new(),
            providers: Vec::new(),
            behaviors: Vec::new(),
            memories: Vec::new(),
            hooks: Vec::new(),
        };
        let event = ExtensionRuntimeEvent {
            event_id: "evt-1".to_string(),
            extension_id: None,
            generation: 1,
            event: "extension.graph_swapped".to_string(),
            health: Some(ExtensionHealth::Ready),
            summary: "ok".to_string(),
            diagnostics: Vec::new(),
            occurred_at: "1".to_string(),
        };

        assert_eq!(snapshot.generation, 1);
        assert_eq!(event.event, "extension.graph_swapped");
    }
}
