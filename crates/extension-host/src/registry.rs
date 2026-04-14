use std::fs;
use std::io;
use std::path::Path;

use ennoia_kernel::ExtensionManifest;

/// RegisteredExtension represents one installed extension with a resolved install path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegisteredExtension {
    pub manifest: ExtensionManifest,
    pub install_dir: String,
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

    pub fn page_ids(&self) -> Vec<String> {
        self.items
            .iter()
            .flat_map(|item| {
                item.manifest
                    .contributes
                    .pages
                    .iter()
                    .map(|page| page.id.clone())
            })
            .collect()
    }
}
