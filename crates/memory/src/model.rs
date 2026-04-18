use ennoia_kernel::OwnerRef;
use serde::{Deserialize, Serialize};

/// MemoryKind groups memory records by intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum MemoryKind {
    Truth,
    Working,
    Review,
    Projection,
}

/// MemoryRecord is the canonical stored memory shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryRecord {
    pub id: String,
    pub owner: OwnerRef,
    pub thread_id: Option<String>,
    pub run_id: Option<String>,
    pub kind: MemoryKind,
    pub source: String,
    pub content: String,
    pub summary: String,
    pub created_at: String,
}

/// ContextView is the assembled context emitted to the orchestrator.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextView {
    pub thread_facts: Vec<String>,
    pub recent_messages: Vec<String>,
    pub active_tasks: Vec<String>,
    pub recalled_memories: Vec<String>,
    pub workspace_summary: Vec<String>,
}

/// ReviewWorkbench keeps the review state attached to one owner.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewWorkbench {
    pub owner: Option<OwnerRef>,
    pub open_findings: Vec<String>,
    pub review_snapshots: Vec<String>,
}
