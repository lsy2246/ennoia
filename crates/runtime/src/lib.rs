//! Runtime module owns execution contracts plus the built-in implementations.

pub mod engine;
pub mod gates;
pub mod model;
pub mod sqlite_store;
pub mod stage;

pub use engine::DefaultDecisionEngine;
pub use gates::{builtin_pipeline, AgentAvailableGate, ContextReadyGate, PlanReadyGate};
pub use model::{
    DecisionEngine, Gate, GateContext, GatePipeline, RuntimeError, RuntimeStore, StageMachine,
};
pub use sqlite_store::SqliteRuntimeStore;
pub use stage::{apply_next_action, PolicyStageMachine};

pub fn module_name() -> &'static str {
    "runtime"
}
