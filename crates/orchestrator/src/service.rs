use std::time::{SystemTime, UNIX_EPOCH};

use ennoia_kernel::{
    MessageRole, MessageSpec, OwnerKind, OwnerRef, RunSpec, RunStatus, TaskKind, TaskSpec,
    TaskStatus, ThreadKind, ThreadSpec,
};
use ennoia_memory::ContextView;

use crate::model::{PlannedRun, RunRequest, RunTrigger};

/// OrchestratorService normalizes private and space messages into tracked runs.
#[derive(Debug, Clone, Default)]
pub struct OrchestratorService;

impl OrchestratorService {
    pub fn new() -> Self {
        Self
    }

    pub fn plan_private_run(&self, agent_id: &str, goal: &str, context: ContextView) -> PlannedRun {
        let timestamp = current_timestamp();
        let owner = OwnerRef {
            kind: OwnerKind::Agent,
            id: agent_id.to_string(),
        };
        let thread = ThreadSpec {
            id: format!("thread-private-{agent_id}"),
            kind: ThreadKind::Private,
            owner: owner.clone(),
            space_id: None,
            title: format!("与 {agent_id} 的私聊"),
            participants: vec!["user".to_string(), agent_id.to_string()],
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let message = MessageSpec {
            id: format!("message-{}-seed", thread.id),
            thread_id: thread.id.clone(),
            sender: "user".to_string(),
            role: MessageRole::User,
            body: goal.to_string(),
            mentions: vec![agent_id.to_string()],
            created_at: timestamp,
        };

        self.plan_run(
            RunRequest {
                owner,
                thread,
                message,
                trigger: RunTrigger::DirectMessage,
                goal: goal.to_string(),
                addressed_agents: vec![agent_id.to_string()],
            },
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
        let timestamp = current_timestamp();
        let owner = OwnerRef {
            kind: OwnerKind::Space,
            id: space_id.to_string(),
        };
        let mut participants = vec!["user".to_string()];
        participants.extend(addressed_agents.iter().cloned());
        let thread = ThreadSpec {
            id: format!("thread-space-{space_id}"),
            kind: ThreadKind::Space,
            owner: owner.clone(),
            space_id: Some(space_id.to_string()),
            title: format!("{space_id} 协作线程"),
            participants,
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };
        let message = MessageSpec {
            id: format!("message-{}-seed", thread.id),
            thread_id: thread.id.clone(),
            sender: "user".to_string(),
            role: MessageRole::User,
            body: goal.to_string(),
            mentions: addressed_agents.to_vec(),
            created_at: timestamp,
        };

        self.plan_run(
            RunRequest {
                owner,
                thread,
                message,
                trigger: RunTrigger::SpaceMessage,
                goal: goal.to_string(),
                addressed_agents: addressed_agents.to_vec(),
            },
            context,
        )
    }

    pub fn plan_run(&self, request: RunRequest, context: ContextView) -> PlannedRun {
        let timestamp = current_timestamp();
        let run_id = format!("run-{}", request.message.id);
        let task_kind = match request.thread.kind {
            ThreadKind::Private => TaskKind::Response,
            ThreadKind::Space => TaskKind::Collaboration,
        };

        let assigned_agents = if request.addressed_agents.is_empty() {
            vec!["system".to_string()]
        } else {
            request.addressed_agents.clone()
        };

        let run = RunSpec {
            id: run_id.clone(),
            owner: request.owner.clone(),
            thread_id: request.thread.id.clone(),
            trigger: trigger_label(&request.trigger).to_string(),
            status: RunStatus::Pending,
            goal: request.goal.clone(),
            created_at: timestamp.clone(),
            updated_at: timestamp.clone(),
        };

        let tasks = assigned_agents
            .into_iter()
            .enumerate()
            .map(|(index, agent_id)| TaskSpec {
                id: format!("task-{run_id}-{}", index + 1),
                run_id: run_id.clone(),
                task_kind: task_kind.clone(),
                title: format!("{} · {}", request.goal, agent_id),
                assigned_agent_id: agent_id,
                status: TaskStatus::Pending,
                created_at: timestamp.clone(),
                updated_at: timestamp.clone(),
            })
            .collect();

        PlannedRun {
            thread: request.thread,
            message: request.message,
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

fn current_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_secs();
    format!("{seconds}")
}
