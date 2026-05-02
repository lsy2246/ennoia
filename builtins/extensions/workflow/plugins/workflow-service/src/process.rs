use std::collections::BTreeMap;
use std::error::Error;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::Arc;

use ennoia_contract::behavior::{
    BehaviorRunRequest, BehaviorSourceRef, BehaviorStatusResponse, BehaviorTrigger,
};
use ennoia_kernel::{
    DecisionSnapshot, ExtensionRpcResponse, GateSeverity, GateVerdict, OwnerRef, RunContext,
    RunSpec, TaskSpec,
};
use ennoia_paths::RuntimePaths;
use ennoia_policy::PolicySet;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};

use crate::orchestrator::OrchestratorService;
use crate::pipeline::{run_behavior, WorkflowRuntime};
use crate::runtime::{
    builtin_pipeline, initialize_workflow_schema, PolicyStageMachine, RuntimeStore,
    SqliteRuntimeStore,
};

#[derive(Debug, Deserialize)]
struct Invocation {
    method: String,
    #[serde(default)]
    params: JsonValue,
    #[serde(default)]
    context: JsonValue,
}

#[derive(Debug, Deserialize)]
struct CreateRunPayload {
    owner: OwnerRef,
    goal: String,
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    #[serde(default)]
    addressed_agents: Vec<String>,
    #[serde(default)]
    context: Option<RunContext>,
    #[serde(default)]
    source_refs: Vec<BehaviorSourceRef>,
    #[serde(default)]
    metadata: JsonValue,
}

#[derive(Debug, Deserialize)]
struct RunIdPayload {
    run_id: String,
}

#[derive(Debug, Deserialize)]
struct RunListPayload {
    #[serde(default)]
    conversation_id: Option<String>,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default)]
    trigger: Option<String>,
    #[serde(default)]
    q: Option<String>,
    #[serde(default)]
    limit: Option<u32>,
}

#[derive(Debug, Serialize)]
struct WorkflowWorkspaceSummary {
    runs_total: i64,
    runs_active: i64,
    runs_blocked: i64,
    runs_completed: i64,
    runs_failed: i64,
    tasks_total: i64,
    artifacts_total: i64,
    handoffs_total: i64,
    decisions_total: i64,
    gate_verdicts_total: i64,
    latest_run_id: Option<String>,
    latest_run_stage: Option<String>,
    latest_goal: Option<String>,
    latest_updated_at: Option<String>,
}

#[derive(Debug, Serialize)]
struct WorkflowRunDetail {
    run: RunSpec,
    #[serde(default)]
    tasks: Vec<TaskSpec>,
    #[serde(default)]
    artifacts: Vec<ennoia_kernel::ArtifactSpec>,
    #[serde(default)]
    handoffs: Vec<ennoia_kernel::HandoffSpec>,
    #[serde(default)]
    stage_events: Vec<ennoia_kernel::RunStageEvent>,
    #[serde(default)]
    gate_verdicts: Vec<GateVerdict>,
    #[serde(default)]
    decisions: Vec<DecisionSnapshot>,
}

#[derive(Clone)]
struct WorkflowServiceState {
    runtime: WorkflowRuntime,
    store: SqliteRuntimeStore,
    pool: SqlitePool,
}

pub async fn run() -> Result<(), Box<dyn Error + Send + Sync>> {
    let runtime_paths = Arc::new(RuntimePaths::resolve(None));
    runtime_paths.ensure_layout()?;

    let database_path = runtime_paths.extension_sqlite_db("workflow", "workflow.db");
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(
            SqliteConnectOptions::new()
                .filename(&database_path)
                .create_if_missing(true),
        )
        .await?;
    initialize_workflow_schema(&pool).await?;

    let policies =
        PolicySet::load(runtime_paths.policies_dir()).unwrap_or_else(|_| PolicySet::builtin());
    let stage_machine = Arc::new(PolicyStageMachine::new(Arc::new(policies.stage)));
    let store = SqliteRuntimeStore::new(pool.clone());
    let runtime_store: Arc<dyn RuntimeStore> = Arc::new(store.clone());
    let orchestrator = OrchestratorService::new(stage_machine, builtin_pipeline());
    let state = WorkflowServiceState {
        runtime: WorkflowRuntime {
            runtime_paths,
            pool: pool.clone(),
            runtime_store,
            orchestrator,
            agents_fallback: Vec::new(),
        },
        store,
        pool,
    };

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut reader = BufReader::new(stdin.lock());
    let mut writer = stdout.lock();
    let mut line = String::new();

    loop {
        line.clear();
        let read = reader.read_line(&mut line)?;
        if read == 0 {
            break;
        }

        let response = match serde_json::from_str::<Invocation>(line.trim_end()) {
            Ok(invocation) => handle_invocation(&state, invocation).await,
            Err(error) => ExtensionRpcResponse::failure("invalid_request", error.to_string()),
        };

        serde_json::to_writer(&mut writer, &response)?;
        writer.write_all(b"\n")?;
        writer.flush()?;
    }

    Ok(())
}

async fn handle_invocation(
    state: &WorkflowServiceState,
    invocation: Invocation,
) -> ExtensionRpcResponse {
    let path = invocation.method.trim_matches('/');
    let _context = invocation.context;
    match path {
        "behavior/status" => {
            ExtensionRpcResponse::success(serde_json::json!(BehaviorStatusResponse {
                extension_id: "workflow".to_string(),
                behavior_id: "default".to_string(),
                healthy: true,
                interfaces: vec![
                    "runs".to_string(),
                    "tasks".to_string(),
                    "artifacts".to_string(),
                    "handoffs".to_string(),
                    "status".to_string(),
                ],
            }))
        }
        "behavior/runs"
        | "behavior/run"
        | "behavior/start"
        | "workflow/runs/create"
        | "workflow/schedules/run" => match parse_run_request(invocation.params) {
            Ok(payload) => match run_behavior(&state.runtime, payload).await {
                Ok(response) => ExtensionRpcResponse::success(serde_json::json!(response)),
                Err(error) => ExtensionRpcResponse::failure("workflow_run_failed", error),
            },
            Err(error) => error,
        },
        "workflow/runs/get" | "run-detail" => match parse_json::<RunIdPayload>(invocation.params) {
            Ok(payload) => match load_run_detail(&state.store, &payload.run_id).await {
                Ok(Some(detail)) => ExtensionRpcResponse::success(serde_json::json!(detail)),
                Ok(None) => ExtensionRpcResponse::failure(
                    "run_not_found",
                    format!("run '{}' not found", payload.run_id),
                ),
                Err(error) => ExtensionRpcResponse::failure("run_detail_failed", error.to_string()),
            },
            Err(error) => error,
        },
        "workflow/runs/list-by-conversation" => {
            match parse_json::<RunListPayload>(invocation.params) {
                Ok(payload) => match list_runs(&state.pool, payload).await {
                    Ok(runs) => ExtensionRpcResponse::success(serde_json::json!(runs)),
                    Err(error) => {
                        ExtensionRpcResponse::failure("run_list_failed", error.to_string())
                    }
                },
                Err(error) => error,
            }
        }
        "workflow/tasks/list-by-run" => match parse_json::<RunIdPayload>(invocation.params) {
            Ok(payload) => match state.store.list_tasks_for_run(&payload.run_id).await {
                Ok(tasks) => ExtensionRpcResponse::success(serde_json::json!(tasks)),
                Err(error) => ExtensionRpcResponse::failure("task_list_failed", error.to_string()),
            },
            Err(error) => error,
        },
        "workflow/artifacts/list-by-run" => match parse_json::<RunIdPayload>(invocation.params) {
            Ok(payload) => match state.store.list_artifacts_for_run(&payload.run_id).await {
                Ok(artifacts) => ExtensionRpcResponse::success(serde_json::json!(artifacts)),
                Err(error) => {
                    ExtensionRpcResponse::failure("artifact_list_failed", error.to_string())
                }
            },
            Err(error) => error,
        },
        "workspace" => match workspace_summary(&state.pool).await {
            Ok(summary) => ExtensionRpcResponse::success(serde_json::json!(summary)),
            Err(error) => {
                ExtensionRpcResponse::failure("workflow_workspace_failed", error.to_string())
            }
        },
        "runs-list" => match parse_json::<RunListPayload>(invocation.params) {
            Ok(payload) => match list_runs(&state.pool, payload).await {
                Ok(runs) => ExtensionRpcResponse::success(serde_json::json!(runs)),
                Err(error) => ExtensionRpcResponse::failure("run_list_failed", error.to_string()),
            },
            Err(error) => error,
        },
        _ => ExtensionRpcResponse::failure(
            "method_not_found",
            format!("workflow worker method '{path}' not found"),
        ),
    }
}

fn parse_run_request(value: JsonValue) -> Result<BehaviorRunRequest, ExtensionRpcResponse> {
    let payload = parse_json::<CreateRunPayload>(value)?;
    Ok(BehaviorRunRequest {
        owner: payload.owner,
        goal: payload.goal,
        trigger: normalize_trigger(payload.trigger.as_deref()),
        participants: payload.participants,
        addressed_agents: payload.addressed_agents,
        context: payload.context.unwrap_or_default(),
        source_refs: payload.source_refs,
        metadata: payload.metadata,
    })
}

fn normalize_trigger(value: Option<&str>) -> BehaviorTrigger {
    let normalized = value.unwrap_or("manual").trim().to_ascii_lowercase();
    if normalized.contains("message") {
        return BehaviorTrigger::Message;
    }
    match normalized.as_str() {
        "schedule" => BehaviorTrigger::Schedule,
        "handoff" => BehaviorTrigger::Handoff,
        "external" => BehaviorTrigger::External,
        _ => BehaviorTrigger::Manual,
    }
}

async fn load_run_detail(
    store: &SqliteRuntimeStore,
    run_id: &str,
) -> Result<Option<WorkflowRunDetail>, Box<dyn Error + Send + Sync>> {
    let Some(run) = store.get_run(run_id).await? else {
        return Ok(None);
    };
    let tasks = store.list_tasks_for_run(run_id).await?;
    let artifacts = store.list_artifacts_for_run(run_id).await?;
    let handoffs = store.list_handoffs_for_run(run_id).await?;
    let stage_events = store.list_stage_events_for_run(run_id).await?;
    let gate_verdicts = store
        .list_gate_verdicts_for_run(run_id)
        .await?
        .into_iter()
        .map(map_gate_record)
        .collect();
    let decisions = store.list_decisions_for_run(run_id).await?;

    Ok(Some(WorkflowRunDetail {
        run,
        tasks,
        artifacts,
        handoffs,
        stage_events,
        gate_verdicts,
        decisions,
    }))
}

fn map_gate_record(record: ennoia_kernel::GateRecord) -> GateVerdict {
    match record.verdict.as_str() {
        "deny" => GateVerdict {
            gate_name: record.gate_name,
            allow: false,
            severity: GateSeverity::Deny,
            reason: record.reason.unwrap_or_else(|| "denied".to_string()),
        },
        "warn" => GateVerdict {
            gate_name: record.gate_name,
            allow: true,
            severity: GateSeverity::Warn,
            reason: record.reason.unwrap_or_else(|| "warning".to_string()),
        },
        _ => GateVerdict {
            gate_name: record.gate_name,
            allow: true,
            severity: GateSeverity::Info,
            reason: record.reason.unwrap_or_else(|| "ok".to_string()),
        },
    }
}

async fn list_runs(
    pool: &SqlitePool,
    payload: RunListPayload,
) -> Result<Vec<RunSpec>, Box<dyn Error + Send + Sync>> {
    let limit = payload.limit.unwrap_or(100).clamp(1, 500) as i64;
    let rows = sqlx::query("SELECT payload_json FROM runs ORDER BY updated_at DESC LIMIT ?1")
        .bind(limit)
        .fetch_all(pool)
        .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let payload_json: String = row.get("payload_json");
        items.push(serde_json::from_str::<RunSpec>(&payload_json)?);
    }

    let q = payload
        .q
        .as_deref()
        .map(|value| value.trim().to_ascii_lowercase());
    Ok(items
        .into_iter()
        .filter(|run| {
            if let Some(conversation_id) = payload.conversation_id.as_deref() {
                if run.conversation_id != conversation_id {
                    return false;
                }
            }
            if let Some(stage) = payload.stage.as_deref() {
                if run.stage.as_str() != stage {
                    return false;
                }
            }
            if let Some(trigger) = payload.trigger.as_deref() {
                if run.trigger != trigger {
                    return false;
                }
            }
            if let Some(keyword) = q.as_deref() {
                let haystack = format!(
                    "{}\n{}\n{}\n{}",
                    run.id, run.goal, run.conversation_id, run.trigger
                )
                .to_ascii_lowercase();
                if !haystack.contains(keyword) {
                    return false;
                }
            }
            true
        })
        .collect())
}

async fn workspace_summary(
    pool: &SqlitePool,
) -> Result<WorkflowWorkspaceSummary, Box<dyn Error + Send + Sync>> {
    let runs_total = count_table(pool, "runs").await?;
    let tasks_total = count_table(pool, "tasks").await?;
    let artifacts_total = count_table(pool, "artifacts").await?;
    let handoffs_total = count_table(pool, "handoffs").await?;
    let decisions_total = count_table(pool, "decisions").await?;
    let gate_verdicts_total = count_table(pool, "gate_verdicts").await?;

    let rows = sqlx::query("SELECT stage, COUNT(*) AS count FROM runs GROUP BY stage")
        .fetch_all(pool)
        .await?;
    let mut by_stage = BTreeMap::new();
    for row in rows {
        let stage: String = row.get("stage");
        let count: i64 = row.get("count");
        by_stage.insert(stage, count);
    }

    let latest_run =
        sqlx::query("SELECT payload_json, updated_at FROM runs ORDER BY updated_at DESC LIMIT 1")
            .fetch_optional(pool)
            .await?;
    let (latest_run_id, latest_run_stage, latest_goal, latest_updated_at) =
        if let Some(row) = latest_run {
            let payload_json: String = row.get("payload_json");
            let run = serde_json::from_str::<RunSpec>(&payload_json)?;
            (
                Some(run.id),
                Some(run.stage.as_str().to_string()),
                Some(run.goal),
                Some(row.get("updated_at")),
            )
        } else {
            (None, None, None, None)
        };

    let runs_completed = *by_stage.get("completed").unwrap_or(&0);
    let runs_failed = *by_stage.get("failed").unwrap_or(&0);
    let runs_blocked = *by_stage.get("blocked").unwrap_or(&0);
    let runs_cancelled = *by_stage.get("cancelled").unwrap_or(&0);
    let runs_active = runs_total - runs_completed - runs_failed - runs_cancelled;

    Ok(WorkflowWorkspaceSummary {
        runs_total,
        runs_active,
        runs_blocked,
        runs_completed,
        runs_failed,
        tasks_total,
        artifacts_total,
        handoffs_total,
        decisions_total,
        gate_verdicts_total,
        latest_run_id,
        latest_run_stage,
        latest_goal,
        latest_updated_at,
    })
}

async fn count_table(pool: &SqlitePool, table: &str) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let sql = format!("SELECT COUNT(*) AS count FROM {table}");
    Ok(sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await?)
}

fn parse_json<T>(value: JsonValue) -> Result<T, ExtensionRpcResponse>
where
    T: for<'de> Deserialize<'de>,
{
    match value {
        JsonValue::Null => serde_json::from_value(JsonValue::Object(Default::default()))
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string())),
        other => serde_json::from_value(other)
            .map_err(|error| ExtensionRpcResponse::failure("invalid_params", error.to_string())),
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_trigger;
    use ennoia_contract::behavior::BehaviorTrigger;

    #[test]
    fn maps_message_like_triggers() {
        assert_eq!(
            normalize_trigger(Some("conversation_message")),
            BehaviorTrigger::Message
        );
        assert_eq!(normalize_trigger(Some("message")), BehaviorTrigger::Message);
    }

    #[test]
    fn maps_known_triggers() {
        assert_eq!(
            normalize_trigger(Some("schedule")),
            BehaviorTrigger::Schedule
        );
        assert_eq!(normalize_trigger(Some("handoff")), BehaviorTrigger::Handoff);
        assert_eq!(
            normalize_trigger(Some("external")),
            BehaviorTrigger::External
        );
        assert_eq!(normalize_trigger(Some("manual")), BehaviorTrigger::Manual);
        assert_eq!(normalize_trigger(None), BehaviorTrigger::Manual);
    }
}
