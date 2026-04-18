//! Orchestrator is the thin coordinator between runtime, memory and inbound triggers.

pub mod model;
pub mod service;

pub use model::{PlannedRun, RunRequest, RunTrigger};
pub use service::OrchestratorService;

/// Returns the current orchestrator module name.
pub fn module_name() -> &'static str {
    "orchestrator"
}
