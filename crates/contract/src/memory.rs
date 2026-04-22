use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MemoryStatusResponse {
    pub memory_id: String,
    pub source_kind: String,
    pub healthy: bool,
    pub enabled: bool,
    #[serde(default)]
    pub interfaces: Vec<String>,
}
