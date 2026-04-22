//! Memory domain model, requests, receipts, store contract and error types.

use ennoia_kernel::{ContextFrame, OwnerRef, RunContext};
use serde::{Deserialize, Serialize};

/// MemoryKind classifies stored memories by type of knowledge.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryKind {
    Fact,
    Preference,
    DecisionNote,
    Procedure,
    Context,
    Observation,
}

impl MemoryKind {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryKind::Fact => "fact",
            MemoryKind::Preference => "preference",
            MemoryKind::DecisionNote => "decision_note",
            MemoryKind::Procedure => "procedure",
            MemoryKind::Context => "context",
            MemoryKind::Observation => "observation",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "preference" => MemoryKind::Preference,
            "decision_note" => MemoryKind::DecisionNote,
            "procedure" => MemoryKind::Procedure,
            "context" => MemoryKind::Context,
            "observation" => MemoryKind::Observation,
            _ => MemoryKind::Fact,
        }
    }
}

/// Stability is orthogonal to kind and describes how durable the memory is.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Stability {
    Working,
    LongTerm,
}

impl Stability {
    pub fn as_str(self) -> &'static str {
        match self {
            Stability::Working => "working",
            Stability::LongTerm => "long_term",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "long_term" => Stability::LongTerm,
            _ => Stability::Working,
        }
    }
}

/// MemoryStatus is the lifecycle status of a memory record.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryStatus {
    Active,
    PendingReview,
    Superseded,
    Retired,
}

impl MemoryStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            MemoryStatus::Active => "active",
            MemoryStatus::PendingReview => "pending_review",
            MemoryStatus::Superseded => "superseded",
            MemoryStatus::Retired => "retired",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "pending_review" => MemoryStatus::PendingReview,
            "superseded" => MemoryStatus::Superseded,
            "retired" => MemoryStatus::Retired,
            _ => MemoryStatus::Active,
        }
    }
}

/// EpisodeKind classifies entries in the episode stream.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EpisodeKind {
    Message,
    ToolCall,
    Artifact,
    Job,
    Decision,
}

impl EpisodeKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EpisodeKind::Message => "message",
            EpisodeKind::ToolCall => "tool_call",
            EpisodeKind::Artifact => "artifact",
            EpisodeKind::Job => "job",
            EpisodeKind::Decision => "decision",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "tool_call" => EpisodeKind::ToolCall,
            "artifact" => EpisodeKind::Artifact,
            "job" => EpisodeKind::Job,
            "decision" => EpisodeKind::Decision,
            _ => EpisodeKind::Message,
        }
    }
}

/// MemorySource is one citation backing a memory record.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemorySource {
    pub kind: String,
    pub reference: String,
}

/// MemoryRecord is the canonical stored memory shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MemoryRecord {
    pub id: String,
    pub owner: OwnerRef,
    pub namespace: String,
    pub memory_kind: MemoryKind,
    pub stability: Stability,
    pub status: MemoryStatus,
    pub superseded_by: Option<String>,
    pub title: Option<String>,
    pub content: String,
    pub summary: Option<String>,
    pub confidence: f32,
    pub importance: f32,
    pub valid_from: Option<String>,
    pub valid_to: Option<String>,
    pub sources: Vec<MemorySource>,
    pub tags: Vec<String>,
    pub entities: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// EpisodeRecord is one append-only event in the episode stream.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeRecord {
    pub id: String,
    pub owner: OwnerRef,
    pub namespace: String,
    pub conversation_id: Option<String>,
    pub run_id: Option<String>,
    pub episode_kind: EpisodeKind,
    pub role: Option<String>,
    pub content: String,
    pub content_type: String,
    pub source_uri: Option<String>,
    pub entities: Vec<String>,
    pub tags: Vec<String>,
    pub importance: f32,
    pub occurred_at: String,
    pub ingested_at: String,
}

/// ReviewAction describes a human or agent review decision on a memory.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewAction {
    pub target_memory_id: String,
    pub reviewer: String,
    pub action: ReviewActionKind,
    pub notes: Option<String>,
}

/// ReviewActionKind lists the review verbs.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewActionKind {
    Approve,
    Reject,
    Supersede,
    Retire,
}

impl ReviewActionKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ReviewActionKind::Approve => "approve",
            ReviewActionKind::Reject => "reject",
            ReviewActionKind::Supersede => "supersede",
            ReviewActionKind::Retire => "retire",
        }
    }
}

// ========== Requests ==========

/// EpisodeRequest captures the input shape for recording an episode.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EpisodeRequest {
    pub owner: OwnerRef,
    pub namespace: String,
    pub conversation_id: Option<String>,
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
    pub conversation_id: Option<String>,
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
            conversation_id: None,
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

/// AssembleRequest describes the owner and conversation for which to build a context view.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AssembleRequest {
    pub owner: OwnerRef,
    pub conversation_id: Option<String>,
    pub run_id: Option<String>,
    pub recent_messages: Vec<String>,
    pub active_tasks: Vec<String>,
    pub budget_chars: Option<u32>,
}

// ========== Receipts ==========

/// RememberReceipt is returned after a memory is persisted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RememberReceipt {
    pub receipt_id: String,
    pub memory_id: String,
    pub action: String,
    pub policy_rule_id: Option<String>,
    pub created_at: String,
}

/// RecallReceipt captures one recall event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RecallReceipt {
    pub receipt_id: String,
    pub owner_kind: String,
    pub owner_id: String,
    pub conversation_id: Option<String>,
    pub run_id: Option<String>,
    pub query_text: Option<String>,
    pub mode: String,
    pub memory_ids: Vec<String>,
    pub chars: u32,
    pub created_at: String,
}

/// ReviewReceipt is produced by any review action.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReviewReceipt {
    pub receipt_id: String,
    pub target_memory_id: String,
    pub action: String,
    pub old_status: Option<String>,
    pub new_status: String,
    pub reviewer: String,
    pub created_at: String,
}

// ========== Error ==========

/// MemoryError unifies failures emitted by MemoryStore implementations.
#[derive(Debug)]
pub enum MemoryError {
    Backend(String),
    Serde(String),
    Policy(String),
    NotFound(String),
    Invalid(String),
}

impl std::fmt::Display for MemoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MemoryError::Backend(reason) => write!(f, "memory backend error: {reason}"),
            MemoryError::Serde(reason) => write!(f, "memory serde error: {reason}"),
            MemoryError::Policy(reason) => write!(f, "memory policy violation: {reason}"),
            MemoryError::NotFound(key) => write!(f, "memory record not found: {key}"),
            MemoryError::Invalid(reason) => write!(f, "memory invalid input: {reason}"),
        }
    }
}

impl std::error::Error for MemoryError {}

// ========== MemoryStore trait ==========

/// MemoryStore is the persistence contract for the memory system.
#[async_trait::async_trait]
pub trait MemoryStore: Send + Sync {
    async fn record_episode(&self, req: EpisodeRequest) -> Result<EpisodeRecord, MemoryError>;
    async fn remember(&self, req: RememberRequest) -> Result<RememberReceipt, MemoryError>;
    async fn recall(&self, query: RecallQuery) -> Result<RecallResult, MemoryError>;
    async fn review(&self, action: ReviewAction) -> Result<ReviewReceipt, MemoryError>;
    async fn assemble_context(&self, req: AssembleRequest) -> Result<RunContext, MemoryError>;
    async fn upsert_frame(&self, frame: ContextFrame) -> Result<(), MemoryError>;
    async fn list_memories(&self, limit: u32) -> Result<Vec<MemoryRecord>, MemoryError>;
    async fn list_episodes_for_owner(
        &self,
        owner: &OwnerRef,
        limit: u32,
    ) -> Result<Vec<EpisodeRecord>, MemoryError>;
}
