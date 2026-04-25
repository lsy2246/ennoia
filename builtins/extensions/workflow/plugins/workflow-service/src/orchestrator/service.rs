use std::sync::Arc;

use chrono::Utc;
use ennoia_kernel::{
    DecisionSnapshot, EvidenceSignals, ExecutionSignals, GateRecord, GateSeverity, GateVerdict,
    IntentSignals, RunContext, RunSpec, RunStage, RunStageEvent, Signals, TaskKind, TaskSpec,
    TaskStatus,
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
        let task_kind = if request.participants.len() > 1 {
            TaskKind::Collaboration
        } else {
            TaskKind::Response
        };

        let assigned_agents = if request.addressed_agents.is_empty() {
            vec!["system".to_string()]
        } else {
            request.addressed_agents.clone()
        };

        let signals = build_signals(&request, &context, &assigned_agents, &available_agents);

        let stage = RunStage::Pending;
        let (decision, transition) = self.stage_machine.decide(stage, &signals);

        let run = RunSpec {
            id: run_id.clone(),
            owner: request.owner.clone(),
            conversation_id: request.conversation_id.clone(),
            lane_id: request.lane_id.clone(),
            trigger: request.trigger.as_str().to_string(),
            stage: transition.to,
            goal: request.goal.clone(),
            created_at: now.clone(),
            updated_at: now.clone(),
        };

        let tasks: Vec<TaskSpec> = assigned_agents
            .iter()
            .enumerate()
            .map(|(index, agent_id)| TaskSpec {
                id: format!("task-{run_id}-{}", index + 1),
                run_id: run_id.clone(),
                conversation_id: request.conversation_id.clone(),
                lane_id: request.lane_id.clone(),
                task_kind,
                title: task_title(&request, agent_id),
                assigned_agent_id: agent_id.clone(),
                status: TaskStatus::Pending,
                created_at: now.clone(),
                updated_at: now.clone(),
            })
            .collect();

        let stage_event = RunStageEvent {
            id: format!("rse-{}", Uuid::new_v4()),
            run_id: run.id.clone(),
            from_stage: Some(stage),
            to_stage: transition.to,
            policy_rule_id: Some(transition.policy_rule_id.clone()),
            reason: Some(transition.reason.clone()),
            at: now.clone(),
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
        let decision_snapshot = DecisionSnapshot {
            id: format!("dec-{}", Uuid::new_v4()),
            run_id: Some(run.id.clone()),
            task_id: None,
            stage: stage.as_str().to_string(),
            signals_json,
            next_action: decision.next_action.as_str().to_string(),
            policy_rule_id: decision.policy_rule_id.clone(),
            at: now.clone(),
        };

        PlannedRun {
            run,
            tasks,
            context,
            signals,
            decision,
            stage_event,
            gate_verdicts,
            gate_records,
            decision_snapshot,
        }
    }
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
        plan_ready: agent_available,
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

fn task_title(request: &RunRequest, agent_id: &str) -> String {
    let mut title = format!("{} · {}", request.goal, agent_id);
    if let Some(model_id) = request
        .requested_model_id
        .as_deref()
        .filter(|item| !item.trim().is_empty())
    {
        title.push_str(&format!(" · model={model_id}"));
    }
    if let Some(max_turns) = request.requested_max_turns {
        title.push_str(&format!(" · max_turns={max_turns}"));
    }
    title
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
