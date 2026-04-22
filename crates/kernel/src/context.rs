use serde::{Deserialize, Serialize};

use crate::OwnerRef;

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

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunContext {
    pub core: Vec<String>,
    pub execution: Vec<String>,
    pub preferences: Vec<String>,
    pub constraints: Vec<String>,
    pub evidence: Vec<String>,
    pub recent_messages: Vec<String>,
    pub active_tasks: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub total_chars: u32,
}

impl RunContext {
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
