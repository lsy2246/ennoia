use std::sync::Arc;

use async_trait::async_trait;
use ennoia_kernel::{ContextView, GateVerdict, RunSpec, Signals};

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
