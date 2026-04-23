//! Kernel defines Ennoia's shared cross-module domain model and shared configuration.

pub mod config;
pub mod context;
pub mod decision;
pub mod domain;
pub mod extension;
pub mod gate;
pub mod overview;
pub mod policy;
pub mod server_settings;
pub mod signals;
pub mod stage;
pub mod ui;

// ========== Re-exports ==========

pub use config::{
    AgentConfig, AppConfig, ExtensionRegistryEntry, ExtensionRegistryFile, InterfaceBindingConfig,
    InterfaceBindingsConfig, ProviderConfig, ProviderModelDiscoveryConfig, ServerConfig,
    SkillConfig, SkillRegistryEntry, SkillRegistryFile, UiConfig,
};
pub use context::{ContextFrame, ContextLayer, RunContext};
pub use decision::{Decision, DecisionSnapshot, NextAction};
pub use domain::{
    AgentSpec, ArtifactKind, ArtifactSpec, ConversationSpec, ConversationTopology, HandoffSpec,
    LaneSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, ParticipantRef, ParticipantType,
    RunSpec, RuntimeProfile, SpaceSpec, TaskKind, TaskSpec, TaskStatus,
};
pub use extension::{
    BehaviorContribution, CommandContribution, ContributionSet, ExtensionAssetsSpec,
    ExtensionCapabilities, ExtensionDiagnostic, ExtensionHealth, ExtensionKind, ExtensionManifest,
    ExtensionPermissionSpec, ExtensionRpcError, ExtensionRpcRequest, ExtensionRpcResponse,
    ExtensionRuntimeEvent, ExtensionRuntimeSpec, ExtensionSourceMode, ExtensionSourceSpec,
    ExtensionUiSpec, ExtensionWatchSpec, ExtensionWorkerSpec, HookContribution,
    HookDispatchResponse, HookEventEnvelope, HookResourceRef, InterfaceContribution,
    LocaleContribution, MemoryContribution, PageContribution, PanelContribution,
    ProviderContribution, ProviderGenerationOption, ResolvedUiEntry, ResolvedWorkerEntry,
    ScheduleActionContribution, ThemeContribution, HOOK_EVENT_ARTIFACT_CREATED,
    HOOK_EVENT_CONVERSATION_CREATED, HOOK_EVENT_CONVERSATION_MESSAGE_CREATED, HOOK_EVENT_JOB_DUE,
    HOOK_EVENT_RUN_REQUESTED, HOOK_EVENT_RUN_STAGE_CHANGED,
};
pub use gate::{GateRecord, GateSeverity, GateVerdict};
pub use overview::{core_modules, PlatformOverview};
pub use policy::{
    GlobPattern, MemoryPolicy, RuntimeRule, RuntimeRuleCondition, RuntimeRuleTarget, StagePolicy,
};
pub use server_settings::{
    default_local_dev_origins, BodyLimitConfig, BootstrapState, CorsConfig, LoggingConfig,
    RateLimitConfig, TimeoutConfig,
};
pub use signals::{EvidenceSignals, ExecutionSignals, IntentSignals, Signals};
pub use stage::{RunStage, RunStageEvent, StageTransition};
pub use ui::{LocalizedText, ThemeAppearance, UiPreference, UiPreferenceRecord};

/// Returns the current kernel module name.
pub fn module_name() -> &'static str {
    "kernel"
}

/// Returns the Ennoia platform name.
pub fn platform_name() -> &'static str {
    "Ennoia"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_overview_lists_core_modules() {
        let overview = PlatformOverview::default();
        assert!(overview.modules.contains(&"kernel".to_string()));
        assert_eq!(overview.app_name, "Ennoia");
    }

    #[test]
    fn run_stage_round_trip() {
        for value in [
            "pending",
            "planning",
            "dispatched",
            "running",
            "blocked",
            "reviewing",
            "completed",
            "failed",
            "cancelled",
        ] {
            let stage = RunStage::from_str(value);
            assert_eq!(stage.as_str(), value);
        }
    }
}
