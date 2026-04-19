//! Kernel defines Ennoia's shared domain model, traits and policy contracts.
//!
//! All cross-crate contracts live here. Implementation crates (memory / runtime /
//! scheduler / auth / policy-loader) depend only on `ennoia-kernel`.

pub mod auth;
pub mod config;
pub mod decision;
pub mod domain;
pub mod extension;
pub mod gate;
pub mod memory;
pub mod overview;
pub mod policy;
pub mod runtime;
pub mod scheduler;
pub mod signals;
pub mod stage;
pub mod system_config;
pub mod ui;

// ========== Re-exports ==========

pub use auth::{
    ApiKey, ApiKeyStore, AuthError, CreateApiKeyRequest, CreateSessionRequest, CreateUserRequest,
    Session, SessionStore, UpdateUserRequest, User, UserRole, UserStore,
};
pub use config::{AgentConfig, AppConfig, ServerConfig, UiConfig};
pub use decision::{Decision, DecisionSnapshot, NextAction};
pub use domain::{
    AgentSpec, ArtifactKind, ArtifactSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, RunSpec,
    SpaceSpec, TaskKind, TaskSpec, TaskStatus, ThreadKind, ThreadSpec,
};
pub use extension::{
    CommandContribution, ContributionSet, ExtensionKind, ExtensionManifest, HookContribution,
    LocaleContribution, PageContribution, PanelContribution, ProviderContribution,
    ThemeContribution,
};
pub use gate::{GateRecord, GateSeverity, GateVerdict};
pub use memory::{
    AssembleRequest, ContextFrame, ContextLayer, ContextView, EpisodeKind, EpisodeRecord,
    EpisodeRequest, MemoryError, MemoryKind, MemoryRecord, MemorySource, MemoryStatus, MemoryStore,
    RecallMode, RecallQuery, RecallReceipt, RecallResult, RememberReceipt, RememberRequest,
    ReviewAction, ReviewActionKind, ReviewReceipt, Stability,
};
pub use overview::{core_modules, PlatformOverview};
pub use policy::{
    GlobPattern, MemoryPolicy, RuntimeRule, RuntimeRuleCondition, RuntimeRuleTarget, StagePolicy,
};
pub use runtime::{
    DecisionEngine, Gate, GateContext, GatePipeline, RuntimeError, RuntimeStore, StageMachine,
};
pub use scheduler::{
    EnqueueRequest, JobHandler, JobKind, JobRecord, JobStatus, ScheduleKind, SchedulerError,
    SchedulerStore,
};
pub use signals::{EvidenceSignals, ExecutionSignals, IntentSignals, Signals};
pub use stage::{RunStage, RunStageEvent, StageTransition};
pub use system_config::{
    AuthConfig, AuthMode, BodyLimitConfig, BootstrapState, ConfigChangeRecord, ConfigEntry,
    ConfigError, ConfigStore, CorsConfig, LoggingConfig, RateLimitConfig, SystemConfig,
    TimeoutConfig, ALL_CONFIG_KEYS, CONFIG_KEY_AUTH, CONFIG_KEY_BODY_LIMIT, CONFIG_KEY_BOOTSTRAP,
    CONFIG_KEY_CORS, CONFIG_KEY_LOGGING, CONFIG_KEY_RATE_LIMIT, CONFIG_KEY_TIMEOUT,
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
