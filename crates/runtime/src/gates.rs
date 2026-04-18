use std::sync::Arc;

use async_trait::async_trait;
use ennoia_kernel::{Gate, GateContext, GatePipeline, GateVerdict};

/// ContextReadyGate fails when the assembled context view is empty.
#[derive(Debug, Default, Clone, Copy)]
pub struct ContextReadyGate;

#[async_trait]
impl Gate for ContextReadyGate {
    fn name(&self) -> &'static str {
        "context_ready"
    }

    async fn check(&self, ctx: &GateContext) -> GateVerdict {
        if ctx.context_view.total_chars == 0
            && ctx.context_view.recent_messages.is_empty()
            && ctx.context_view.recalled_memory_ids.is_empty()
        {
            GateVerdict::warn(self.name(), "context view is empty")
        } else {
            GateVerdict::allow(self.name())
        }
    }
}

/// AgentAvailableGate denies when any assigned agent is not declared available.
#[derive(Debug, Default, Clone, Copy)]
pub struct AgentAvailableGate;

#[async_trait]
impl Gate for AgentAvailableGate {
    fn name(&self) -> &'static str {
        "agent_available"
    }

    async fn check(&self, ctx: &GateContext) -> GateVerdict {
        if ctx.assigned_agents.is_empty() {
            return GateVerdict::warn(self.name(), "no agent assigned");
        }
        for agent_id in &ctx.assigned_agents {
            if !ctx.available_agents.iter().any(|a| a == agent_id) {
                return GateVerdict::deny(
                    self.name(),
                    format!("agent '{agent_id}' is not available"),
                );
            }
        }
        GateVerdict::allow(self.name())
    }
}

/// PlanReadyGate forwards the execution.plan_ready signal as a verdict.
#[derive(Debug, Default, Clone, Copy)]
pub struct PlanReadyGate;

#[async_trait]
impl Gate for PlanReadyGate {
    fn name(&self) -> &'static str {
        "plan_ready"
    }

    async fn check(&self, ctx: &GateContext) -> GateVerdict {
        if ctx.signals.execution.plan_ready {
            GateVerdict::allow(self.name())
        } else {
            GateVerdict::warn(self.name(), "plan is not yet ready")
        }
    }
}

/// builtin_pipeline returns the default pipeline with the ships-with gates.
pub fn builtin_pipeline() -> GatePipeline {
    let gates: Vec<Arc<dyn Gate>> = vec![
        Arc::new(ContextReadyGate),
        Arc::new(AgentAvailableGate),
        Arc::new(PlanReadyGate),
    ];
    GatePipeline::new(gates)
}
