use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ExtensionManifest, MessageRole, MessageSpec,
    OwnerKind, OwnerRef, RunSpec, RunStatus, SpaceSpec, TaskKind, TaskSpec, TaskStatus, ThreadKind,
    ThreadSpec,
};
use ennoia_memory::{MemoryKind, MemoryRecord};
use ennoia_orchestrator::PlannedRun;
use serde::Serialize;
use sqlx::{Row, SqlitePool};

const SCHEMA_SQL: &str = include_str!("../../../migrations/0001_ennoia_bootstrap.sql");

#[derive(Debug, Clone, Serialize)]
pub struct JobRecord {
    pub id: String,
    pub owner_kind: String,
    pub owner_id: String,
    pub schedule_kind: String,
    pub schedule_value: String,
    pub description: String,
    pub status: String,
}

pub async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in SCHEMA_SQL
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
    {
        sqlx::query(statement).execute(pool).await?;
    }
    Ok(())
}

pub async fn upsert_agents(pool: &SqlitePool, agents: &[AgentConfig]) -> Result<(), sqlx::Error> {
    for agent in agents {
        sqlx::query(
            "INSERT OR REPLACE INTO agents \
            (id, display_name, kind, workspace_mode, default_model, skills_dir, workspace_dir, artifacts_dir) \
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&agent.id)
        .bind(&agent.display_name)
        .bind(&agent.kind)
        .bind(&agent.workspace_mode)
        .bind(&agent.default_model)
        .bind(&agent.skills_dir)
        .bind(&agent.workspace_dir)
        .bind(&agent.artifacts_dir)
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn upsert_spaces(pool: &SqlitePool, spaces: &[SpaceSpec]) -> Result<(), sqlx::Error> {
    for space in spaces {
        sqlx::query(
            "INSERT OR REPLACE INTO spaces (id, display_name, mention_policy, default_agents_json) \
            VALUES (?, ?, ?, ?)",
        )
        .bind(&space.id)
        .bind(&space.display_name)
        .bind(&space.mention_policy)
        .bind(json_or_default(&space.default_agents))
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
    .bind(owner_kind_as_str(&thread.owner.kind))
    .bind(&thread.owner.id)
    .bind(&thread.space_id)
    .bind(thread_kind_as_str(&thread.kind))
    .bind(&thread.title)
    .bind(json_or_default(&thread.participants))
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
    .bind(message_role_as_str(&message.role))
    .bind(&message.body)
    .bind(json_or_default(&message.mentions))
    .bind(&message.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_planned_run(
    pool: &SqlitePool,
    planned_run: &PlannedRun,
) -> Result<(), sqlx::Error> {
    upsert_thread(pool, &planned_run.thread).await?;
    insert_message(pool, &planned_run.message).await?;

    sqlx::query(
        "INSERT OR REPLACE INTO runs \
        (id, owner_kind, owner_id, thread_id, trigger, status, goal, created_at, updated_at) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&planned_run.run.id)
    .bind(owner_kind_as_str(&planned_run.run.owner.kind))
    .bind(&planned_run.run.owner.id)
    .bind(&planned_run.run.thread_id)
    .bind(&planned_run.run.trigger)
    .bind(run_status_as_str(&planned_run.run.status))
    .bind(&planned_run.run.goal)
    .bind(&planned_run.run.created_at)
    .bind(&planned_run.run.updated_at)
    .execute(pool)
    .await?;

    for task in &planned_run.tasks {
        sqlx::query(
            "INSERT OR REPLACE INTO tasks \
            (id, run_id, task_kind, title, assigned_agent_id, status, created_at, updated_at) \
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&task.run_id)
        .bind(task_kind_as_str(&task.task_kind))
        .bind(&task.title)
        .bind(&task.assigned_agent_id)
        .bind(task_status_as_str(&task.status))
        .bind(&task.created_at)
        .bind(&task.updated_at)
        .execute(pool)
        .await?;
    }

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
    .bind(owner_kind_as_str(&artifact.owner.kind))
    .bind(&artifact.owner.id)
    .bind(&artifact.run_id)
    .bind(artifact_kind_as_str(&artifact.kind))
    .bind(&artifact.relative_path)
    .bind(&artifact.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_memory(pool: &SqlitePool, memory: &MemoryRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO memories \
        (id, owner_kind, owner_id, thread_id, run_id, memory_kind, source, content, summary, created_at) \
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&memory.id)
    .bind(owner_kind_as_str(&memory.owner.kind))
    .bind(&memory.owner.id)
    .bind(&memory.thread_id)
    .bind(&memory.run_id)
    .bind(memory_kind_as_str(&memory.kind))
    .bind(&memory.source)
    .bind(&memory.content)
    .bind(&memory.summary)
    .bind(&memory.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_job(pool: &SqlitePool, job: &JobRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO jobs \
        (id, owner_kind, owner_id, schedule_kind, schedule_value, description, status) \
        VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&job.id)
    .bind(&job.owner_kind)
    .bind(&job.owner_id)
    .bind(&job.schedule_kind)
    .bind(&job.schedule_value)
    .bind(&job.description)
    .bind(&job.status)
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
        "SELECT id, owner_kind, owner_id, thread_id, trigger, status, goal, created_at, updated_at \
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
        "SELECT id, owner_kind, owner_id, thread_id, trigger, status, goal, created_at, updated_at \
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
    .bind(owner_kind_as_str(&owner.kind))
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

pub async fn list_jobs(pool: &SqlitePool) -> Result<Vec<JobRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, schedule_kind, schedule_value, description, status \
        FROM jobs ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| JobRecord {
            id: row.get("id"),
            owner_kind: row.get("owner_kind"),
            owner_id: row.get("owner_id"),
            schedule_kind: row.get("schedule_kind"),
            schedule_value: row.get("schedule_value"),
            description: row.get("description"),
            status: row.get("status"),
        })
        .collect())
}

pub async fn list_memories(pool: &SqlitePool) -> Result<Vec<MemoryRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, thread_id, run_id, memory_kind, source, content, summary, created_at \
        FROM memories ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_memory).collect())
}

pub async fn load_memories_for_owner(
    pool: &SqlitePool,
    owner: &OwnerRef,
) -> Result<Vec<MemoryRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, thread_id, run_id, memory_kind, source, content, summary, created_at \
        FROM memories WHERE owner_kind = ? AND owner_id = ? ORDER BY created_at DESC",
    )
    .bind(owner_kind_as_str(&owner.kind))
    .bind(&owner.id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_memory).collect())
}

pub async fn load_memories_for_thread(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<MemoryRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, thread_id, run_id, memory_kind, source, content, summary, created_at \
        FROM memories WHERE thread_id = ? ORDER BY created_at DESC",
    )
    .bind(thread_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_memory).collect())
}

pub async fn load_memories_for_run(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<MemoryRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, thread_id, run_id, memory_kind, source, content, summary, created_at \
        FROM memories WHERE run_id = ? ORDER BY created_at DESC",
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_memory).collect())
}

pub async fn count_rows(pool: &SqlitePool, table: &str) -> Result<i64, sqlx::Error> {
    let statement = format!("SELECT COUNT(*) AS count FROM {table}");
    let row = sqlx::query(&statement).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count"))
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
    .bind(json_or_default(&manifest.contributes.pages))
    .bind(json_or_default(&manifest.contributes.panels))
    .bind(json_or_default(&manifest.contributes.commands))
    .bind(json_or_default(&manifest.contributes.themes))
    .bind(json_or_default(&manifest.contributes.hooks))
    .bind(json_or_default(&manifest.contributes.providers))
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
        status: run_status_from_str(&row.get::<String, _>("status")),
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

fn map_memory(row: sqlx::sqlite::SqliteRow) -> MemoryRecord {
    MemoryRecord {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        thread_id: row.get("thread_id"),
        run_id: row.get("run_id"),
        kind: memory_kind_from_str(&row.get::<String, _>("memory_kind")),
        source: row.get("source"),
        content: row.get("content"),
        summary: row.get("summary"),
        created_at: row.get("created_at"),
    }
}

fn owner_kind_as_str(kind: &OwnerKind) -> &'static str {
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

fn thread_kind_as_str(kind: &ThreadKind) -> &'static str {
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

fn message_role_as_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::User => "user",
        MessageRole::Agent => "agent",
        MessageRole::System => "system",
    }
}

fn message_role_from_str(value: &str) -> MessageRole {
    match value {
        "agent" => MessageRole::Agent,
        "system" => MessageRole::System,
        _ => MessageRole::User,
    }
}

fn run_status_as_str(status: &RunStatus) -> &'static str {
    match status {
        RunStatus::Pending => "pending",
        RunStatus::Running => "running",
        RunStatus::Blocked => "blocked",
        RunStatus::Completed => "completed",
    }
}

fn run_status_from_str(value: &str) -> RunStatus {
    match value {
        "running" => RunStatus::Running,
        "blocked" => RunStatus::Blocked,
        "completed" => RunStatus::Completed,
        _ => RunStatus::Pending,
    }
}

fn task_kind_as_str(kind: &TaskKind) -> &'static str {
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

fn task_status_as_str(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Pending => "pending",
        TaskStatus::Running => "running",
        TaskStatus::Failed => "failed",
        TaskStatus::Completed => "completed",
    }
}

fn task_status_from_str(value: &str) -> TaskStatus {
    match value {
        "running" => TaskStatus::Running,
        "failed" => TaskStatus::Failed,
        "completed" => TaskStatus::Completed,
        _ => TaskStatus::Pending,
    }
}

fn artifact_kind_as_str(kind: &ArtifactKind) -> &'static str {
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

fn memory_kind_as_str(kind: &MemoryKind) -> &'static str {
    match kind {
        MemoryKind::Truth => "truth",
        MemoryKind::Working => "working",
        MemoryKind::Review => "review",
        MemoryKind::Projection => "projection",
    }
}

fn memory_kind_from_str(value: &str) -> MemoryKind {
    match value {
        "truth" => MemoryKind::Truth,
        "review" => MemoryKind::Review,
        "projection" => MemoryKind::Projection,
        _ => MemoryKind::Working,
    }
}

fn json_or_default<T>(value: &T) -> String
where
    T: serde::Serialize,
{
    serde_json::to_string(value).unwrap_or_default()
}
