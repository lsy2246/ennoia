//! Workflow extension backend.
//!
//! The workflow extension owns planning, stage decisions, gate checks and
//! artifact emission. Core only hosts and proxies the extension process.

pub mod orchestrator;
pub mod pipeline;
pub mod runtime;

pub use ennoia_kernel::{ConversationMessageHookPayload, ConversationWorkflowOutput};
pub use pipeline::{run_conversation_workflow, WorkflowRuntime};
