//! Extension host loads system extensions and skills from disk.

pub mod registry;

pub use registry::{ExtensionRegistry, RegisteredExtension};

/// Returns the current extension host module name.
pub fn module_name() -> &'static str {
    "extension-host"
}

#[cfg(test)]
mod tests {
    use ennoia_kernel::{ContributionSet, ExtensionKind, ExtensionManifest};

    use crate::ExtensionRegistry;

    #[test]
    fn registry_exposes_items() {
        let registry = ExtensionRegistry::new(vec![ExtensionManifest {
            id: "observatory".to_string(),
            kind: ExtensionKind::SystemExtension,
            version: "0.1.0".to_string(),
            frontend_bundle: Some("frontend/index.js".to_string()),
            backend_entry: Some("backend/index.js".to_string()),
            contributes: ContributionSet::default(),
        }]);

        assert_eq!(registry.items().len(), 1);
    }
}
