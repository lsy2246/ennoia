use ennoia_kernel::{EpisodeKind, MemoryKind, MemoryRecord, MemorySource, OwnerRef, Stability};
use serde::{Deserialize, Serialize};

/// EpisodeRequest captures the input shape for recording an episode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeRequest {
    pub owner: OwnerRef,
    pub namespace: String,
    pub thread_id: Option<String>,
    pub run_id: Option<String>,
    pub episode_kind: EpisodeKind,
    pub role: Option<String>,
    pub content: String,
    pub content_type: Option<String>,
    pub source_uri: Option<String>,
    pub entities: Vec<String>,
    pub tags: Vec<String>,
    pub importance: Option<f32>,
    pub occurred_at: Option<String>,
}

/// RememberRequest captures the input shape for persisting a memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RememberRequest {
    pub owner: OwnerRef,
    pub namespace: String,
    pub memory_kind: MemoryKind,
    pub stability: Stability,
    pub title: Option<String>,
    pub content: String,
    pub summary: Option<String>,
    pub confidence: Option<f32>,
    pub importance: Option<f32>,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub sources: Vec<MemorySource>,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
}

/// RecallMode selects which backend to query during recall.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RecallMode {
    Namespace,
    Fts,
    Hybrid,
}

impl RecallMode {
    pub fn as_str(self) -> &'static str {
        match self {
            RecallMode::Namespace => "namespace",
            RecallMode::Fts => "fts",
            RecallMode::Hybrid => "hybrid",
        }
    }
}

/// RecallQuery describes how to recall memories for an owner.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecallQuery {
    pub owner: OwnerRef,
    pub thread_id: Option<String>,
    pub run_id: Option<String>,
    pub query_text: Option<String>,
    pub namespace_prefix: Option<String>,
    pub memory_kind: Option<MemoryKind>,
    pub mode: RecallMode,
    pub limit: u32,
}

impl RecallQuery {
    pub fn by_owner(owner: OwnerRef) -> Self {
        Self {
            owner,
            thread_id: None,
            run_id: None,
            query_text: None,
            namespace_prefix: None,
            memory_kind: None,
            mode: RecallMode::Namespace,
            limit: 20,
        }
    }
}

/// RecallResult is the structured output of one recall call.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecallResult {
    pub memories: Vec<MemoryRecord>,
    pub receipt_id: String,
    pub mode: String,
    pub total_chars: u32,
}

/// AssembleRequest describes the owner and thread for which to build a context view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssembleRequest {
    pub owner: OwnerRef,
    pub thread_id: Option<String>,
    pub run_id: Option<String>,
    pub recent_messages: Vec<String>,
    pub active_tasks: Vec<String>,
    pub budget_chars: Option<u32>,
}
