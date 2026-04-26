use ennoia_error_utils::normalize_error_message;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::ui::{LocalizedText, ThemeAppearance};
use crate::OwnerRef;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ExtensionKind {
    #[serde(alias = "extension", alias = "system", alias = "system_extension")]
    SystemExtension,
    #[serde(alias = "skill")]
    Skill,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PageNavContribution {
    #[serde(default)]
    pub default_pinned: bool,
    #[serde(default)]
    pub order: Option<i32>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PanelContribution {
    pub id: String,
    pub title: LocalizedText,
    pub mount: String,
    pub slot: String,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ThemeContribution {
    pub id: String,
    pub label: LocalizedText,
    pub appearance: ThemeAppearance,
    pub tokens_entry: String,
    #[serde(default)]
    pub contract: Option<String>,
    pub preview_color: Option<String>,
    pub extends: Option<String>,
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LocaleContribution {
    pub locale: String,
    pub namespace: String,
    pub entry: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CommandContribution {
    pub id: String,
    pub title: LocalizedText,
    pub action: String,
    pub shortcut: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResourceTypeContribution {
    pub id: String,
    #[serde(default)]
    pub title: Option<LocalizedText>,
    pub content_kind: String,
    #[serde(default)]
    pub metadata_schema: Option<String>,
    #[serde(default)]
    pub content_schema: Option<String>,
    #[serde(default)]
    pub operations: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CapabilityContribution {
    pub id: String,
    pub contract: String,
    pub kind: String,
    #[serde(default)]
    pub title: Option<LocalizedText>,
    #[serde(default)]
    pub runtime: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub input_schema: Option<String>,
    #[serde(default)]
    pub output_schema: Option<String>,
    #[serde(default)]
    pub consumes: Vec<String>,
    #[serde(default)]
    pub produces: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub emits: Vec<String>,
    #[serde(default)]
    pub metadata: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SurfaceContribution {
    pub id: String,
    pub kind: String,
    pub mount: String,
    #[serde(default)]
    pub title: Option<LocalizedText>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub slot: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub nav: Option<PageNavContribution>,
    #[serde(default)]
    pub match_resource_types: Vec<String>,
    #[serde(default)]
    pub match_capability_contracts: Vec<String>,
    #[serde(default)]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SubscriptionContribution {
    pub event: String,
    pub capability: String,
    #[serde(default)]
    pub match_resource_types: Vec<String>,
    #[serde(default)]
    pub match_capability_ids: Vec<String>,
    #[serde(default)]
    pub match_capability_contracts: Vec<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HookContribution {
    pub event: String,
    pub handler: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BehaviorContribution {
    pub id: String,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryContribution {
    pub id: String,
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub entry: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InterfaceContribution {
    pub key: String,
    pub method: String,
    #[serde(default)]
    pub schema: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleActionContribution {
    pub id: String,
    pub method: String,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionCapabilities {
    #[serde(default)]
    pub resource_types: bool,
    #[serde(default)]
    pub capabilities: bool,
    #[serde(default)]
    pub surfaces: bool,
    #[serde(default)]
    pub locales: bool,
    #[serde(default)]
    pub themes: bool,
    #[serde(default)]
    pub commands: bool,
    #[serde(default)]
    pub subscriptions: bool,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionSourceSpec {
    #[serde(default)]
    pub mode: ExtensionSourceMode,
    #[serde(default)]
    pub root: Option<String>,
    #[serde(default)]
    pub dev: bool,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionWorkerSpec {
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub entry: Option<String>,
    #[serde(default)]
    pub abi: Option<String>,
    #[serde(default)]
    pub protocol: Option<String>,
}

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionBuildSpec {
    #[serde(default)]
    pub out_dir: Option<String>,
    #[serde(default)]
    pub ui_bundle: Option<String>,
    #[serde(default)]
    pub worker_bundle: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionAssetsSpec {
    #[serde(default)]
    pub locales_dir: Option<String>,
    #[serde(default)]
    pub themes_dir: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionWatchSpec {
    #[serde(default)]
    pub include: Vec<String>,
    #[serde(default)]
    pub exclude: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionManifest {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub kind: ExtensionKind,
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
    pub ui_bundle: Option<String>,
    #[serde(default)]
    pub worker_entry: Option<String>,
    #[serde(default)]
    pub resource_types: Vec<ResourceTypeContribution>,
    #[serde(default)]
    pub capabilities: Vec<CapabilityContribution>,
    #[serde(default)]
    pub surfaces: Vec<SurfaceContribution>,
    #[serde(default)]
    pub locales: Vec<LocaleContribution>,
    #[serde(default)]
    pub themes: Vec<ThemeContribution>,
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub subscriptions: Vec<SubscriptionContribution>,
}

impl ExtensionManifest {
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }

    pub fn effective_capabilities(&self) -> ExtensionCapabilities {
        ExtensionCapabilities {
            resource_types: !self.resource_types.is_empty(),
            capabilities: !self.capabilities.is_empty(),
            surfaces: !self.surfaces.is_empty(),
            locales: !self.locales.is_empty(),
            themes: !self.themes.is_empty(),
            commands: !self.commands.is_empty(),
            subscriptions: !self.subscriptions.is_empty(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedUiEntry {
    pub kind: String,
    pub entry: String,
    pub hmr: bool,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResolvedWorkerEntry {
    pub kind: String,
    pub entry: String,
    pub abi: String,
    #[serde(default)]
    pub protocol: Option<String>,
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
                message: normalize_error_message(message.into()),
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRpcError {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionDiagnostic {
    pub level: String,
    pub summary: String,
    #[serde(default)]
    pub detail: Option<String>,
    pub at: String,
}

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
