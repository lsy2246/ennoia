//! Runtime implementations: stage machines, decision engines, gates, sqlite store.

pub mod engine;
pub mod gates;
pub mod sqlite_store;
pub mod stage;

pub use engine::DefaultDecisionEngine;
pub use gates::{builtin_pipeline, AgentAvailableGate, ContextReadyGate, PlanReadyGate};
pub use sqlite_store::SqliteRuntimeStore;
pub use stage::{apply_next_action, PolicyStageMachine};

pub fn module_name() -> &'static str {
    "runtime"
}
