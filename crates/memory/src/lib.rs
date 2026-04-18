//! Memory owns truth, working state, context views and review flows.

pub mod error;
pub mod model;
pub mod requests;
pub mod sqlite;
pub mod store;

pub use error::MemoryError;
pub use ennoia_kernel::{
    ContextFrame, ContextLayer, ContextView, EpisodeKind, EpisodeRecord, MemoryKind, MemoryRecord,
    MemorySource, MemoryStatus, ReviewAction, ReviewActionKind, Stability,
};
pub use model::{RecallReceipt, RememberReceipt, ReviewReceipt};
pub use requests::{
    AssembleRequest, EpisodeRequest, RecallMode, RecallQuery, RecallResult, RememberRequest,
};
pub use sqlite::SqliteMemoryStore;
pub use store::MemoryStore;

/// Returns the current memory module name.
pub fn module_name() -> &'static str {
    "memory"
}
