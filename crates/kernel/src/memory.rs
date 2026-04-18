use serde::{Deserialize, Serialize};

use crate::OwnerRef;

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
    pub thread_id: Option<String>,
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

/// ContextLayer labels the semantic layer of an assembled context fragment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextLayer {
    Core,
    Execution,
    Preferences,
    Constraints,
    Evidence,
}

impl ContextLayer {
    pub fn as_str(self) -> &'static str {
        match self {
            ContextLayer::Core => "core",
            ContextLayer::Execution => "execution",
            ContextLayer::Preferences => "preferences",
            ContextLayer::Constraints => "constraints",
            ContextLayer::Evidence => "evidence",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "execution" => ContextLayer::Execution,
            "preferences" => ContextLayer::Preferences,
            "constraints" => ContextLayer::Constraints,
            "evidence" => ContextLayer::Evidence,
            _ => ContextLayer::Core,
        }
    }
}

/// ContextFrame is a reusable chunk of assembled text tied to a layer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextFrame {
    pub id: String,
    pub owner: OwnerRef,
    pub namespace: String,
    pub layer: ContextLayer,
    pub frame_kind: String,
    pub content: String,
    pub source_memory_ids: Vec<String>,
    pub budget_chars: Option<u32>,
    pub ttl_seconds: Option<u32>,
    pub created_at: String,
    pub updated_at: String,
}

/// ContextView is the assembled context emitted to the orchestrator for a run.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextView {
    pub core: Vec<String>,
    pub execution: Vec<String>,
    pub preferences: Vec<String>,
    pub constraints: Vec<String>,
    pub evidence: Vec<String>,
    pub recent_messages: Vec<String>,
    pub active_tasks: Vec<String>,
    pub recalled_memory_ids: Vec<String>,
    pub total_chars: u32,
}

impl ContextView {
    pub fn push(&mut self, layer: ContextLayer, content: String) {
        self.total_chars += content.chars().count() as u32;
        match layer {
            ContextLayer::Core => self.core.push(content),
            ContextLayer::Execution => self.execution.push(content),
            ContextLayer::Preferences => self.preferences.push(content),
            ContextLayer::Constraints => self.constraints.push(content),
            ContextLayer::Evidence => self.evidence.push(content),
        }
    }
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
