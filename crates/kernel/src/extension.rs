use serde::{Deserialize, Serialize};

/// ExtensionKind represents the top-level extension classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtensionKind {
    SystemExtension,
    Skill,
}

/// PageContribution describes a child page mounted inside the shell content area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageContribution {
    pub id: String,
    pub title: String,
    pub route: String,
}

/// PanelContribution describes a panel mounted in the shell dock area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelContribution {
    pub id: String,
    pub title: String,
    pub mount: String,
}

/// ThemeContribution describes a UI contribution that can become the active theme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeContribution {
    pub id: String,
    pub label: String,
}

/// CommandContribution describes a shell command palette action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    pub action: String,
}

/// ProviderContribution describes a backend or frontend provider entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderContribution {
    pub id: String,
    pub kind: String,
}

/// HookContribution describes one event name exported by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookContribution {
    pub event: String,
}

/// ContributionSet groups all extension contributions in one place.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionSet {
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub commands: Vec<CommandContribution>,
    pub providers: Vec<ProviderContribution>,
    pub hooks: Vec<HookContribution>,
}

/// ExtensionManifest is the canonical manifest parsed from disk.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionManifest {
    pub id: String,
    pub kind: ExtensionKind,
    pub version: String,
    pub frontend_bundle: Option<String>,
    pub backend_entry: Option<String>,
    #[serde(default)]
    pub contributes: ContributionSet,
}
