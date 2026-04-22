//! Orchestrator is the thin coordinator between workflow runtime and inbound triggers.

pub mod model;
pub mod service;

pub use model::{PlannedRun, RunRequest};
pub use service::OrchestratorService;

/// Returns the current orchestrator module name.
pub fn module_name() -> &'static str {
    "orchestrator"
}
