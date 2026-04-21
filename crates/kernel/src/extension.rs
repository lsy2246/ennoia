use serde::{Deserialize, Serialize};

use crate::ui::{LocalizedText, ThemeAppearance};

/// ExtensionKind represents the top-level extension classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtensionKind {
    #[serde(alias = "extension", alias = "system", alias = "system_extension")]
    SystemExtension,
    #[serde(alias = "skill")]
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
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub model_discovery: bool,
    #[serde(default)]
    pub recommended_model: Option<String>,
    #[serde(default = "default_manual_model")]
    pub manual_model: bool,
}

fn default_manual_model() -> bool {
    true
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
    #[serde(default)]
    pub pages: Vec<PageContribution>,
    #[serde(default)]
    pub panels: Vec<PanelContribution>,
    #[serde(default)]
    pub themes: Vec<ThemeContribution>,
    #[serde(default)]
    pub locales: Vec<LocaleContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub providers: Vec<ProviderContribution>,
    #[serde(default)]
    pub hooks: Vec<HookContribution>,
}

/// ExtensionSourceMode identifies whether an extension comes from a workspace
/// source tree or from a packaged install directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSourceMode {
    Workspace,
    Package,
}

impl Default for ExtensionSourceMode {
    fn default() -> Self {
        Self::Package
    }
}

/// ExtensionHealth represents the runtime health state of one extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionHealth {
    Discovering,
    Resolving,
    Ready,
    Degraded,
    Failed,
    Stopped,
}

/// ExtensionSourceSpec describes where the descriptor is resolved from.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionSourceSpec {
    #[serde(default)]
    pub mode: ExtensionSourceMode,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub workspace: bool,
}

/// ExtensionFrontendSpec describes the logical frontend entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionFrontendSpec {
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub dev_url: Option<String>,
    #[serde(default)]
    pub dev_command: Option<String>,
    #[serde(default)]
    pub hmr: bool,
}

/// ExtensionBackendSpec describes the logical backend entry.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionBackendSpec {
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub dev_command: Option<String>,
    #[serde(default)]
    pub healthcheck: Option<String>,
    #[serde(default)]
    pub restart: Option<String>,
}

/// ExtensionBuildSpec describes build outputs for release packaging.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionBuildSpec {
    #[serde(default)]
    pub out_dir: Option<String>,
    #[serde(default)]
    pub frontend_bundle: Option<String>,
    #[serde(default)]
    pub backend_bundle: Option<String>,
}

/// ExtensionAssetsSpec describes asset roots.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionAssetsSpec {
    #[serde(default)]
    pub locales_dir: Option<String>,
    #[serde(default)]
    pub themes_dir: Option<String>,
}

/// ExtensionWatchSpec describes development watch patterns.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionWatchSpec {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

/// ExtensionCapabilities lets descriptors declare which contribution families
/// they intend to expose at runtime.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionCapabilities {
    #[serde(default)]
    pub pages: bool,
    #[serde(default)]
    pub panels: bool,
    #[serde(default)]
    pub themes: bool,
    #[serde(default)]
    pub locales: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub providers: bool,
    #[serde(default)]
    pub hooks: bool,
}

impl ExtensionCapabilities {
    pub fn from_contributions(contributes: &ContributionSet) -> Self {
        Self {
            pages: !contributes.pages.is_empty(),
            panels: !contributes.panels.is_empty(),
            themes: !contributes.themes.is_empty(),
            locales: !contributes.locales.is_empty(),
            commands: !contributes.commands.is_empty(),
            providers: !contributes.providers.is_empty(),
            hooks: !contributes.hooks.is_empty(),
        }
    }
}

/// ExtensionManifest is the canonical descriptor parsed from disk. It supports
/// both the old manifest fields and the new workspace/package runtime fields.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionManifest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub kind: ExtensionKind,
    pub version: String,
    #[serde(default)]
    pub source: ExtensionSourceSpec,
    #[serde(default)]
    pub frontend: ExtensionFrontendSpec,
    #[serde(default)]
    pub backend: ExtensionBackendSpec,
    #[serde(default)]
    pub build: ExtensionBuildSpec,
    #[serde(default)]
    pub assets: ExtensionAssetsSpec,
    #[serde(default)]
    pub watch: ExtensionWatchSpec,
    #[serde(default)]
    pub capabilities: ExtensionCapabilities,
    #[serde(default)]
    pub frontend_bundle: Option<String>,
    #[serde(default)]
    pub backend_entry: Option<String>,
    #[serde(default)]
    pub contributes: ContributionSet,
}

impl ExtensionManifest {
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }

    pub fn effective_capabilities(&self) -> ExtensionCapabilities {
        let declared = self.capabilities.clone();
        let inferred = ExtensionCapabilities::from_contributions(&self.contributes);
        ExtensionCapabilities {
            pages: declared.pages || inferred.pages,
            panels: declared.panels || inferred.panels,
            themes: declared.themes || inferred.themes,
            locales: declared.locales || inferred.locales,
            commands: declared.commands || inferred.commands,
            providers: declared.providers || inferred.providers,
            hooks: declared.hooks || inferred.hooks,
        }
    }
}

/// ResolvedFrontendEntry is the runtime-facing frontend result after Ennoia has
/// interpreted the descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedFrontendEntry {
    pub kind: String,
    pub entry: String,
    pub hmr: bool,
}

/// ResolvedBackendEntry is the runtime-facing backend result after Ennoia has
/// interpreted the descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedBackendEntry {
    pub kind: String,
    pub runtime: String,
    pub entry: String,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub healthcheck: Option<String>,
    pub status: String,
    #[serde(default)]
    pub pid: Option<u32>,
}

/// ExtensionDiagnostic records one resolution or runtime observation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDiagnostic {
    pub level: String,
    pub summary: String,
    #[serde(default)]
    pub detail: Option<String>,
    pub at: String,
}

/// ExtensionRuntimeEvent records one runtime event visible to API, CLI and Web.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRuntimeEvent {
    pub event_id: String,
    #[serde(default)]
    pub extension_id: Option<String>,
    pub generation: u64,
    pub event: String,
    #[serde(default)]
    pub health: Option<ExtensionHealth>,
    pub summary: String,
    #[serde(default)]
    pub diagnostics: Vec<ExtensionDiagnostic>,
    pub occurred_at: String,
}
