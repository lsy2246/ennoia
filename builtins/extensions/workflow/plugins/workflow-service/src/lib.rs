//! Workflow extension backend.
//!
//! The workflow extension owns planning, stage decisions, gate checks and
//! artifact emission. Core only hosts and proxies the extension process.

pub mod orchestrator;
pub mod pipeline;
pub mod runtime;

pub use ennoia_contract::behavior::{
    BehaviorRunDetailResponse, BehaviorRunRequest, BehaviorRunResponse, BehaviorStatusResponse,
};
pub use pipeline::{run_behavior, WorkflowRuntime};
