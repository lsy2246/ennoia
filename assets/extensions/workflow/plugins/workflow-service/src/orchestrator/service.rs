use std::sync::Arc;

use chrono::Utc;
use ennoia_kernel::{
    DecisionSnapshot, EvidenceSignals, ExecutionSignals, GateRecord, GateSeverity, GateVerdict,
    IntentSignals, RunContext, RunSpec, RunStage, RunStageEvent, Signals,
};
use uuid::Uuid;

use crate::orchestrator::model::{PlannedRun, RunRequest};
use crate::runtime::{GateContext, GatePipeline, StageMachine};

/// OrchestratorService is the thin coordinator. It assembles signals, calls the runtime,
/// and emits a PlannedRun snapshot that upstream code persists.
#[derive(Clone)]
pub struct OrchestratorService {
    stage_machine: Arc<dyn StageMachine>,
    gate_pipeline: GatePipeline,
}

impl OrchestratorService {
    pub fn new(stage_machine: Arc<dyn StageMachine>, gate_pipeline: GatePipeline) -> Self {
        Self {
            stage_machine,
            gate_pipeline,
        }
    }

    /// plan_run drives one run from a RunRequest + prepared RunContext + available agents.
    pub async fn plan_run(
        &self,
        request: RunRequest,
        context: RunContext,
        available_agents: Vec<String>,
    ) -> PlannedRun {
        let now = now_iso();
        let run_id = format!("run-{}", Uuid::new_v4());

        let assigned_agents = if request.addressed_agents.is_empty() {
            vec!["system".to_string()]
        } else {
            request.addressed_agents.clone()
        };

        let signals = build_signals(&request, &context, &assigned_agents, &available_agents);

        let (stage, decision, stage_events, decision_snapshots) =
            build_initial_plan_trace(&*self.stage_machine, &signals, &run_id);

        let run = RunSpec {
            id: run_id.clone(),
            owner: request.owner.clone(),
            conversation_id: request.conversation_id.clone(),
            lane_id: request.lane_id.clone(),
            source_message_id: request.source_message_id.clone(),
            trigger: request.trigger.as_str().to_string(),
            stage,
            goal: request.goal.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let gate_ctx = GateContext {
            run: run.clone(),
            signals: signals.clone(),
            context_view: context.clone(),
            assigned_agents: assigned_agents.clone(),
            available_agents,
        };
        let gate_verdicts = self.gate_pipeline.run(&gate_ctx).await;
        let gate_records = gate_verdicts
            .iter()
            .map(|verdict| to_gate_record(&run.id, verdict, &now))
            .collect();

        let signals_json = serde_json::to_string(&signals).unwrap_or_else(|_| "{}".to_string());
        let decision_snapshots = decision_snapshots
            .into_iter()
            .map(|snapshot| DecisionSnapshot {
                run_id: Some(run.id.clone()),
                signals_json: signals_json.clone(),
                ..snapshot
            })
            .collect();
        let stage_events = stage_events
            .into_iter()
            .map(|event| RunStageEvent {
                run_id: run.id.clone(),
                ..event
            })
            .collect();

        PlannedRun {
            run,
            plan: None,
            tasks: Vec::new(),
            context,
            signals,
            decision,
            stage_events,
            gate_verdicts,
            gate_records,
            decision_snapshots,
        }
    }
}

fn build_initial_plan_trace(
    stage_machine: &dyn StageMachine,
    signals: &Signals,
    run_id: &str,
) -> (
    RunStage,
    ennoia_kernel::Decision,
    Vec<RunStageEvent>,
    Vec<DecisionSnapshot>,
) {
    let mut stage = RunStage::Pending;
    let mut stage_events = Vec::new();
    let mut decision_snapshots = Vec::new();
    let final_decision = loop {
        let now = now_iso();
        let (decision, transition) = stage_machine.decide(stage, signals);
        decision_snapshots.push(DecisionSnapshot {
            id: format!("dec-{}", Uuid::new_v4()),
            run_id: Some(run_id.to_string()),
            task_id: None,
            stage: stage.as_str().to_string(),
            signals_json: String::new(),
            next_action: decision.next_action.as_str().to_string(),
            policy_rule_id: decision.policy_rule_id.clone(),
            at: now.clone(),
        });

        if transition.to == stage {
            break decision;
        }

        stage_events.push(RunStageEvent {
            id: format!("rse-{}", Uuid::new_v4()),
            run_id: run_id.to_string(),
            from_stage: Some(stage),
            to_stage: transition.to,
            policy_rule_id: Some(transition.policy_rule_id.clone()),
            reason: Some(transition.reason.clone()),
            at: now,
        });
        stage = transition.to;

        if stage != RunStage::Planning {
            break decision;
        }
    };

    (stage, final_decision, stage_events, decision_snapshots)
}

fn build_signals(
    request: &RunRequest,
    context: &RunContext,
    assigned_agents: &[String],
    available_agents: &[String],
) -> Signals {
    let intent = IntentSignals {
        trigger: request.trigger.as_str().to_string(),
        mention_count: request.addressed_agents.len() as u32,
        goal_len: request.goal.chars().count() as u32,
        has_question_mark: request.goal.contains('?') || request.goal.contains('？'),
    };
    let evidence = EvidenceSignals {
        recalled_memory_count: context.evidence_refs.len() as u32,
        source_count: 0,
        freshness_days: None,
        local_evidence_sufficient: !context.evidence_refs.is_empty()
            || !context.recent_messages.is_empty(),
    };
    let agent_available = !assigned_agents.is_empty()
        && assigned_agents
            .iter()
            .all(|a| available_agents.iter().any(|b| b == a) || a == "system");
    let execution = ExecutionSignals {
        plan_ready: false,
        agent_available,
        blocked: false,
        blocked_reason: None,
    };
    Signals {
        intent,
        evidence,
        execution,
    }
}

fn to_gate_record(run_id: &str, verdict: &GateVerdict, at: &str) -> GateRecord {
    let severity = match verdict.severity {
        GateSeverity::Info => "allow",
        GateSeverity::Warn => "warn",
        GateSeverity::Deny => "deny",
    };
    GateRecord {
        id: format!("gate-{}", Uuid::new_v4()),
        run_id: Some(run_id.to_string()),
        task_id: None,
        gate_name: verdict.gate_name.clone(),
        verdict: severity.to_string(),
        reason: Some(verdict.reason.clone()),
        details_json: "{}".to_string(),
        at: at.to_string(),
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{GatePipeline, PolicyStageMachine};
    use ennoia_kernel::{OwnerRef, StagePolicy};

    fn build_request(agent_id: &str) -> RunRequest {
        RunRequest {
            owner: OwnerRef::global("runtime"),
            conversation_id: "conversation-1".to_string(),
            lane_id: None,
            source_message_id: Some("message-1".to_string()),
            trigger: ennoia_contract::behavior::BehaviorTrigger::Message,
            goal: "test goal".to_string(),
            requested_model_id: None,
            requested_max_turns: None,
            participants: vec![agent_id.to_string()],
            addressed_agents: vec![agent_id.to_string()],
        }
    }

    #[tokio::test]
    async fn plan_run_starts_in_planning_even_when_agent_is_ready() {
        let orchestrator = OrchestratorService::new(
            Arc::new(PolicyStageMachine::new(Arc::new(StagePolicy::builtin()))),
            GatePipeline::new(Vec::new()),
        );

        let plan = orchestrator
            .plan_run(
                build_request("agent-a"),
                RunContext::default(),
                vec!["agent-a".to_string()],
            )
            .await;

        assert_eq!(plan.run.stage, RunStage::Planning);
        assert_eq!(plan.stage_events.len(), 1);
        assert_eq!(plan.stage_events[0].to_stage, RunStage::Planning);
        assert_eq!(plan.decision.reason, "no-rule-matched");
        assert_eq!(plan.decision_snapshots.len(), 2);
        assert!(plan.tasks.is_empty());
        assert!(plan.plan.is_none());
    }

    #[tokio::test]
    async fn plan_run_stays_in_planning_when_agent_is_not_ready() {
        let orchestrator = OrchestratorService::new(
            Arc::new(PolicyStageMachine::new(Arc::new(StagePolicy::builtin()))),
            GatePipeline::new(Vec::new()),
        );

        let plan = orchestrator
            .plan_run(
                build_request("missing-agent"),
                RunContext::default(),
                Vec::new(),
            )
            .await;

        assert_eq!(plan.run.stage, RunStage::Planning);
        assert_eq!(plan.stage_events.len(), 1);
        assert_eq!(plan.stage_events[0].to_stage, RunStage::Planning);
        assert_eq!(plan.decision.reason, "no-rule-matched");
        assert_eq!(plan.decision_snapshots.len(), 2);
    }
}
