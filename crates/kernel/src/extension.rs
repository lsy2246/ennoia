use serde::{Deserialize, Serialize};

use crate::ui::{LocalizedText, ThemeAppearance};

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
    pub title: LocalizedText,
    pub route: String,
    pub mount: String,
    pub icon: Option<String>,
}

/// PanelContribution describes a panel mounted in the shell dock area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelContribution {
    pub id: String,
    pub title: LocalizedText,
    pub mount: String,
    pub slot: String,
    pub icon: Option<String>,
}

/// ThemeContribution describes a UI contribution that can become the active theme.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeContribution {
    pub id: String,
    pub label: LocalizedText,
    pub appearance: ThemeAppearance,
    pub tokens_entry: String,
    pub preview_color: Option<String>,
    pub extends: Option<String>,
    pub category: Option<String>,
}

/// LocaleContribution describes one locale bundle provided by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocaleContribution {
    pub locale: String,
    pub namespace: String,
    pub entry: String,
    pub version: String,
}

/// CommandContribution describes a shell command palette action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandContribution {
    pub id: String,
    pub title: LocalizedText,
    pub action: String,
    pub shortcut: Option<String>,
}

/// ProviderContribution describes a backend or frontend provider entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderContribution {
    pub id: String,
    pub kind: String,
    pub entry: Option<String>,
}

/// HookContribution describes one event name exported by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookContribution {
    pub event: String,
    pub handler: Option<String>,
}

/// ContributionSet groups all extension contributions in one place.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContributionSet {
    pub pages: Vec<PageContribution>,
    pub panels: Vec<PanelContribution>,
    pub themes: Vec<ThemeContribution>,
    pub locales: Vec<LocaleContribution>,
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
