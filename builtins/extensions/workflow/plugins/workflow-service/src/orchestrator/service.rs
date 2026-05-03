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

        let task_titles = derive_plan_steps(&request);
        let tasks: Vec<TaskSpec> = task_titles
            .into_iter()
            .enumerate()
            .map(|(index, title)| {
                let agent_id = assigned_agents
                    .get(index % assigned_agents.len())
                    .cloned()
                    .unwrap_or_else(|| "system".to_string());
                TaskSpec {
                    id: format!("task-{run_id}-{}", index + 1),
                    run_id: run_id.clone(),
                    conversation_id: request.conversation_id.clone(),
                    lane_id: request.lane_id.clone(),
                    task_kind,
                    title,
                    assigned_agent_id: agent_id,
                    status: TaskStatus::Pending,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                }
            })
            .collect();

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
            tasks,
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

fn derive_plan_steps(request: &RunRequest) -> Vec<String> {
    let goal = request.goal.trim();
    let subject = extract_goal_subject(goal);
    let target_path = extract_target_path(goal);
    if looks_like_build_task(goal) {
        let mut steps = vec![
            format!("梳理{}的功能范围、运行方式和交付约束", subject),
            format!("搭建{}的基础结构和入口文件", subject),
            format!("实现{}的核心逻辑与主要交互", subject),
            "补齐边界处理、提示反馈和必要资源".to_string(),
        ];
        if let Some(path) = target_path {
            steps.push(format!("把产物整理并写入 {}", path));
        }
        steps.push("完成一次自检，确认计划无误后进入实际执行".to_string());
        return steps;
    }

    let mut steps = vec![
        format!("梳理{}的目标、约束和交付形式", subject),
        format!("拆分{}的关键处理步骤", subject),
        "准备执行所需的输入、路径或上下文信息".to_string(),
    ];
    if let Some(path) = target_path {
        steps.push(format!("确认结果最终落到 {}", path));
    }
    steps.push("完成一次计划自检，确认后进入实际执行".to_string());
    steps
}

fn looks_like_build_task(goal: &str) -> bool {
    let normalized = goal.to_ascii_lowercase();
    [
        "写",
        "实现",
        "开发",
        "做一个",
        "做个",
        "创建",
        "搭建",
        "游戏",
        "页面",
        "网站",
        "脚本",
        "程序",
        "组件",
        "app",
        "api",
        "game",
        "build",
        "code",
    ]
    .iter()
    .any(|pattern| goal.contains(pattern) || normalized.contains(pattern))
}

fn extract_goal_subject(goal: &str) -> String {
    let mut subject = goal.replace(['"', '\''], "");
    for marker in [
        "放在",
        "保存到",
        "写到",
        "输出到",
        "放到",
        "存到",
        "并放在",
        "并写到",
    ] {
        if let Some((head, _)) = subject.split_once(marker) {
            subject = head
                .trim()
                .trim_end_matches(['，', ',', '。', ';', '；'])
                .to_string();
        }
    }
    for prefix in [
        "我想让你帮我",
        "我想请你帮我",
        "请你帮我",
        "帮我",
        "请你",
        "我想",
    ] {
        if let Some(rest) = subject.strip_prefix(prefix) {
            subject = rest.trim().to_string();
            break;
        }
    }
    for marker in [
        "写一个",
        "做一个",
        "做个",
        "实现一个",
        "开发一个",
        "创建一个",
    ] {
        if let Some((_, tail)) = subject.split_once(marker) {
            subject = tail.trim().to_string();
            break;
        }
    }
    let subject = subject
        .trim()
        .trim_matches(['，', ',', '。', ';', '；', '：', ':'])
        .trim();
    if subject.is_empty() {
        "当前任务".to_string()
    } else {
        subject.to_string()
    }
}

fn extract_target_path(goal: &str) -> Option<String> {
    let quoted = extract_quoted_segments(goal);
    quoted
        .into_iter()
        .find(|segment| segment.contains('\\') || segment.contains('/'))
}

fn extract_quoted_segments(value: &str) -> Vec<String> {
    let mut segments = Vec::new();
    let mut start = None;
    for (index, ch) in value.char_indices() {
        if ch == '"' {
            if let Some(open) = start.take() {
                let segment = value[(open + 1)..index].trim();
                if !segment.is_empty() {
                    segments.push(segment.to_string());
                }
            } else {
                start = Some(index);
            }
        }
    }
    segments
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
    async fn plan_run_advances_to_dispatched_when_agent_is_ready() {
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

        assert_eq!(plan.run.stage, RunStage::Dispatched);
        assert_eq!(plan.stage_events.len(), 2);
        assert_eq!(plan.stage_events[0].to_stage, RunStage::Planning);
        assert_eq!(plan.stage_events[1].to_stage, RunStage::Dispatched);
        assert_eq!(plan.decision.reason, "plan-ready-agent-available");
        assert_eq!(plan.decision_snapshots.len(), 2);
        assert!(plan.tasks.len() >= 4);
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

    #[test]
    fn derive_plan_steps_generates_readable_build_plan() {
        let request = RunRequest {
            owner: OwnerRef::global("runtime"),
            conversation_id: "conversation-1".to_string(),
            lane_id: None,
            source_message_id: Some("message-1".to_string()),
            trigger: ennoia_contract::behavior::BehaviorTrigger::Message,
            goal:
                "我想让你帮我写一个贪吃蛇的游戏，写了放在\"C:\\Users\\Administrator\\Desktop\\ttt\""
                    .to_string(),
            requested_model_id: None,
            requested_max_turns: None,
            participants: vec!["agent-a".to_string()],
            addressed_agents: vec!["agent-a".to_string()],
        };
        let steps = derive_plan_steps(&request);
        assert!(steps.len() >= 5);
        assert!(steps.iter().any(|step| step.contains("贪吃蛇")));
        assert!(steps
            .iter()
            .any(|step| step.contains("C:\\Users\\Administrator\\Desktop\\ttt")));
    }
}
