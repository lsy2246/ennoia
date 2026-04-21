//! Kernel defines Ennoia's shared cross-module domain model and shared configuration.

pub mod config;
pub mod decision;
pub mod domain;
pub mod extension;
pub mod gate;
pub mod overview;
pub mod policy;
pub mod signals;
pub mod stage;
pub mod system_config;
pub mod ui;

// ========== Re-exports ==========

pub use config::{
    AgentConfig, AppConfig, ExtensionRegistryEntry, ExtensionRegistryFile, ProviderConfig,
    ProviderModelDiscoveryConfig, ServerConfig, SkillConfig, SkillRegistryEntry, SkillRegistryFile,
    UiConfig,
};
pub use decision::{Decision, DecisionSnapshot, NextAction};
pub use domain::{
    AgentSpec, ArtifactKind, ArtifactSpec, ConversationSpec, ConversationTopology, HandoffSpec,
    LaneSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, ParticipantRef, ParticipantType,
    RunSpec, RuntimeProfile, SpaceSpec, TaskKind, TaskSpec, TaskStatus,
};
pub use extension::{
    CommandContribution, ContributionSet, ExtensionAssetsSpec, ExtensionBackendSpec,
    ExtensionCapabilities, ExtensionDiagnostic, ExtensionFrontendSpec, ExtensionHealth,
    ExtensionKind, ExtensionManifest, ExtensionRuntimeEvent, ExtensionSourceMode,
    ExtensionSourceSpec, ExtensionWatchSpec, HookContribution, LocaleContribution,
    PageContribution, PanelContribution, ProviderContribution, ResolvedBackendEntry,
    ResolvedFrontendEntry, ThemeContribution,
};
pub use gate::{GateRecord, GateSeverity, GateVerdict};
pub use overview::{core_modules, PlatformOverview};
pub use policy::{
    GlobPattern, MemoryPolicy, RuntimeRule, RuntimeRuleCondition, RuntimeRuleTarget, StagePolicy,
};
pub use signals::{EvidenceSignals, ExecutionSignals, IntentSignals, Signals};
pub use stage::{RunStage, RunStageEvent, StageTransition};
pub use system_config::{
    default_local_dev_origins, BodyLimitConfig, BootstrapState, ConfigChangeRecord, ConfigEntry,
    ConfigError, ConfigStore, CorsConfig, LoggingConfig, RateLimitConfig, SystemConfig,
    TimeoutConfig, ALL_CONFIG_KEYS, CONFIG_KEY_BODY_LIMIT, CONFIG_KEY_BOOTSTRAP, CONFIG_KEY_CORS,
    CONFIG_KEY_LOGGING, CONFIG_KEY_RATE_LIMIT, CONFIG_KEY_TIMEOUT,
};
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
