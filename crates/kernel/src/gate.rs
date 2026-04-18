use serde::{Deserialize, Serialize};

/// GateSeverity describes how a gate verdict should be treated by the pipeline.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GateSeverity {
    Info,
    Warn,
    Deny,
}

/// GateVerdict is the outcome emitted by one gate.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateVerdict {
    pub gate_name: String,
    pub allow: bool,
    pub severity: GateSeverity,
    pub reason: String,
}

impl GateVerdict {
    pub fn allow(gate_name: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            allow: true,
            severity: GateSeverity::Info,
            reason: String::from("ok"),
        }
    }

    pub fn deny(gate_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            allow: false,
            severity: GateSeverity::Deny,
            reason: reason.into(),
        }
    }

    pub fn warn(gate_name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self {
            gate_name: gate_name.into(),
            allow: true,
            severity: GateSeverity::Warn,
            reason: reason.into(),
        }
    }
}

/// GateRecord is the persisted audit row for a gate verdict.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GateRecord {
    pub id: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub gate_name: String,
    pub verdict: String,
    pub reason: Option<String>,
    pub details_json: String,
    pub at: String,
}
