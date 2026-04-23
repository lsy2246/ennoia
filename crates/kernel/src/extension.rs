use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::OwnerRef;

use crate::ui::{LocalizedText, ThemeAppearance};

/// ExtensionKind represents the top-level extension classification.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtensionKind {
    #[serde(alias = "extension", alias = "system", alias = "system_extension")]
    SystemExtension,
    #[serde(alias = "skill")]
    Skill,
}

/// PageContribution describes a child page mounted inside the web content area.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageContribution {
    pub id: String,
    pub title: LocalizedText,
    pub route: String,
    pub mount: String,
    pub icon: Option<String>,
    #[serde(default)]
    pub nav: Option<PageNavContribution>,
}

/// PageNavContribution describes optional navigation exposure for an extension page.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageNavContribution {
    #[serde(default)]
    pub default_pinned: bool,
    #[serde(default)]
    pub order: Option<i32>,
}

/// PanelContribution describes a panel mounted in the web dock area.
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

/// CommandContribution describes a web command palette action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandContribution {
    pub id: String,
    pub title: LocalizedText,
    pub action: String,
    pub shortcut: Option<String>,
}

/// ProviderGenerationOption declares optional request controls owned by a provider implementation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderGenerationOption {
    pub id: String,
    pub label: LocalizedText,
    pub value_type: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub default_value: Option<String>,
    #[serde(default)]
    pub allowed_values: Vec<String>,
}

/// ProviderContribution describes a provider capability entry.
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
    #[serde(default)]
    pub generation_options: Vec<ProviderGenerationOption>,
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

/// BehaviorContribution describes one behavior capability exported by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehaviorContribution {
    pub id: String,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub entry: Option<String>,
    pub version: String,
}

/// MemoryContribution describes one memory capability exported by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryContribution {
    pub id: String,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub entry: Option<String>,
    pub version: String,
}

/// InterfaceContribution describes one fine-grained system action implemented by an extension.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceContribution {
    pub key: String,
    pub method: String,
    pub version: String,
    #[serde(default)]
    pub schema: Option<String>,
}

/// ScheduleActionContribution describes one action that can be invoked by the host scheduler.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleActionContribution {
    pub id: String,
    pub method: String,
    pub version: String,
    #[serde(default)]
    pub title: Option<LocalizedText>,
    #[serde(default)]
    pub schema: Option<String>,
}

pub const HOOK_EVENT_CONVERSATION_CREATED: &str = "conversation.created";
pub const HOOK_EVENT_CONVERSATION_MESSAGE_CREATED: &str = "conversation.message.created";
pub const HOOK_EVENT_RUN_REQUESTED: &str = "run.requested";
pub const HOOK_EVENT_RUN_STAGE_CHANGED: &str = "run.stage.changed";
pub const HOOK_EVENT_ARTIFACT_CREATED: &str = "artifact.created";
pub const HOOK_EVENT_JOB_DUE: &str = "job.due";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookResourceRef {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub task_id: Option<String>,
    #[serde(default)]
    pub artifact_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookEventEnvelope {
    pub event: String,
    pub occurred_at: String,
    #[serde(default)]
    pub owner: Option<OwnerRef>,
    pub resource: HookResourceRef,
    #[serde(default)]
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct HookDispatchResponse {
    #[serde(default)]
    pub handled: bool,
    #[serde(default)]
    pub result: Option<JsonValue>,
    #[serde(default)]
    pub message: Option<String>,
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
    pub behaviors: Vec<BehaviorContribution>,
    #[serde(default)]
    pub memories: Vec<MemoryContribution>,
    #[serde(default)]
    pub hooks: Vec<HookContribution>,
    #[serde(default)]
    pub interfaces: Vec<InterfaceContribution>,
    #[serde(default)]
    pub schedule_actions: Vec<ScheduleActionContribution>,
}

/// ExtensionSourceMode identifies whether an extension comes from a development
/// source tree or from a packaged install directory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSourceMode {
    Dev,
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
    pub dev: bool,
}

/// ExtensionUiSpec describes an optional UI module contributed by an extension package.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionUiSpec {
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub dev_url: Option<String>,
    #[serde(default)]
    pub hmr: bool,
}

/// ExtensionWorkerSpec describes an optional sandboxed execution unit.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionWorkerSpec {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub abi: Option<String>,
}

/// ExtensionPermissionSpec declares host capabilities an extension may request.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionPermissionSpec {
    #[serde(default)]
    pub storage: Option<String>,
    #[serde(default)]
    pub sqlite: bool,
    #[serde(default)]
    pub network: Vec<String>,
    #[serde(default)]
    pub events: Vec<String>,
    #[serde(default)]
    pub fs: Vec<String>,
    #[serde(default)]
    pub env: Vec<String>,
}

/// ExtensionRuntimeSpec describes worker lifecycle limits owned by the host.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRuntimeSpec {
    #[serde(default = "default_worker_startup")]
    pub startup: String,
    #[serde(default = "default_worker_timeout_ms")]
    pub timeout_ms: u64,
    #[serde(default = "default_worker_memory_limit_mb")]
    pub memory_limit_mb: u32,
}

impl Default for ExtensionRuntimeSpec {
    fn default() -> Self {
        Self {
            startup: default_worker_startup(),
            timeout_ms: default_worker_timeout_ms(),
            memory_limit_mb: default_worker_memory_limit_mb(),
        }
    }
}

fn default_worker_startup() -> String {
    "lazy".to_string()
}

fn default_worker_timeout_ms() -> u64 {
    30_000
}

fn default_worker_memory_limit_mb() -> u32 {
    128
}

/// ExtensionBuildSpec describes build outputs for release packaging.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionBuildSpec {
    #[serde(default)]
    pub out_dir: Option<String>,
    #[serde(default)]
    pub ui_bundle: Option<String>,
    #[serde(default)]
    pub worker_bundle: Option<String>,
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
    pub behaviors: bool,
    #[serde(default)]
    pub memories: bool,
    #[serde(default)]
    pub hooks: bool,
    #[serde(default)]
    pub interfaces: bool,
    #[serde(default)]
    pub schedule_actions: bool,
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
            behaviors: !contributes.behaviors.is_empty(),
            memories: !contributes.memories.is_empty(),
            hooks: !contributes.hooks.is_empty(),
            interfaces: !contributes.interfaces.is_empty(),
            schedule_actions: !contributes.schedule_actions.is_empty(),
        }
    }
}

/// ExtensionManifest is the canonical descriptor parsed from disk and used by
/// installed packages, built-in packages and development sources.
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
    pub ui: ExtensionUiSpec,
    #[serde(default)]
    pub worker: ExtensionWorkerSpec,
    #[serde(default)]
    pub permissions: ExtensionPermissionSpec,
    #[serde(default)]
    pub runtime: ExtensionRuntimeSpec,
    #[serde(default)]
    pub build: ExtensionBuildSpec,
    #[serde(default)]
    pub assets: ExtensionAssetsSpec,
    #[serde(default)]
    pub watch: ExtensionWatchSpec,
    #[serde(default)]
    pub capabilities: ExtensionCapabilities,
    #[serde(default)]
    pub ui_bundle: Option<String>,
    #[serde(default)]
    pub worker_entry: Option<String>,
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
            behaviors: declared.behaviors || inferred.behaviors,
            memories: declared.memories || inferred.memories,
            hooks: declared.hooks || inferred.hooks,
            interfaces: declared.interfaces || inferred.interfaces,
            schedule_actions: declared.schedule_actions || inferred.schedule_actions,
        }
    }
}

/// ResolvedUiEntry is the runtime-facing UI result after Ennoia has
/// interpreted the descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedUiEntry {
    pub kind: String,
    pub entry: String,
    pub hmr: bool,
}

/// ResolvedWorkerEntry is the runtime-facing Worker result after Ennoia has
/// interpreted the descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedWorkerEntry {
    pub kind: String,
    pub entry: String,
    pub abi: String,
    pub status: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ExtensionRpcRequest {
    #[serde(default)]
    pub params: JsonValue,
    #[serde(default)]
    pub context: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionRpcResponse {
    pub ok: bool,
    #[serde(default)]
    pub data: JsonValue,
    #[serde(default)]
    pub error: Option<ExtensionRpcError>,
}

impl ExtensionRpcResponse {
    pub fn success(data: JsonValue) -> Self {
        Self {
            ok: true,
            data,
            error: None,
        }
    }

    pub fn failure(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            ok: false,
            data: JsonValue::Null,
            error: Some(ExtensionRpcError {
                code: code.into(),
                message: message.into(),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRpcError {
    pub code: String,
    pub message: String,
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
