use std::fs;
use std::io;
use std::path::Path;

use ennoia_kernel::{
    CommandContribution, ExtensionKind, ExtensionManifest, HookContribution, PageContribution,
    PanelContribution, ProviderContribution, ThemeContribution,
};
use serde::Serialize;

/// RegisteredExtension represents one installed extension with a resolved install path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredExtension {
    pub manifest: ExtensionManifest,
    pub install_dir: String,
}

/// RegisteredPageContribution describes one mounted page with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredExtensionSnapshot {
    pub id: String,
    pub kind: ExtensionKind,
    pub version: String,
    pub install_dir: String,
    pub frontend_bundle: Option<String>,
    pub backend_entry: Option<String>,
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub commands: Vec<CommandContribution>,
    pub providers: Vec<ProviderContribution>,
    pub hooks: Vec<HookContribution>,
}

/// RegisteredPageContribution describes one mounted page with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPageContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub page: PageContribution,
}

/// RegisteredPanelContribution describes one mounted panel with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredPanelContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub panel: PanelContribution,
}

/// RegisteredThemeContribution describes one theme contribution with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredThemeContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub theme: ThemeContribution,
}

/// RegisteredCommandContribution describes one command contribution with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredCommandContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub command: CommandContribution,
}

/// RegisteredProviderContribution describes one provider contribution with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredProviderContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub provider: ProviderContribution,
}

/// RegisteredHookContribution describes one hook contribution with its extension metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RegisteredHookContribution {
    pub extension_id: String,
    pub extension_kind: ExtensionKind,
    pub extension_version: String,
    pub install_dir: String,
    pub hook: HookContribution,
}

/// ExtensionRegistrySnapshot is the server-facing representation of the full registry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ExtensionRegistrySnapshot {
    pub extensions: Vec<RegisteredExtensionSnapshot>,
    pub pages: Vec<RegisteredPageContribution>,
    pub panels: Vec<RegisteredPanelContribution>,
    pub themes: Vec<RegisteredThemeContribution>,
    pub commands: Vec<RegisteredCommandContribution>,
    pub providers: Vec<RegisteredProviderContribution>,
    pub hooks: Vec<RegisteredHookContribution>,
}

/// ExtensionRegistry keeps the registered extension manifests in memory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ExtensionRegistry {
    items: Vec<RegisteredExtension>,
}

impl ExtensionRegistry {
    pub fn new(manifests: Vec<ExtensionManifest>) -> Self {
        let items = manifests
            .into_iter()
            .map(|manifest| RegisteredExtension {
                install_dir: format!("~/.ennoia/global/extensions/{}", manifest.id),
                manifest,
            })
            .collect();
        Self { items }
    }

    pub fn from_registered(items: Vec<RegisteredExtension>) -> Self {
        Self { items }
    }

    pub fn scan_install_dir(path: impl AsRef<Path>) -> io::Result<Self> {
        let mut items = Vec::new();
        let path = path.as_ref();
        if !path.exists() {
            return Ok(Self { items });
        }

        for entry in fs::read_dir(path)? {
            let entry = entry?;
            if !entry.file_type()?.is_dir() {
                continue;
            }
            let manifest_path = entry.path().join("manifest.toml");
            if !manifest_path.exists() {
                continue;
            }

            let contents = fs::read_to_string(&manifest_path)?;
            let manifest: ExtensionManifest =
                toml::from_str(&contents).map_err(io::Error::other)?;
            items.push(RegisteredExtension {
                manifest,
                install_dir: entry.path().display().to_string(),
            });
        }

        Ok(Self { items })
    }

    pub fn items(&self) -> &[RegisteredExtension] {
        &self.items
    }

    pub fn snapshot(&self) -> ExtensionRegistrySnapshot {
        ExtensionRegistrySnapshot {
            extensions: self
                .items
                .iter()
                .map(RegisteredExtensionSnapshot::from_extension)
                .collect(),
            pages: self.pages(),
            panels: self.panels(),
            themes: self.themes(),
            commands: self.commands(),
            providers: self.providers(),
            hooks: self.hooks(),
        }
    }

    pub fn pages(&self) -> Vec<RegisteredPageContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .pages
                    .iter()
                    .cloned()
                    .map(|page| RegisteredPageContribution::from_extension(item, page))
            })
            .collect()
    }

    pub fn panels(&self) -> Vec<RegisteredPanelContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .panels
                    .iter()
                    .cloned()
                    .map(|panel| RegisteredPanelContribution::from_extension(item, panel))
            })
            .collect()
    }

    pub fn themes(&self) -> Vec<RegisteredThemeContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .themes
                    .iter()
                    .cloned()
                    .map(|theme| RegisteredThemeContribution::from_extension(item, theme))
            })
            .collect()
    }

    pub fn commands(&self) -> Vec<RegisteredCommandContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .commands
                    .iter()
                    .cloned()
                    .map(|command| RegisteredCommandContribution::from_extension(item, command))
            })
            .collect()
    }

    pub fn providers(&self) -> Vec<RegisteredProviderContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .providers
                    .iter()
                    .cloned()
                    .map(|provider| RegisteredProviderContribution::from_extension(item, provider))
            })
            .collect()
    }

    pub fn hooks(&self) -> Vec<RegisteredHookContribution> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .hooks
                    .iter()
                    .cloned()
                    .map(|hook| RegisteredHookContribution::from_extension(item, hook))
            })
            .collect()
    }

    pub fn page_ids(&self) -> Vec<String> {
        self.pages().into_iter().map(|page| page.page.id).collect()
    }
}

impl RegisteredPageContribution {
    fn from_extension(item: &RegisteredExtension, page: PageContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            page,
        }
    }
}

impl RegisteredExtensionSnapshot {
    fn from_extension(item: &RegisteredExtension) -> Self {
        Self {
            id: item.manifest.id.clone(),
            kind: item.manifest.kind.clone(),
            version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            frontend_bundle: item.manifest.frontend_bundle.clone(),
            backend_entry: item.manifest.backend_entry.clone(),
            pages: item.manifest.contributes.pages.clone(),
            panels: item.manifest.contributes.panels.clone(),
            themes: item.manifest.contributes.themes.clone(),
            commands: item.manifest.contributes.commands.clone(),
            providers: item.manifest.contributes.providers.clone(),
            hooks: item.manifest.contributes.hooks.clone(),
        }
    }
}

impl RegisteredPanelContribution {
    fn from_extension(item: &RegisteredExtension, panel: PanelContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            panel,
        }
    }
}

impl RegisteredThemeContribution {
    fn from_extension(item: &RegisteredExtension, theme: ThemeContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            theme,
        }
    }
}

impl RegisteredCommandContribution {
    fn from_extension(item: &RegisteredExtension, command: CommandContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            command,
        }
    }
}

impl RegisteredProviderContribution {
    fn from_extension(item: &RegisteredExtension, provider: ProviderContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            provider,
        }
    }
}

impl RegisteredHookContribution {
    fn from_extension(item: &RegisteredExtension, hook: HookContribution) -> Self {
        Self {
            extension_id: item.manifest.id.clone(),
            extension_kind: item.manifest.kind.clone(),
            extension_version: item.manifest.version.clone(),
            install_dir: item.install_dir.clone(),
            hook,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ennoia_kernel::ContributionSet;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn registry_snapshot_flattens_contributions() {
        let registry = ExtensionRegistry::new(vec![sample_manifest()]);
        let snapshot = registry.snapshot();

        assert_eq!(snapshot.extensions.len(), 1);
        assert_eq!(snapshot.pages.len(), 1);
        assert_eq!(snapshot.panels.len(), 1);
        assert_eq!(snapshot.commands.len(), 1);
        assert_eq!(snapshot.providers.len(), 1);
        assert_eq!(snapshot.hooks.len(), 1);
        assert_eq!(snapshot.pages[0].page.mount, "observatory.events.page");
        assert_eq!(snapshot.panels[0].panel.slot, "right");
    }

    #[test]
    fn scan_install_dir_reads_manifest_and_path() {
        let root = unique_test_dir("extension-registry");
        let extension_dir = root.join("observatory");
        fs::create_dir_all(&extension_dir).expect("create extension dir");
        fs::write(
            extension_dir.join("manifest.toml"),
            toml::to_string(&sample_manifest()).expect("serialize manifest"),
        )
        .expect("write manifest");

        let registry = ExtensionRegistry::scan_install_dir(&root).expect("scan registry");

        assert_eq!(registry.items().len(), 1);
        assert!(registry.items()[0].install_dir.contains("observatory"));

        fs::remove_dir_all(&root).expect("cleanup test dir");
    }

    fn sample_manifest() -> ExtensionManifest {
        ExtensionManifest {
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
                themes: vec![ThemeContribution {
                    id: "observatory.daybreak".to_string(),
                    label: "Daybreak".to_string(),
                    entry: Some("frontend/themes/daybreak.css".to_string()),
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
            },
        }
    }

    fn unique_test_dir(prefix: &str) -> std::path::PathBuf {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time")
            .as_nanos();
        std::env::temp_dir().join(format!("ennoia-{prefix}-{suffix}"))
    }
}
