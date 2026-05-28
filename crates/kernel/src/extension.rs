use ennoia_error_utils::normalize_error_message;
use serde::{de::Deserializer, Deserialize, Serialize};
use serde_json::Value as JsonValue;

use crate::ui::{LocalizedText, ThemeAppearance};
use crate::OwnerRef;

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
#[serde(deny_unknown_fields)]
pub struct MessageRendererContribution {
    pub id: String,
    pub format: String,
    pub mount: String,
    #[serde(default)]
    pub priority: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionCompatSpec {
    #[serde(default)]
    pub ennoia: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionViewSpec {
    pub name: String,
    #[serde(rename = "type")]
    pub view_type: String,
    pub title: LocalizedText,
    #[serde(default)]
    pub nav: Option<String>,
    #[serde(default)]
    pub order: Option<i32>,
    #[serde(default)]
    pub slot: Option<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub route: Option<String>,
    #[serde(default)]
    pub priority: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionProviderSpec {
    pub kind: String,
    #[serde(default)]
    pub interfaces: Vec<String>,
    #[serde(default)]
    pub model_discovery: bool,
    #[serde(default = "default_manual_model")]
    pub manual_model: bool,
    #[serde(default)]
    pub generation_options: Vec<ProviderGenerationOption>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionOperationSpec {
    pub name: String,
    #[serde(default)]
    pub title: Option<LocalizedText>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub agent: bool,
    #[serde(default)]
    pub input: Option<String>,
    #[serde(default)]
    pub output: Option<String>,
    #[serde(default)]
    pub provider: Option<ExtensionProviderSpec>,
    #[serde(default)]
    pub schedule: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionEventSpec {
    pub on: String,
    pub operation: String,
    #[serde(default)]
    pub priority: i32,
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

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ProviderModelDescriptor {
    pub id: String,
    #[serde(default)]
    pub max_context_tokens: Option<u32>,
    #[serde(default)]
    pub max_input_tokens: Option<u32>,
}

impl<'de> Deserialize<'de> for ProviderModelDescriptor {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(untagged)]
        enum ProviderModelDescriptorInput {
            Id(String),
            Object {
                id: String,
                #[serde(default)]
                max_context_tokens: Option<u32>,
                #[serde(default)]
                max_input_tokens: Option<u32>,
            },
        }

        match ProviderModelDescriptorInput::deserialize(deserializer)? {
            ProviderModelDescriptorInput::Id(id) => Ok(Self {
                id,
                max_context_tokens: None,
                max_input_tokens: None,
            }),
            ProviderModelDescriptorInput::Object {
                id,
                max_context_tokens,
                max_input_tokens,
            } => Ok(Self {
                id,
                max_context_tokens,
                max_input_tokens,
            }),
        }
    }
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
    #[serde(default)]
    pub priority: i32,
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
#[serde(rename_all = "snake_case")]
pub enum ActionPhase {
    Before,
    Execute,
    AfterSuccess,
    AfterError,
}

impl Default for ActionPhase {
    fn default() -> Self {
        Self::Execute
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ActionResultMode {
    Void,
    First,
    Last,
    Collect,
    Merge,
}

impl Default for ActionResultMode {
    fn default() -> Self {
        Self::Last
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActionRule {
    pub action: String,
    pub operation: String,
    pub method: String,
    #[serde(default)]
    pub phase: ActionPhase,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub result_mode: ActionResultMode,
    #[serde(default)]
    pub when: JsonValue,
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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionConversationSpec {
    #[serde(default)]
    pub visible: bool,
    #[serde(default)]
    pub resources: Vec<String>,
    #[serde(default)]
    pub operations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExtensionSettingFieldType {
    Text,
    Textarea,
    Boolean,
    Select,
    Number,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionSettingOptionSpec {
    pub label: LocalizedText,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum ExtensionSettingValue {
    String(String),
    Integer(i64),
    Boolean(bool),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionSettingFieldSpec {
    pub key: String,
    pub label: LocalizedText,
    #[serde(default)]
    pub description: Option<LocalizedText>,
    #[serde(rename = "type")]
    pub field_type: ExtensionSettingFieldType,
    #[serde(default)]
    pub placeholder: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub options: Vec<ExtensionSettingOptionSpec>,
    #[serde(default)]
    pub default_value: Option<ExtensionSettingValue>,
}

pub const HOOK_EVENT_CONVERSATION_CREATED: &str = "conversation.created";
pub const HOOK_EVENT_CONVERSATION_MESSAGE_CREATED: &str = "conversation.message.created";
pub const HOOK_EVENT_OPERATION_UPDATED: &str = "operation.updated";
pub const HOOK_EVENT_PERMISSION_APPROVAL_RESOLVED: &str = "permission.approval.resolved";
pub const HOOK_EVENT_RUN_REQUESTED: &str = "run.requested";
pub const HOOK_EVENT_RUN_STAGE_CHANGED: &str = "run.stage.changed";
pub const HOOK_EVENT_ARTIFACT_CREATED: &str = "artifact.created";
pub const HOOK_EVENT_JOB_DUE: &str = "job.due";

fn default_enabled() -> bool {
    true
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HookEventPublishRequest {
    pub event: String,
    pub resource_kind: String,
    pub resource_id: String,
    #[serde(default)]
    pub payload: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionStateEntry {
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub key: String,
    pub value: JsonValue,
    pub version: i64,
    pub updated_at: String,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionStateSelector {
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionRecordEntry {
    pub id: String,
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub kind: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub payload: JsonValue,
    #[serde(default)]
    pub related_message_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    #[serde(default)]
    pub closed_at: Option<String>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ExtensionManifest {
    pub id: String,
    #[serde(default)]
    pub version: Option<String>,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub docs: Option<String>,
    #[serde(default)]
    pub compat: ExtensionCompatSpec,
    #[serde(default)]
    pub views: Vec<ExtensionViewSpec>,
    #[serde(default)]
    pub operations: Vec<ExtensionOperationSpec>,
    #[serde(default)]
    pub events: Vec<ExtensionEventSpec>,
    #[serde(default)]
    pub settings: Vec<ExtensionSettingFieldSpec>,
    #[serde(default)]
    pub message_renderers: Vec<MessageRendererContribution>,
    #[serde(default)]
    pub conversation: ExtensionConversationSpec,
}

impl ExtensionManifest {
    pub fn display_name(&self) -> String {
        self.name.clone().unwrap_or_else(|| self.id.clone())
    }

    pub fn display_description(&self) -> String {
        self.description.clone().unwrap_or_default()
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
        Self::failure_with_details(code, message, None)
    }

    pub fn failure_with_details(
        code: impl Into<String>,
        message: impl Into<String>,
        details: Option<JsonValue>,
    ) -> Self {
        Self {
            ok: false,
            data: JsonValue::Null,
            error: Some(ExtensionRpcError {
                code: code.into(),
                message: normalize_error_message(message.into()),
                details,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRpcError {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionStateGetQuery {
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionStateListQuery {
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub key: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionStatePut {
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub key: String,
    pub value: JsonValue,
    #[serde(default)]
    pub expires_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionRecordAppend {
    pub extension_id: String,
    pub namespace: String,
    pub scope_type: String,
    pub scope_id: String,
    pub kind: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub payload: JsonValue,
    #[serde(default)]
    pub related_message_id: Option<String>,
    #[serde(default)]
    pub parent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExtensionRecordUpdate {
    pub id: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub payload: Option<JsonValue>,
    #[serde(default)]
    pub related_message_id: Option<Option<String>>,
    #[serde(default)]
    pub parent_id: Option<Option<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExtensionRecordListQuery {
    #[serde(default)]
    pub extension_id: Option<String>,
    #[serde(default)]
    pub namespace: Option<String>,
    #[serde(default)]
    pub scope_type: Option<String>,
    #[serde(default)]
    pub scope_id: Option<String>,
    #[serde(default)]
    pub kind: Option<String>,
    #[serde(default)]
    pub related_message_id: Option<String>,
    #[serde(default)]
    pub open_only: Option<bool>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RuntimeOperationRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub run_id: String,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub arguments: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OperationStatus {
    Queued,
    Running,
    Blocked,
    Succeeded,
    Failed,
    Cancelled,
}

impl OperationStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Queued => "queued",
            Self::Running => "running",
            Self::Blocked => "blocked",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationPerformRequest {
    pub agent_id: String,
    pub conversation_id: String,
    pub run_id: String,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub lane_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
    pub kind: String,
    pub name: String,
    #[serde(default)]
    pub deferred: bool,
    #[serde(default)]
    pub input: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationRecord {
    pub id: String,
    pub extension_id: String,
    pub agent_id: String,
    pub conversation_id: String,
    #[serde(default)]
    pub branch_id: Option<String>,
    #[serde(default)]
    pub lane_id: Option<String>,
    pub run_id: String,
    #[serde(default)]
    pub message_id: Option<String>,
    pub kind: String,
    pub name: String,
    pub status: OperationStatus,
    #[serde(default)]
    pub input: JsonValue,
    #[serde(default)]
    pub output: Option<JsonValue>,
    #[serde(default)]
    pub error: Option<JsonValue>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationEventRecord {
    pub id: String,
    pub operation_id: String,
    pub conversation_id: String,
    pub event: String,
    #[serde(default)]
    pub payload: JsonValue,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperationApprovalLink {
    pub operation_id: String,
    pub approval_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct OperationListQuery {
    #[serde(default)]
    pub conversation_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OperationPerformResponse {
    pub operation: OperationRecord,
    #[serde(default)]
    pub content: JsonValue,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ExtensionHostCapabilityRequest {
    ExtensionsRuntimeSnapshot,
    ActionDispatch {
        action: String,
        #[serde(default)]
        params: JsonValue,
        #[serde(default)]
        context: JsonValue,
    },
    ProviderInvoke {
        provider_kind: String,
        method: String,
        payload: ExtensionRpcRequest,
    },
    RuntimeOperation {
        operation: String,
        payload: RuntimeOperationRequest,
    },
    HookEventPublish {
        payload: HookEventPublishRequest,
    },
    OperationPerform {
        payload: OperationPerformRequest,
    },
    ExtensionStateGet {
        query: ExtensionStateGetQuery,
    },
    ExtensionStatePut {
        payload: ExtensionStatePut,
    },
    ExtensionStateDelete {
        query: ExtensionStateGetQuery,
    },
    ExtensionRecordAppend {
        payload: ExtensionRecordAppend,
    },
    ExtensionRecordUpdate {
        payload: ExtensionRecordUpdate,
    },
    ExtensionRecordClose {
        record_id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ProcessWorkerControlMessage {
    HostCall {
        call_id: String,
        request: ExtensionHostCapabilityRequest,
    },
    HostResult {
        call_id: String,
        response: ExtensionRpcResponse,
    },
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
