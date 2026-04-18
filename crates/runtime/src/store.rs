use async_trait::async_trait;
use ennoia_kernel::{DecisionSnapshot, GateRecord, RunStageEvent};

use crate::error::RuntimeError;

/// RuntimeStore persists decision snapshots, stage transitions, and gate verdicts.
#[async_trait]
pub trait RuntimeStore: Send + Sync {
    async fn log_stage_event(&self, event: &RunStageEvent) -> Result<(), RuntimeError>;
    async fn log_decision(&self, snapshot: &DecisionSnapshot) -> Result<(), RuntimeError>;
    async fn log_gate_verdict(&self, record: &GateRecord) -> Result<(), RuntimeError>;
    async fn list_stage_events_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<RunStageEvent>, RuntimeError>;
    async fn list_decisions_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<DecisionSnapshot>, RuntimeError>;
    async fn list_gate_verdicts_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<GateRecord>, RuntimeError>;
}
