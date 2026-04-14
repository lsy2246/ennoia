use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{AgentConfig, ExtensionManifest, OwnerKind, OwnerRef, SpaceSpec};
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
        .bind(serde_json::to_string(&space.default_agents).unwrap_or_default())
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
    .bind(serde_json::to_string(&manifest.contributes.pages).unwrap_or_default())
    .bind(serde_json::to_string(&manifest.contributes.panels).unwrap_or_default())
    .bind(serde_json::to_string(&manifest.contributes.commands).unwrap_or_default())
    .bind(serde_json::to_string(&manifest.contributes.themes).unwrap_or_default())
    .bind(serde_json::to_string(&manifest.contributes.hooks).unwrap_or_default())
    .bind(serde_json::to_string(&manifest.contributes.providers).unwrap_or_default())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn insert_planned_run(
    pool: &SqlitePool,
    planned_run: &PlannedRun,
    goal: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO runs (id, owner_kind, owner_id, thread_id, trigger, status, goal) \
        VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&planned_run.run.id)
    .bind(owner_kind_as_str(&planned_run.run.owner.kind))
    .bind(&planned_run.run.owner.id)
    .bind(&planned_run.run.thread_id)
    .bind(&planned_run.run.trigger)
    .bind(format!("{:?}", planned_run.run.status))
    .bind(goal)
    .execute(pool)
    .await?;

    for task in &planned_run.tasks {
        sqlx::query(
            "INSERT OR REPLACE INTO tasks (id, run_id, title, assigned_agent_id, status) \
            VALUES (?, ?, ?, ?, ?)",
        )
        .bind(&task.id)
        .bind(&task.run_id)
        .bind(&task.title)
        .bind(&task.assigned_agent_id)
        .bind(format!("{:?}", task.status))
        .execute(pool)
        .await?;
    }

    Ok(())
}

pub async fn insert_memory(pool: &SqlitePool, memory: &MemoryRecord) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR REPLACE INTO memories \
        (id, owner_kind, owner_id, memory_kind, source, content, summary) \
        VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&memory.id)
    .bind(owner_kind_as_str(&memory.owner.kind))
    .bind(&memory.owner.id)
    .bind(memory_kind_as_str(&memory.kind))
    .bind(&memory.source)
    .bind(&memory.content)
    .bind(&memory.summary)
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

pub async fn list_runs(pool: &SqlitePool) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, thread_id, trigger, status, goal \
        FROM runs ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "owner_kind": row.get::<String, _>("owner_kind"),
                "owner_id": row.get::<String, _>("owner_id"),
                "thread_id": row.get::<String, _>("thread_id"),
                "trigger": row.get::<String, _>("trigger"),
                "status": row.get::<String, _>("status"),
                "goal": row.get::<String, _>("goal")
            })
        })
        .collect())
}

pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<serde_json::Value>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, run_id, title, assigned_agent_id, status FROM tasks ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| {
            serde_json::json!({
                "id": row.get::<String, _>("id"),
                "run_id": row.get::<String, _>("run_id"),
                "title": row.get::<String, _>("title"),
                "assigned_agent_id": row.get::<String, _>("assigned_agent_id"),
                "status": row.get::<String, _>("status")
            })
        })
        .collect())
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
        "SELECT id, owner_kind, owner_id, memory_kind, source, content, summary \
        FROM memories ORDER BY id DESC",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MemoryRecord {
            id: row.get("id"),
            owner: OwnerRef {
                kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
                id: row.get("owner_id"),
            },
            kind: memory_kind_from_str(&row.get::<String, _>("memory_kind")),
            source: row.get("source"),
            content: row.get("content"),
            summary: row.get("summary"),
        })
        .collect())
}

pub async fn load_memories_for_owner(
    pool: &SqlitePool,
    owner: &OwnerRef,
) -> Result<Vec<MemoryRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, memory_kind, source, content, summary \
        FROM memories WHERE owner_kind = ? AND owner_id = ? ORDER BY id DESC",
    )
    .bind(owner_kind_as_str(&owner.kind))
    .bind(&owner.id)
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| MemoryRecord {
            id: row.get("id"),
            owner: OwnerRef {
                kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
                id: row.get("owner_id"),
            },
            kind: memory_kind_from_str(&row.get::<String, _>("memory_kind")),
            source: row.get("source"),
            content: row.get("content"),
            summary: row.get("summary"),
        })
        .collect())
}

pub async fn count_rows(pool: &SqlitePool, table: &str) -> Result<i64, sqlx::Error> {
    let statement = format!("SELECT COUNT(*) AS count FROM {table}");
    let row = sqlx::query(&statement).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count"))
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
