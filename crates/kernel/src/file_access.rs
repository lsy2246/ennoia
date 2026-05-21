use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentFileAccessProfile {
    #[serde(default = "default_file_access_default_root")]
    pub default_root: String,
    #[serde(default = "default_file_access_roots")]
    pub roots: Vec<AgentFileAccessRoot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentFileAccessRoot {
    pub id: String,
    pub path: String,
    #[serde(default = "default_file_access_mode")]
    pub mode: String,
}

impl Default for AgentFileAccessProfile {
    fn default() -> Self {
        Self {
            default_root: default_file_access_default_root(),
            roots: default_file_access_roots(),
        }
    }
}

fn default_file_access_default_root() -> String {
    "/workspace".to_string()
}

fn default_file_access_mode() -> String {
    "read_write".to_string()
}

fn default_file_access_roots() -> Vec<AgentFileAccessRoot> {
    vec![
        AgentFileAccessRoot {
            id: "workspace".to_string(),
            path: "/workspace".to_string(),
            mode: default_file_access_mode(),
        },
        AgentFileAccessRoot {
            id: "artifacts".to_string(),
            path: "/artifacts".to_string(),
            mode: default_file_access_mode(),
        },
        AgentFileAccessRoot {
            id: "temp".to_string(),
            path: "/tmp".to_string(),
            mode: default_file_access_mode(),
        },
    ]
}
