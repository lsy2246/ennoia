use ennoia_kernel::{
    OwnerKind, OwnerRef, RunSpec, RunStatus, TaskSpec, TaskStatus, ThreadKind, ThreadSpec,
};
use ennoia_memory::ContextView;

use crate::model::{PlannedRun, RunTrigger};

/// OrchestratorService normalizes private and space messages into planned runs.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorService;

impl OrchestratorService {
    pub fn new() -> Self {
        Self
    }

    pub fn plan_private_run(&self, agent_id: &str, goal: &str, context: ContextView) -> PlannedRun {
        let owner = OwnerRef {
            kind: OwnerKind::Agent,
            id: agent_id.to_string(),
        };
        let thread = ThreadSpec {
            id: format!("thread-private-{agent_id}"),
            kind: ThreadKind::Private,
            owner: owner.clone(),
            participants: vec!["user".to_string(), agent_id.to_string()],
        };
        self.build_run(
            owner,
            thread,
            RunTrigger::DirectMessage,
            goal,
            vec![agent_id.to_string()],
            context,
        )
    }

    pub fn plan_space_run(
        &self,
        space_id: &str,
        addressed_agents: &[String],
        goal: &str,
        context: ContextView,
    ) -> PlannedRun {
        let owner = OwnerRef {
            kind: OwnerKind::Space,
            id: space_id.to_string(),
        };
        let mut participants = vec!["user".to_string()];
        participants.extend_from_slice(addressed_agents);
        let thread = ThreadSpec {
            id: format!("thread-space-{space_id}"),
            kind: ThreadKind::Space,
            owner: owner.clone(),
            participants,
        };
        self.build_run(
            owner,
            thread,
            RunTrigger::SpaceMessage,
            goal,
            addressed_agents.to_vec(),
            context,
        )
    }

    fn build_run(
        &self,
        owner: OwnerRef,
        thread: ThreadSpec,
        trigger: RunTrigger,
        goal: &str,
        addressed_agents: Vec<String>,
        context: ContextView,
    ) -> PlannedRun {
        let run_id = format!("run-{}-{}", thread.id, addressed_agents.len().max(1));
        let first_agent = addressed_agents
            .first()
            .cloned()
            .unwrap_or_else(|| "system".to_string());
        let run = RunSpec {
            id: run_id.clone(),
            owner,
            thread_id: thread.id,
            trigger: trigger_label(&trigger).to_string(),
            status: RunStatus::Pending,
        };
        let tasks = vec![TaskSpec {
            id: format!("task-{run_id}-1"),
            run_id,
            title: goal.to_string(),
            assigned_agent_id: first_agent,
            status: TaskStatus::Pending,
        }];

        PlannedRun {
            run,
            tasks,
            context,
        }
    }
}

fn trigger_label(trigger: &RunTrigger) -> &'static str {
    match trigger {
        RunTrigger::DirectMessage => "direct_message",
        RunTrigger::SpaceMessage => "space_message",
        RunTrigger::ScheduledJob => "scheduled_job",
    }
}
