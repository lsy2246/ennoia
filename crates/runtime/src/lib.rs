//! Runtime drives stage transitions, decision snapshots, and gate checks.

pub mod engine;
pub mod error;
pub mod gate;
pub mod gates;
pub mod sqlite_store;
pub mod stage;
pub mod store;

pub use engine::{DecisionEngine, DefaultDecisionEngine};
pub use error::RuntimeError;
pub use gate::{Gate, GateContext, GatePipeline};
pub use gates::{builtin_pipeline, AgentAvailableGate, ContextReadyGate, PlanReadyGate};
pub use sqlite_store::SqliteRuntimeStore;
pub use stage::{apply_next_action, PolicyStageMachine, StageMachine};
pub use store::RuntimeStore;

/// Returns the current runtime module name.
pub fn module_name() -> &'static str {
    "runtime"
}
