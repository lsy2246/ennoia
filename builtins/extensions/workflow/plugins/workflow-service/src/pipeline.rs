use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ContextLayer, ConversationMessageHookPayload,
    ConversationSpec, ConversationTopology, ConversationWorkflowOutput, LaneSpec, OwnerRef,
    RunContext, RunSpec,
};
use ennoia_paths::RuntimePaths;
use sqlx::SqlitePool;
use uuid::Uuid;

use crate::orchestrator::{OrchestratorService, RunRequest, RunTrigger};
use crate::runtime::RuntimeStore;

#[derive(Clone)]
pub struct WorkflowRuntime {
    pub runtime_paths: Arc<RuntimePaths>,
    pub pool: SqlitePool,
    pub runtime_store: Arc<dyn RuntimeStore>,
    pub orchestrator: OrchestratorService,
    pub agents_fallback: Vec<AgentConfig>,
}

pub async fn run_conversation_workflow(
    runtime: &WorkflowRuntime,
    payload: ConversationMessageHookPayload,
) -> Result<ConversationWorkflowOutput, String> {
    let recent_messages = vec![format!(
        "{}: {}",
        payload.message.sender, payload.message.body
    )];

    let mut context = RunContext {
        recent_messages,
        ..RunContext::default()
    };
    context.total_chars += context
        .recent_messages
        .iter()
        .map(|item| item.chars().count() as u32)
        .sum::<u32>();
    context.push(ContextLayer::Core, format!("goal={}", payload.goal));
    context.push(
        ContextLayer::Execution,
        format!(
            "lane={} participants={}",
            payload.lane.name,
            payload.lane.participants.join(",")
        ),
    );

    let available_agents: Vec<String> = load_agent_configs(&runtime.runtime_paths)
        .unwrap_or_else(|_| runtime.agents_fallback.clone())
        .into_iter()
        .map(|agent| agent.id)
        .collect();
    let request = RunRequest {
        owner: payload.conversation.owner.clone(),
        conversation: payload.conversation.clone(),
        message: payload.message.clone(),
        trigger: match payload.conversation.topology {
            ConversationTopology::Direct => RunTrigger::DirectConversation,
            ConversationTopology::Group => RunTrigger::GroupConversation,
        },
        goal: payload.goal.clone(),
        addressed_agents: payload.message.mentions.clone(),
    };
    let plan = runtime
        .orchestrator
        .plan_run(request, context.clone(), available_agents)
        .await;

    runtime
        .runtime_store
        .log_stage_event(&plan.stage_event)
        .await
        .map_err(|error| error.to_string())?;
    runtime
        .runtime_store
        .log_decision(&plan.decision_snapshot)
        .await
        .map_err(|error| error.to_string())?;
    for record in &plan.gate_records {
        runtime
            .runtime_store
            .log_gate_verdict(record)
            .await
            .map_err(|error| error.to_string())?;
    }

    let artifact = persist_run_artifact(
        &runtime.runtime_paths,
        &plan.run,
        &payload.conversation.owner,
        &payload.goal,
    );

    Ok(ConversationWorkflowOutput {
        conversation: plan.conversation,
        lane: payload.lane,
        message: plan.message,
        run: plan.run,
        tasks: plan.tasks,
        artifacts: vec![artifact],
        context: plan.context,
        gate_verdicts: plan.gate_verdicts,
        stage_event: plan.stage_event,
        decision: plan.decision,
    })
}

pub fn persist_run_artifact(
    runtime_paths: &RuntimePaths,
    run: &RunSpec,
    owner: &OwnerRef,
    goal: &str,
) -> ArtifactSpec {
    let owner_root = runtime_paths.owner_run_artifact_dir(owner, &run.id);
    let _ = fs::create_dir_all(&owner_root);
    let relative_path = runtime_paths.owner_run_artifact_relative_path(owner, &run.id);

    let _ = fs::write(
        owner_root.join("summary.json"),
        serde_json::to_string_pretty(&serde_json::json!({
            "run_id": run.id,
            "conversation_id": run.conversation_id,
            "lane_id": run.lane_id,
            "owner": owner,
            "goal": goal
        }))
        .unwrap_or_default(),
    );

    ArtifactSpec {
        id: format!("art-{}", Uuid::new_v4()),
        owner: owner.clone(),
        run_id: run.id.clone(),
        conversation_id: Some(run.conversation_id.clone()),
        lane_id: run.lane_id.clone(),
        kind: ArtifactKind::Summary,
        relative_path,
        created_at: now_iso(),
    }
}

pub fn resolve_addressed_agents(
    conversation: &ConversationSpec,
    lane: &LaneSpec,
    addressed_agents: Vec<String>,
) -> Vec<String> {
    if !addressed_agents.is_empty() {
        return addressed_agents;
    }

    let source = if !lane.participants.is_empty() {
        &lane.participants
    } else {
        &conversation.participants
    };
    source
        .iter()
        .filter(|participant| participant.as_str() != "operator")
        .cloned()
        .collect()
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}

fn load_agent_configs(
    paths: &RuntimePaths,
) -> Result<Vec<AgentConfig>, Box<dyn std::error::Error + Send + Sync>> {
    let mut agents = load_configs_from_dir::<AgentConfig>(paths.agents_config_dir())?;
    for agent in &mut agents {
        if agent.model_id.is_empty() && !agent.default_model.is_empty() {
            agent.model_id = agent.default_model.clone();
        }
        if agent.default_model.is_empty() && !agent.model_id.is_empty() {
            agent.default_model = agent.model_id.clone();
        }
        if !agent.working_dir.is_empty() {
            agent.working_dir = paths.display_for_user(paths.expand_home_token(&agent.working_dir));
        } else {
            agent.working_dir = paths.display_for_user(paths.agent_working_dir(&agent.id));
        }
        if !agent.skills_dir.is_empty() {
            agent.skills_dir = paths.display_for_user(paths.expand_home_token(&agent.skills_dir));
        } else {
            agent.skills_dir = paths.display_for_user(paths.agent_skills_dir(&agent.id));
        }
        if !agent.artifacts_dir.is_empty() {
            agent.artifacts_dir =
                paths.display_for_user(paths.expand_home_token(&agent.artifacts_dir));
        } else {
            agent.artifacts_dir = paths.display_for_user(paths.agent_artifacts_dir(&agent.id));
        }
    }
    Ok(agents)
}

fn load_configs_from_dir<T>(
    dir: PathBuf,
) -> Result<Vec<T>, Box<dyn std::error::Error + Send + Sync>>
where
    T: serde::de::DeserializeOwned,
{
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut items = Vec::new();
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let contents = fs::read_to_string(entry.path())?;
        items.push(toml::from_str(&contents)?);
    }
    Ok(items)
}
