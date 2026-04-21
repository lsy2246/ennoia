//! Runtime traits: stage machine, gate pipeline, store contract and error types.
//!
//! The runtime module owns its execution contracts and built-in implementations.

use std::sync::Arc;

use async_trait::async_trait;

use ennoia_kernel::{
    Decision, DecisionSnapshot, GateRecord, GateVerdict, RunSpec, RunStage, RunStageEvent, Signals,
    StageTransition,
};
use ennoia_memory::ContextView;

// ========== StageMachine ==========

/// StageMachine owns the rule that decides "given current stage and signals, what's next".
pub trait StageMachine: Send + Sync {
    fn decide(&self, stage: RunStage, signals: &Signals) -> (Decision, StageTransition);
}

// ========== DecisionEngine ==========

/// DecisionEngine produces a Decision for a given stage + signals.
pub trait DecisionEngine: Send + Sync {
    fn decide(&self, stage: RunStage, signals: &Signals) -> Decision;
}

// ========== Gate + GatePipeline ==========

/// GateContext is the snapshot handed to every gate in the pipeline.
#[derive(Debug, Clone)]
pub struct GateContext {
    pub run: RunSpec,
    pub signals: Signals,
    pub context_view: ContextView,
    pub assigned_agents: Vec<String>,
    pub available_agents: Vec<String>,
}

/// Gate is one check in the execution readiness pipeline.
#[async_trait]
pub trait Gate: Send + Sync {
    fn name(&self) -> &'static str;
    async fn check(&self, ctx: &GateContext) -> GateVerdict;
}

/// GatePipeline runs its gates in order and returns every verdict.
#[derive(Clone)]
pub struct GatePipeline {
    gates: Vec<Arc<dyn Gate>>,
}

impl GatePipeline {
    pub fn new(gates: Vec<Arc<dyn Gate>>) -> Self {
        Self { gates }
    }

    pub fn gates(&self) -> &[Arc<dyn Gate>] {
        &self.gates
    }

    pub async fn run(&self, ctx: &GateContext) -> Vec<GateVerdict> {
        let mut verdicts = Vec::with_capacity(self.gates.len());
        for gate in &self.gates {
            verdicts.push(gate.check(ctx).await);
        }
        verdicts
    }

    pub fn any_deny(verdicts: &[GateVerdict]) -> Option<&GateVerdict> {
        verdicts.iter().find(|v| !v.allow)
    }
}

// ========== RuntimeStore trait ==========

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

// ========== Error ==========

#[derive(Debug)]
pub enum RuntimeError {
    Backend(String),
    Serde(String),
    Invalid(String),
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RuntimeError::Backend(reason) => write!(f, "runtime backend error: {reason}"),
            RuntimeError::Serde(reason) => write!(f, "runtime serde error: {reason}"),
            RuntimeError::Invalid(reason) => write!(f, "runtime invalid input: {reason}"),
        }
    }
}

impl std::error::Error for RuntimeError {}
