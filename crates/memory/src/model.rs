use serde::{Deserialize, Serialize};

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
    pub thread_id: Option<String>,
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
