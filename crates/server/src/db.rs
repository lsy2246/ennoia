use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ExtensionManifest, MessageRole, MessageSpec,
    OwnerKind, OwnerRef, RunSpec, RunStage, SpaceSpec, TaskKind, TaskSpec, TaskStatus, ThreadKind,
    ThreadSpec,
};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

pub const SCHEMA_SQL: &str = include_str!("../../../migrations/0001_ennoia_core.sql");
pub const SYSTEM_CONFIG_SQL: &str =
    include_str!("../../../migrations/0002_system_config.sql");
pub const AUTH_SQL: &str = include_str!("../../../migrations/0003_auth.sql");

#[derive(Debug, Clone, Serialize)]
pub struct JobRow {
    pub id: String,
    pub owner_kind: String,
    pub owner_id: String,
    pub job_kind: String,
    pub schedule_kind: String,
    pub schedule_value: String,
    pub status: String,
    pub next_run_at: Option<String>,
    pub created_at: String,
}

pub async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for migration in [SCHEMA_SQL, SYSTEM_CONFIG_SQL, AUTH_SQL] {
        for statement in migration
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            sqlx::query(statement).execute(pool).await?;
        }
    }
    Ok(())
}

pub async fn upsert_agents(pool: &SqlitePool, agents: &[AgentConfig]) -> Result<(), sqlx::Error> {
    let now = now_iso();
    for agent in agents {
        sqlx::query(
            "INSERT INTO agents \
             (id, display_name, kind, workspace_mode, default_model, skills_dir, workspace_dir, artifacts_dir, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
               display_name = excluded.display_name, \
               kind = excluded.kind, \
               workspace_mode = excluded.workspace_mode, \
               default_model = excluded.default_model, \
               skills_dir = excluded.skills_dir, \
               workspace_dir = excluded.workspace_dir, \
               artifacts_dir = excluded.artifacts_dir, \
               updated_at = excluded.updated_at",
        )
        .bind(&agent.id)
        .bind(&agent.display_name)
        .bind(&agent.kind)
        .bind(&agent.workspace_mode)
        .bind(&agent.default_model)
        .bind(&agent.skills_dir)
        .bind(&agent.workspace_dir)
        .bind(&agent.artifacts_dir)
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn upsert_spaces(pool: &SqlitePool, spaces: &[SpaceSpec]) -> Result<(), sqlx::Error> {
    let now = now_iso();
    for space in spaces {
        sqlx::query(
            "INSERT INTO spaces (id, display_name, mention_policy, default_agents_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
               display_name = excluded.display_name, \
               mention_policy = excluded.mention_policy, \
               default_agents_json = excluded.default_agents_json, \
               updated_at = excluded.updated_at",
        )
        .bind(&space.id)
        .bind(&space.display_name)
        .bind(&space.mention_policy)
        .bind(serde_json::to_string(&space.default_agents).unwrap_or_else(|_| "[]".to_string()))
        .bind(&now)
        .bind(&now)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn upsert_extensions(
    pool: &SqlitePool,
    registry: &ExtensionRegistry,
) -> Result<(), sqlx::Error> {
    for item in registry.items() {
        persist_extension(pool, &item.manifest, &item.install_dir).await?;
    }
    Ok(())
}

pub async fn upsert_thread(pool: &SqlitePool, thread: &ThreadSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO threads \
         (id, owner_kind, owner_id, space_id, thread_kind, title, participants_json, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
           owner_kind = excluded.owner_kind, \
           owner_id = excluded.owner_id, \
           space_id = excluded.space_id, \
           thread_kind = excluded.thread_kind, \
           title = excluded.title, \
           participants_json = excluded.participants_json, \
           updated_at = excluded.updated_at",
    )
    .bind(&thread.id)
    .bind(owner_kind_str(&thread.owner.kind))
    .bind(&thread.owner.id)
    .bind(&thread.space_id)
    .bind(thread_kind_str(&thread.kind))
    .bind(&thread.title)
    .bind(serde_json::to_string(&thread.participants).unwrap_or_else(|_| "[]".to_string()))
    .bind(&thread.created_at)
    .bind(&thread.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_message(pool: &SqlitePool, message: &MessageSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO messages \
         (id, thread_id, sender, role, body, mentions_json, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&message.id)
    .bind(&message.thread_id)
    .bind(&message.sender)
    .bind(message_role_str(&message.role))
    .bind(&message.body)
    .bind(serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string()))
    .bind(&message.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_run(pool: &SqlitePool, run: &RunSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO runs \
         (id, thread_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
           stage = excluded.stage, \
           goal = excluded.goal, \
           updated_at = excluded.updated_at",
    )
    .bind(&run.id)
    .bind(&run.thread_id)
    .bind(owner_kind_str(&run.owner.kind))
    .bind(&run.owner.id)
    .bind(&run.trigger)
    .bind(&run.goal)
    .bind(run.stage.as_str())
    .bind(&run.created_at)
    .bind(&run.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_task(pool: &SqlitePool, task: &TaskSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO tasks \
         (id, run_id, task_kind, title, assigned_agent_id, status, created_at, updated_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?) \
         ON CONFLICT(id) DO UPDATE SET \
           task_kind = excluded.task_kind, \
           title = excluded.title, \
           assigned_agent_id = excluded.assigned_agent_id, \
           status = excluded.status, \
           updated_at = excluded.updated_at",
    )
    .bind(&task.id)
    .bind(&task.run_id)
    .bind(task_kind_str(&task.task_kind))
    .bind(&task.title)
    .bind(&task.assigned_agent_id)
    .bind(task_status_str(&task.status))
    .bind(&task.created_at)
    .bind(&task.updated_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_artifact(
    pool: &SqlitePool,
    artifact: &ArtifactSpec,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO artifacts \
         (id, owner_kind, owner_id, run_id, artifact_kind, relative_path, created_at) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&artifact.id)
    .bind(owner_kind_str(&artifact.owner.kind))
    .bind(&artifact.owner.id)
    .bind(&artifact.run_id)
    .bind(artifact_kind_str(&artifact.kind))
    .bind(&artifact.relative_path)
    .bind(&artifact.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_threads(pool: &SqlitePool) -> Result<Vec<ThreadSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, space_id, thread_kind, title, participants_json, created_at, updated_at \
         FROM threads ORDER BY updated_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_thread).collect())
}

pub async fn list_messages_for_thread(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<MessageSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, thread_id, sender, role, body, mentions_json, created_at \
         FROM messages WHERE thread_id = ? ORDER BY created_at ASC",
    )
    .bind(thread_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_message).collect())
}

pub async fn list_runs(pool: &SqlitePool) -> Result<Vec<RunSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, thread_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at \
         FROM runs ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_runs_for_thread(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<RunSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, thread_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at \
         FROM runs WHERE thread_id = ? ORDER BY created_at DESC",
    )
    .bind(thread_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, run_id, task_kind, title, assigned_agent_id, status, created_at, updated_at \
         FROM tasks ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_tasks_for_run(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, run_id, task_kind, title, assigned_agent_id, status, created_at, updated_at \
         FROM tasks WHERE run_id = ? ORDER BY created_at ASC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_active_tasks_for_owner(
    pool: &SqlitePool,
    owner: &OwnerRef,
    limit: usize,
) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT tasks.id, tasks.run_id, tasks.task_kind, tasks.title, tasks.assigned_agent_id, tasks.status, tasks.created_at, tasks.updated_at \
         FROM tasks INNER JOIN runs ON runs.id = tasks.run_id \
         WHERE runs.owner_kind = ? AND runs.owner_id = ? ORDER BY tasks.created_at DESC LIMIT ?",
    )
    .bind(owner_kind_str(&owner.kind))
    .bind(&owner.id)
    .bind(limit as i64)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_artifacts(pool: &SqlitePool) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, run_id, artifact_kind, relative_path, created_at \
         FROM artifacts ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_artifact).collect())
}

pub async fn list_artifacts_for_run(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, run_id, artifact_kind, relative_path, created_at \
         FROM artifacts WHERE run_id = ? ORDER BY created_at ASC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_artifact).collect())
}

pub async fn count_rows(pool: &SqlitePool, table: &str) -> Result<i64, sqlx::Error> {
    let statement = format!("SELECT COUNT(*) AS count FROM {table}");
    let row = sqlx::query(&statement).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count"))
}

pub async fn list_jobs(pool: &SqlitePool) -> Result<Vec<JobRow>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, job_kind, schedule_kind, schedule_value, status, next_run_at, created_at \
         FROM jobs ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| JobRow {
            id: row.get("id"),
            owner_kind: row.get("owner_kind"),
            owner_id: row.get("owner_id"),
            job_kind: row.get("job_kind"),
            schedule_kind: row.get("schedule_kind"),
            schedule_value: row.get("schedule_value"),
            status: row.get("status"),
            next_run_at: row.get("next_run_at"),
            created_at: row.get("created_at"),
        })
        .collect())
}

async fn persist_extension(
    pool: &SqlitePool,
    manifest: &ExtensionManifest,
    install_dir: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO extensions \
         (id, kind, version, install_dir, frontend_bundle, backend_entry, pages_json, panels_json, commands_json, themes_json, hooks_json, providers_json) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&manifest.id)
    .bind(format!("{:?}", manifest.kind))
    .bind(&manifest.version)
    .bind(install_dir)
    .bind(&manifest.frontend_bundle)
    .bind(&manifest.backend_entry)
    .bind(serde_json::to_string(&manifest.contributes.pages).unwrap_or_else(|_| "[]".to_string()))
    .bind(serde_json::to_string(&manifest.contributes.panels).unwrap_or_else(|_| "[]".to_string()))
    .bind(serde_json::to_string(&manifest.contributes.commands).unwrap_or_else(|_| "[]".to_string()))
    .bind(serde_json::to_string(&manifest.contributes.themes).unwrap_or_else(|_| "[]".to_string()))
    .bind(serde_json::to_string(&manifest.contributes.hooks).unwrap_or_else(|_| "[]".to_string()))
    .bind(
        serde_json::to_string(&manifest.contributes.providers)
            .unwrap_or_else(|_| "[]".to_string()),
    )
    .execute(pool)
    .await?;
    Ok(())
}

fn map_thread(row: sqlx::sqlite::SqliteRow) -> ThreadSpec {
    ThreadSpec {
        id: row.get("id"),
        kind: thread_kind_from_str(&row.get::<String, _>("thread_kind")),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        space_id: row.get("space_id"),
        title: row.get("title"),
        participants: serde_json::from_str(&row.get::<String, _>("participants_json"))
            .unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_message(row: sqlx::sqlite::SqliteRow) -> MessageSpec {
    MessageSpec {
        id: row.get("id"),
        thread_id: row.get("thread_id"),
        sender: row.get("sender"),
        role: message_role_from_str(&row.get::<String, _>("role")),
        body: row.get("body"),
        mentions: serde_json::from_str(&row.get::<String, _>("mentions_json")).unwrap_or_default(),
        created_at: row.get("created_at"),
    }
}

fn map_run(row: sqlx::sqlite::SqliteRow) -> RunSpec {
    RunSpec {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        thread_id: row.get("thread_id"),
        trigger: row.get("trigger"),
        stage: RunStage::from_str(&row.get::<String, _>("stage")),
        goal: row.get("goal"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_task(row: sqlx::sqlite::SqliteRow) -> TaskSpec {
    TaskSpec {
        id: row.get("id"),
        run_id: row.get("run_id"),
        task_kind: task_kind_from_str(&row.get::<String, _>("task_kind")),
        title: row.get("title"),
        assigned_agent_id: row.get("assigned_agent_id"),
        status: task_status_from_str(&row.get::<String, _>("status")),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_artifact(row: sqlx::sqlite::SqliteRow) -> ArtifactSpec {
    ArtifactSpec {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        run_id: row.get("run_id"),
        kind: artifact_kind_from_str(&row.get::<String, _>("artifact_kind")),
        relative_path: row.get("relative_path"),
        created_at: row.get("created_at"),
    }
}

fn owner_kind_str(kind: &OwnerKind) -> &'static str {
    match kind {
        OwnerKind::Global => "global",
        OwnerKind::Agent => "agent",
        OwnerKind::Space => "space",
    }
}

fn owner_kind_from_str(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn thread_kind_str(kind: &ThreadKind) -> &'static str {
    match kind {
        ThreadKind::Private => "private",
        ThreadKind::Space => "space",
    }
}

fn thread_kind_from_str(value: &str) -> ThreadKind {
    match value {
        "space" => ThreadKind::Space,
        _ => ThreadKind::Private,
    }
}

fn message_role_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Agent => "agent",
        MessageRole::System => "system",
        MessageRole::Tool => "tool",
    }
}

fn message_role_from_str(value: &str) -> MessageRole {
    match value {
        "agent" => MessageRole::Agent,
        "system" => MessageRole::System,
        "tool" => MessageRole::Tool,
        _ => MessageRole::User,
    }
}

fn task_kind_str(kind: &TaskKind) -> &'static str {
    match kind {
        TaskKind::Response => "response",
        TaskKind::Collaboration => "collaboration",
        TaskKind::Maintenance => "maintenance",
    }
}

fn task_kind_from_str(value: &str) -> TaskKind {
    match value {
        "collaboration" => TaskKind::Collaboration,
        "maintenance" => TaskKind::Maintenance,
        _ => TaskKind::Response,
    }
}

fn task_status_str(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Completed => "completed",
        TaskStatus::Failed => "failed",
    }
}

fn task_status_from_str(value: &str) -> TaskStatus {
    match value {
        "running" => TaskStatus::Running,
        "completed" => TaskStatus::Completed,
        "failed" => TaskStatus::Failed,
        _ => TaskStatus::Pending,
    }
}

fn artifact_kind_str(kind: &ArtifactKind) -> &'static str {
    match kind {
        ArtifactKind::Screenshot => "screenshot",
        ArtifactKind::Har => "har",
        ArtifactKind::Report => "report",
        ArtifactKind::Export => "export",
        ArtifactKind::Log => "log",
    }
}

fn artifact_kind_from_str(value: &str) -> ArtifactKind {
    match value {
        "screenshot" => ArtifactKind::Screenshot,
        "har" => ArtifactKind::Har,
        "export" => ArtifactKind::Export,
        "log" => ArtifactKind::Log,
        _ => ArtifactKind::Report,
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
