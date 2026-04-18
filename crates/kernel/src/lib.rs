//! Kernel defines Ennoia's shared domain model and contracts.

pub mod config;
pub mod decision;
pub mod domain;
pub mod extension;
pub mod gate;
pub mod memory;
pub mod overview;
pub mod signals;
pub mod stage;

pub use config::{AgentConfig, AppConfig, ServerConfig, UiConfig};
pub use decision::{Decision, DecisionSnapshot, NextAction};
pub use domain::{
    AgentSpec, ArtifactKind, ArtifactSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, RunSpec,
    SpaceSpec, TaskKind, TaskSpec, TaskStatus, ThreadKind, ThreadSpec,
};
pub use extension::{
    CommandContribution, ContributionSet, ExtensionKind, ExtensionManifest, HookContribution,
    PageContribution, PanelContribution, ProviderContribution, ThemeContribution,
};
pub use gate::{GateRecord, GateSeverity, GateVerdict};
pub use memory::{
    ContextFrame, ContextLayer, ContextView, EpisodeKind, EpisodeRecord, MemoryKind, MemoryRecord,
    MemorySource, MemoryStatus, ReviewAction, ReviewActionKind, Stability,
};
pub use overview::{core_modules, PlatformOverview};
pub use signals::{EvidenceSignals, ExecutionSignals, IntentSignals, Signals};
pub use stage::{RunStage, RunStageEvent, StageTransition};

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
