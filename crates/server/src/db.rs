use ennoia_assets::migrations;
use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ConversationSpec, ConversationTopology,
    ExtensionManifest, HandoffSpec, LaneSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef,
    RunSpec, RunStage, SpaceSpec, TaskKind, TaskSpec, TaskStatus, UiPreference, WorkspaceProfile,
};
use serde::Serialize;
use sqlx::{Row, SqlitePool};

const INSTANCE_PREFERENCE_ID: &str = "instance";

#[derive(Debug, Clone, Copy)]
pub enum CountTable {
    Conversations,
    Messages,
    Runs,
    Tasks,
    Artifacts,
    Memories,
    Jobs,
    Decisions,
}

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

#[derive(Debug, Clone, Serialize)]
pub struct LogRecordRow {
    pub id: String,
    pub kind: String,
    pub level: String,
    pub title: String,
    pub summary: String,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UiPreferenceRow {
    pub subject_id: String,
    pub preference: UiPreference,
}

pub async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for migration in migrations::all() {
        for statement in split_sql_statements(migration.contents) {
            if let Err(error) = sqlx::query(&statement).execute(pool).await {
                let message = error.to_string();
                if message.contains("duplicate column name") {
                    continue;
                }
                return Err(error);
            }
        }
    }
    Ok(())
}

fn split_sql_statements(contents: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_trigger = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }
        current.push_str(line);
        current.push('\n');

        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE TEMP TRIGGER") {
            in_trigger = true;
        }
        if in_trigger {
            if upper == "END;" {
                statements.push(current.trim().to_string());
                current.clear();
                in_trigger = false;
            }
            continue;
        }
        if trimmed.ends_with(';') {
            statements.push(current.trim().to_string());
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}

pub async fn upsert_agents(pool: &SqlitePool, agents: &[AgentConfig]) -> Result<(), sqlx::Error> {
    let now = now_iso();
    for agent in agents {
        sqlx::query(
            r#"
            INSERT INTO agents (
              id, display_name, kind, workspace_mode, default_model,
              skills_dir, workspace_dir, artifacts_dir, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              display_name = excluded.display_name,
              kind = excluded.kind,
              workspace_mode = excluded.workspace_mode,
              default_model = excluded.default_model,
              skills_dir = excluded.skills_dir,
              workspace_dir = excluded.workspace_dir,
              artifacts_dir = excluded.artifacts_dir,
              updated_at = excluded.updated_at
            "#,
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
        let default_agents =
            serde_json::to_string(&space.default_agents).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            r#"
            INSERT INTO spaces (
              id, display_name, description, primary_goal, mention_policy,
              default_agents_json, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
              display_name = excluded.display_name,
              description = excluded.description,
              primary_goal = excluded.primary_goal,
              mention_policy = excluded.mention_policy,
              default_agents_json = excluded.default_agents_json,
              updated_at = excluded.updated_at
            "#,
        )
        .bind(&space.id)
        .bind(&space.display_name)
        .bind(&space.description)
        .bind(&space.primary_goal)
        .bind(&space.mention_policy)
        .bind(default_agents)
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

pub async fn ensure_workspace_profile(
    pool: &SqlitePool,
    profile: &WorkspaceProfile,
) -> Result<WorkspaceProfile, sqlx::Error> {
    if let Some(existing) = get_workspace_profile(pool).await? {
        return Ok(existing);
    }
    update_workspace_profile(pool, profile).await
}

pub async fn get_workspace_profile(
    pool: &SqlitePool,
) -> Result<Option<WorkspaceProfile>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, display_name, locale, time_zone, default_space_id, created_at, updated_at
        FROM workspace_profile
        LIMIT 1
        "#,
    )
    .fetch_optional(pool)
    .await?;

    Ok(row.map(map_workspace_profile))
}

pub async fn update_workspace_profile(
    pool: &SqlitePool,
    profile: &WorkspaceProfile,
) -> Result<WorkspaceProfile, sqlx::Error> {
    let created_at = if profile.created_at.is_empty() {
        now_iso()
    } else {
        profile.created_at.clone()
    };
    let updated_at = if profile.updated_at.is_empty() {
        now_iso()
    } else {
        profile.updated_at.clone()
    };

    sqlx::query(
        r#"
        INSERT INTO workspace_profile (
          id, display_name, locale, time_zone, default_space_id, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          display_name = excluded.display_name,
          locale = excluded.locale,
          time_zone = excluded.time_zone,
          default_space_id = excluded.default_space_id,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&profile.id)
    .bind(&profile.display_name)
    .bind(&profile.locale)
    .bind(&profile.time_zone)
    .bind(&profile.default_space_id)
    .bind(&created_at)
    .bind(&updated_at)
    .execute(pool)
    .await?;

    Ok(WorkspaceProfile {
        created_at,
        updated_at,
        ..profile.clone()
    })
}

pub async fn get_instance_ui_preference(
    pool: &SqlitePool,
) -> Result<Option<UiPreferenceRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        FROM instance_ui_preferences
        WHERE id = ?
        "#,
    )
    .bind(INSTANCE_PREFERENCE_ID)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| UiPreferenceRow {
        subject_id: row.get("id"),
        preference: map_ui_preference_row(&row),
    }))
}

pub async fn upsert_instance_ui_preference(
    pool: &SqlitePool,
    preference: &UiPreference,
) -> Result<UiPreferenceRow, sqlx::Error> {
    let updated_at = if preference.updated_at.is_empty() {
        now_iso()
    } else {
        preference.updated_at.clone()
    };

    sqlx::query(
        r#"
        INSERT INTO instance_ui_preferences (
          id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          locale = excluded.locale,
          theme_id = excluded.theme_id,
          time_zone = excluded.time_zone,
          date_style = excluded.date_style,
          density = excluded.density,
          motion = excluded.motion,
          version = excluded.version,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(INSTANCE_PREFERENCE_ID)
    .bind(&preference.locale)
    .bind(&preference.theme_id)
    .bind(&preference.time_zone)
    .bind(&preference.date_style)
    .bind(&preference.density)
    .bind(&preference.motion)
    .bind(preference.version as i64)
    .bind(&updated_at)
    .execute(pool)
    .await?;

    Ok(UiPreferenceRow {
        subject_id: INSTANCE_PREFERENCE_ID.to_string(),
        preference: UiPreference {
            updated_at,
            ..preference.clone()
        },
    })
}

pub async fn get_space_ui_preference(
    pool: &SqlitePool,
    space_id: &str,
) -> Result<Option<UiPreferenceRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT space_id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        FROM space_ui_preferences
        WHERE space_id = ?
        "#,
    )
    .bind(space_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| UiPreferenceRow {
        subject_id: row.get("space_id"),
        preference: map_ui_preference_row(&row),
    }))
}

pub async fn list_space_ui_preferences(
    pool: &SqlitePool,
) -> Result<Vec<UiPreferenceRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT space_id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        FROM space_ui_preferences
        ORDER BY space_id ASC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .map(|row| UiPreferenceRow {
            subject_id: row.get("space_id"),
            preference: map_ui_preference_row(&row),
        })
        .collect())
}

pub async fn upsert_space_ui_preference(
    pool: &SqlitePool,
    space_id: &str,
    preference: &UiPreference,
) -> Result<UiPreferenceRow, sqlx::Error> {
    let updated_at = if preference.updated_at.is_empty() {
        now_iso()
    } else {
        preference.updated_at.clone()
    };

    sqlx::query(
        r#"
        INSERT INTO space_ui_preferences (
          space_id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(space_id) DO UPDATE SET
          locale = excluded.locale,
          theme_id = excluded.theme_id,
          time_zone = excluded.time_zone,
          date_style = excluded.date_style,
          density = excluded.density,
          motion = excluded.motion,
          version = excluded.version,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(space_id)
    .bind(&preference.locale)
    .bind(&preference.theme_id)
    .bind(&preference.time_zone)
    .bind(&preference.date_style)
    .bind(&preference.density)
    .bind(&preference.motion)
    .bind(preference.version as i64)
    .bind(&updated_at)
    .execute(pool)
    .await?;

    Ok(UiPreferenceRow {
        subject_id: space_id.to_string(),
        preference: UiPreference {
            updated_at,
            ..preference.clone()
        },
    })
}

pub async fn max_ui_preference_version(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let instance_max =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM instance_ui_preferences")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
    let space_max =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM space_ui_preferences")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
    Ok(instance_max.max(space_max) as u64)
}

pub async fn list_conversations(pool: &SqlitePool) -> Result<Vec<ConversationSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at
        FROM conversations
        ORDER BY updated_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(map_conversation(pool, row).await?);
    }
    Ok(items)
}

pub async fn get_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Option<ConversationSpec>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at
        FROM conversations
        WHERE id = ?
        "#,
    )
    .bind(conversation_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => Ok(Some(map_conversation(pool, row).await?)),
        None => Ok(None),
    }
}

pub async fn upsert_conversation(
    pool: &SqlitePool,
    conversation: &ConversationSpec,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO conversations (
          id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          topology = excluded.topology,
          owner_kind = excluded.owner_kind,
          owner_id = excluded.owner_id,
          space_id = excluded.space_id,
          title = excluded.title,
          default_lane_id = excluded.default_lane_id,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&conversation.id)
    .bind(conversation_topology_str(&conversation.topology))
    .bind(owner_kind_str(&conversation.owner.kind))
    .bind(&conversation.owner.id)
    .bind(&conversation.space_id)
    .bind(&conversation.title)
    .bind(&conversation.default_lane_id)
    .bind(&conversation.created_at)
    .bind(&conversation.updated_at)
    .execute(pool)
    .await?;

    replace_conversation_participants(pool, &conversation.id, &conversation.participants).await?;
    Ok(())
}

pub async fn list_messages_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<MessageSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, conversation_id, lane_id, sender, role, body, mentions_json, created_at
        FROM messages
        WHERE conversation_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_message).collect())
}

pub async fn insert_message(pool: &SqlitePool, message: &MessageSpec) -> Result<(), sqlx::Error> {
    let mentions = serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string());
    sqlx::query(
        r#"
        INSERT INTO messages (
          id, conversation_id, lane_id, sender, role, body, mentions_json, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO NOTHING
        "#,
    )
    .bind(&message.id)
    .bind(&message.conversation_id)
    .bind(&message.lane_id)
    .bind(&message.sender)
    .bind(message_role_str(&message.role))
    .bind(&message.body)
    .bind(mentions)
    .bind(&message.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_lanes_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<LaneSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, conversation_id, space_id, name, lane_type, status, goal, created_at, updated_at
        FROM lanes
        WHERE conversation_id = ?
        ORDER BY updated_at DESC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(map_lane(pool, row).await?);
    }
    Ok(items)
}

pub async fn get_lane(pool: &SqlitePool, lane_id: &str) -> Result<Option<LaneSpec>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT id, conversation_id, space_id, name, lane_type, status, goal, created_at, updated_at
        FROM lanes
        WHERE id = ?
        "#,
    )
    .bind(lane_id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(row) => Ok(Some(map_lane(pool, row).await?)),
        None => Ok(None),
    }
}

pub async fn insert_lane(pool: &SqlitePool, lane: &LaneSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO lanes (
          id, conversation_id, space_id, name, lane_type, status, goal, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          conversation_id = excluded.conversation_id,
          space_id = excluded.space_id,
          name = excluded.name,
          lane_type = excluded.lane_type,
          status = excluded.status,
          goal = excluded.goal,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&lane.id)
    .bind(&lane.conversation_id)
    .bind(&lane.space_id)
    .bind(&lane.name)
    .bind(&lane.lane_type)
    .bind(&lane.status)
    .bind(&lane.goal)
    .bind(&lane.created_at)
    .bind(&lane.updated_at)
    .execute(pool)
    .await?;

    replace_lane_members(pool, &lane.id, &lane.participants).await?;
    Ok(())
}

pub async fn list_handoffs_for_lane(
    pool: &SqlitePool,
    lane_id: &str,
) -> Result<Vec<HandoffSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, from_lane_id, to_lane_id, from_agent_id, to_agent_id, summary, instructions, status, created_at
        FROM handoffs
        WHERE from_lane_id = ? OR to_lane_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(lane_id)
    .bind(lane_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_handoff).collect())
}

pub async fn insert_handoff(pool: &SqlitePool, handoff: &HandoffSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO handoffs (
          id, from_lane_id, to_lane_id, from_agent_id, to_agent_id, summary, instructions, status, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          from_lane_id = excluded.from_lane_id,
          to_lane_id = excluded.to_lane_id,
          from_agent_id = excluded.from_agent_id,
          to_agent_id = excluded.to_agent_id,
          summary = excluded.summary,
          instructions = excluded.instructions,
          status = excluded.status
        "#,
    )
    .bind(&handoff.id)
    .bind(&handoff.from_lane_id)
    .bind(&handoff.to_lane_id)
    .bind(&handoff.from_agent_id)
    .bind(&handoff.to_agent_id)
    .bind(&handoff.summary)
    .bind(&handoff.instructions)
    .bind(&handoff.status)
    .bind(&handoff.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn upsert_run(pool: &SqlitePool, run: &RunSpec) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO runs (
          id, conversation_id, lane_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          conversation_id = excluded.conversation_id,
          lane_id = excluded.lane_id,
          owner_kind = excluded.owner_kind,
          owner_id = excluded.owner_id,
          trigger = excluded.trigger,
          goal = excluded.goal,
          stage = excluded.stage,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&run.id)
    .bind(&run.conversation_id)
    .bind(&run.lane_id)
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
        r#"
        INSERT INTO tasks (
          id, run_id, conversation_id, lane_id, task_kind, title, assigned_agent_id, status, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          run_id = excluded.run_id,
          conversation_id = excluded.conversation_id,
          lane_id = excluded.lane_id,
          task_kind = excluded.task_kind,
          title = excluded.title,
          assigned_agent_id = excluded.assigned_agent_id,
          status = excluded.status,
          updated_at = excluded.updated_at
        "#,
    )
    .bind(&task.id)
    .bind(&task.run_id)
    .bind(&task.conversation_id)
    .bind(&task.lane_id)
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
        r#"
        INSERT INTO artifacts (
          id, owner_kind, owner_id, run_id, conversation_id, lane_id, artifact_kind, relative_path, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO NOTHING
        "#,
    )
    .bind(&artifact.id)
    .bind(owner_kind_str(&artifact.owner.kind))
    .bind(&artifact.owner.id)
    .bind(&artifact.run_id)
    .bind(&artifact.conversation_id)
    .bind(&artifact.lane_id)
    .bind(artifact_kind_str(&artifact.kind))
    .bind(&artifact.relative_path)
    .bind(&artifact.created_at)
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn list_runs(pool: &SqlitePool) -> Result<Vec<RunSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, conversation_id, lane_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at
        FROM runs
        ORDER BY created_at DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_runs_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<RunSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, conversation_id, lane_id, owner_kind, owner_id, trigger, goal, stage, created_at, updated_at
        FROM runs
        WHERE conversation_id = ?
        ORDER BY created_at DESC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, run_id, conversation_id, lane_id, task_kind, title, assigned_agent_id, status, created_at, updated_at
        FROM tasks
        ORDER BY created_at DESC
        "#,
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
        r#"
        SELECT id, run_id, conversation_id, lane_id, task_kind, title, assigned_agent_id, status, created_at, updated_at
        FROM tasks
        WHERE run_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_artifacts(pool: &SqlitePool) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, owner_kind, owner_id, run_id, conversation_id, lane_id, artifact_kind, relative_path, created_at
        FROM artifacts
        ORDER BY created_at DESC
        "#,
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
        r#"
        SELECT id, owner_kind, owner_id, run_id, conversation_id, lane_id, artifact_kind, relative_path, created_at
        FROM artifacts
        WHERE run_id = ?
        ORDER BY created_at ASC
        "#,
    )
    .bind(run_id)
    .fetch_all(pool)
    .await?;

    Ok(rows.into_iter().map(map_artifact).collect())
}

pub async fn count_rows(pool: &SqlitePool, table: CountTable) -> Result<i64, sqlx::Error> {
    let table_name = match table {
        CountTable::Conversations => "conversations",
        CountTable::Messages => "messages",
        CountTable::Runs => "runs",
        CountTable::Tasks => "tasks",
        CountTable::Artifacts => "artifacts",
        CountTable::Memories => "memories",
        CountTable::Jobs => "jobs",
        CountTable::Decisions => "decisions",
    };
    let sql = format!("SELECT COUNT(*) as count FROM {table_name}");
    let count = sqlx::query_scalar::<_, i64>(&sql).fetch_one(pool).await?;
    Ok(count)
}

pub async fn list_jobs(pool: &SqlitePool) -> Result<Vec<JobRow>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT id, owner_kind, owner_id, job_kind, schedule_kind, schedule_value, status, next_run_at, created_at
        FROM jobs
        ORDER BY created_at DESC
        "#,
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

pub async fn list_recent_logs(
    pool: &SqlitePool,
    limit: u32,
) -> Result<Vec<LogRecordRow>, sqlx::Error> {
    let limit = i64::from(limit.max(1));

    let stage_rows = sqlx::query(
        r#"
        SELECT id, run_id, from_stage, to_stage, policy_rule_id, reason, at
        FROM run_stage_events
        ORDER BY at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let decision_rows = sqlx::query(
        r#"
        SELECT id, run_id, task_id, stage, next_action, policy_rule_id, at
        FROM decisions
        ORDER BY at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let gate_rows = sqlx::query(
        r#"
        SELECT id, run_id, task_id, gate_name, verdict, reason, at
        FROM gate_verdicts
        ORDER BY at DESC
        LIMIT ?
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let mut logs = Vec::new();
    logs.extend(stage_rows.into_iter().map(|row| {
        let to_stage: String = row.get("to_stage");
        let from_stage: Option<String> = row.get("from_stage");
        let reason: Option<String> = row.get("reason");
        let policy_rule_id: Option<String> = row.get("policy_rule_id");

        LogRecordRow {
            id: row.get("id"),
            kind: "stage".to_string(),
            level: "info".to_string(),
            title: format!(
                "{} → {}",
                from_stage.unwrap_or_else(|| "start".to_string()),
                to_stage
            ),
            summary: match (reason, policy_rule_id) {
                (Some(reason), Some(rule)) if !reason.is_empty() => {
                    format!("{reason} · rule {rule}")
                }
                (Some(reason), _) if !reason.is_empty() => reason,
                (_, Some(rule)) => format!("rule {rule}"),
                _ => "stage transition".to_string(),
            },
            run_id: row.get("run_id"),
            task_id: None,
            at: row.get("at"),
        }
    }));

    logs.extend(decision_rows.into_iter().map(|row| LogRecordRow {
        id: row.get("id"),
        kind: "decision".to_string(),
        level: "info".to_string(),
        title: format!("decision @ {}", row.get::<String, _>("stage")),
        summary: format!(
            "{} · rule {}",
            row.get::<String, _>("next_action"),
            row.get::<String, _>("policy_rule_id")
        ),
        run_id: row.get("run_id"),
        task_id: row.get("task_id"),
        at: row.get("at"),
    }));

    logs.extend(gate_rows.into_iter().map(|row| {
        let verdict: String = row.get("verdict");
        let level = match verdict.as_str() {
            "deny" | "failed" => "error",
            "warn" | "pending" => "warn",
            _ => "info",
        };
        let reason: Option<String> = row.get("reason");
        LogRecordRow {
            id: row.get("id"),
            kind: "gate".to_string(),
            level: level.to_string(),
            title: format!("gate {}", row.get::<String, _>("gate_name")),
            summary: match reason {
                Some(reason) if !reason.is_empty() => format!("{verdict} · {reason}"),
                _ => verdict,
            },
            run_id: row.get("run_id"),
            task_id: row.get("task_id"),
            at: row.get("at"),
        }
    }));

    logs.sort_by(|left, right| right.at.cmp(&left.at));
    logs.truncate(limit as usize);
    Ok(logs)
}

pub async fn delete_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let exists = sqlx::query("SELECT id FROM conversations WHERE id = ? LIMIT 1")
        .bind(conversation_id)
        .fetch_optional(&mut *transaction)
        .await?
        .is_some();

    if !exists {
        transaction.rollback().await?;
        return Ok(false);
    }

    let lane_ids = sqlx::query("SELECT id FROM lanes WHERE conversation_id = ?")
        .bind(conversation_id)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    let run_ids = sqlx::query("SELECT id FROM runs WHERE conversation_id = ?")
        .bind(conversation_id)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    let task_ids = sqlx::query("SELECT id FROM tasks WHERE conversation_id = ?")
        .bind(conversation_id)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    for lane_id in &lane_ids {
        sqlx::query("DELETE FROM handoffs WHERE from_lane_id = ? OR to_lane_id = ?")
            .bind(lane_id)
            .bind(lane_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM lane_members WHERE lane_id = ?")
            .bind(lane_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM artifacts WHERE lane_id = ?")
            .bind(lane_id)
            .execute(&mut *transaction)
            .await?;
    }

    for run_id in &run_ids {
        sqlx::query("DELETE FROM run_stage_events WHERE run_id = ?")
            .bind(run_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM decisions WHERE run_id = ?")
            .bind(run_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM gate_verdicts WHERE run_id = ?")
            .bind(run_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM artifacts WHERE run_id = ?")
            .bind(run_id)
            .execute(&mut *transaction)
            .await?;
    }

    for task_id in &task_ids {
        sqlx::query("DELETE FROM decisions WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *transaction)
            .await?;
        sqlx::query("DELETE FROM gate_verdicts WHERE task_id = ?")
            .bind(task_id)
            .execute(&mut *transaction)
            .await?;
    }

    sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM tasks WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM runs WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM artifacts WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM lanes WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("DELETE FROM conversations WHERE id = ?")
        .bind(conversation_id)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(true)
}

async fn persist_extension(
    pool: &SqlitePool,
    manifest: &ExtensionManifest,
    install_dir: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        INSERT INTO extensions (
          id, kind, version, install_dir, frontend_bundle, backend_entry,
          pages_json, panels_json, commands_json, themes_json, locales_json, hooks_json, providers_json
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
          kind = excluded.kind,
          version = excluded.version,
          install_dir = excluded.install_dir,
          frontend_bundle = excluded.frontend_bundle,
          backend_entry = excluded.backend_entry,
          pages_json = excluded.pages_json,
          panels_json = excluded.panels_json,
          commands_json = excluded.commands_json,
          themes_json = excluded.themes_json,
          locales_json = excluded.locales_json,
          hooks_json = excluded.hooks_json,
          providers_json = excluded.providers_json
        "#,
    )
    .bind(&manifest.id)
    .bind(format!("{:?}", manifest.kind))
    .bind(&manifest.version)
    .bind(install_dir)
    .bind(&manifest.frontend_bundle)
    .bind(&manifest.backend_entry)
    .bind(serde_json::to_string(&manifest.contributes.pages).unwrap_or_else(|_| "[]".to_string()))
    .bind(
        serde_json::to_string(&manifest.contributes.panels).unwrap_or_else(|_| "[]".to_string()),
    )
    .bind(
        serde_json::to_string(&manifest.contributes.commands)
            .unwrap_or_else(|_| "[]".to_string()),
    )
    .bind(
        serde_json::to_string(&manifest.contributes.themes).unwrap_or_else(|_| "[]".to_string()),
    )
    .bind(
        serde_json::to_string(&manifest.contributes.locales).unwrap_or_else(|_| "[]".to_string()),
    )
    .bind(serde_json::to_string(&manifest.contributes.hooks).unwrap_or_else(|_| "[]".to_string()))
    .bind(
        serde_json::to_string(&manifest.contributes.providers)
            .unwrap_or_else(|_| "[]".to_string()),
    )
    .execute(pool)
    .await?;
    Ok(())
}

async fn map_conversation(
    pool: &SqlitePool,
    row: sqlx::sqlite::SqliteRow,
) -> Result<ConversationSpec, sqlx::Error> {
    Ok(ConversationSpec {
        id: row.get("id"),
        topology: conversation_topology_from_str(&row.get::<String, _>("topology")),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        space_id: row.get("space_id"),
        title: row.get("title"),
        participants: fetch_conversation_participants(pool, &row.get::<String, _>("id")).await?,
        default_lane_id: row.get("default_lane_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

async fn map_lane(
    pool: &SqlitePool,
    row: sqlx::sqlite::SqliteRow,
) -> Result<LaneSpec, sqlx::Error> {
    Ok(LaneSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        space_id: row.get("space_id"),
        name: row.get("name"),
        lane_type: row.get("lane_type"),
        status: row.get("status"),
        goal: row.get("goal"),
        participants: fetch_lane_members(pool, &row.get::<String, _>("id")).await?,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn map_workspace_profile(row: sqlx::sqlite::SqliteRow) -> WorkspaceProfile {
    WorkspaceProfile {
        id: row.get("id"),
        display_name: row.get("display_name"),
        locale: row.get("locale"),
        time_zone: row.get("time_zone"),
        default_space_id: row.get("default_space_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_message(row: sqlx::sqlite::SqliteRow) -> MessageSpec {
    MessageSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
        sender: row.get("sender"),
        role: message_role_from_str(&row.get::<String, _>("role")),
        body: row.get("body"),
        mentions: serde_json::from_str(&row.get::<String, _>("mentions_json")).unwrap_or_default(),
        created_at: row.get("created_at"),
    }
}

fn map_handoff(row: sqlx::sqlite::SqliteRow) -> HandoffSpec {
    HandoffSpec {
        id: row.get("id"),
        from_lane_id: row.get("from_lane_id"),
        to_lane_id: row.get("to_lane_id"),
        from_agent_id: row.get("from_agent_id"),
        to_agent_id: row.get("to_agent_id"),
        summary: row.get("summary"),
        instructions: row.get("instructions"),
        status: row.get("status"),
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
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
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
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
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
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
        kind: artifact_kind_from_str(&row.get::<String, _>("artifact_kind")),
        relative_path: row.get("relative_path"),
        created_at: row.get("created_at"),
    }
}

fn map_ui_preference_row(row: &sqlx::sqlite::SqliteRow) -> UiPreference {
    UiPreference {
        locale: row.get("locale"),
        theme_id: row.get("theme_id"),
        time_zone: row.get("time_zone"),
        date_style: row.get("date_style"),
        density: row.get("density"),
        motion: row.get("motion"),
        version: row.get::<i64, _>("version") as u64,
        updated_at: row.get("updated_at"),
    }
}

async fn fetch_conversation_participants(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT participant_id
        FROM conversation_participants
        WHERE conversation_id = ?
        ORDER BY position ASC
        "#,
    )
    .bind(conversation_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get("participant_id"))
        .collect())
}

async fn fetch_lane_members(pool: &SqlitePool, lane_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(
        r#"
        SELECT participant_id
        FROM lane_members
        WHERE lane_id = ?
        ORDER BY position ASC
        "#,
    )
    .bind(lane_id)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get("participant_id"))
        .collect())
}

async fn replace_conversation_participants(
    pool: &SqlitePool,
    conversation_id: &str,
    participants: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ?")
        .bind(conversation_id)
        .execute(pool)
        .await?;

    for (index, participant_id) in participants.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO conversation_participants (
              conversation_id, participant_id, participant_kind, position
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(conversation_id)
        .bind(participant_id)
        .bind(participant_kind_for_id(participant_id))
        .bind(index as i64)
        .execute(pool)
        .await?;
    }

    Ok(())
}

async fn replace_lane_members(
    pool: &SqlitePool,
    lane_id: &str,
    participants: &[String],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM lane_members WHERE lane_id = ?")
        .bind(lane_id)
        .execute(pool)
        .await?;

    for (index, participant_id) in participants.iter().enumerate() {
        sqlx::query(
            r#"
            INSERT INTO lane_members (
              lane_id, participant_id, participant_kind, position
            ) VALUES (?, ?, ?, ?)
            "#,
        )
        .bind(lane_id)
        .bind(participant_id)
        .bind(participant_kind_for_id(participant_id))
        .bind(index as i64)
        .execute(pool)
        .await?;
    }

    Ok(())
}

fn participant_kind_for_id(participant_id: &str) -> &'static str {
    if participant_id == "operator" {
        "operator"
    } else {
        "agent"
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

fn conversation_topology_str(value: &ConversationTopology) -> &'static str {
    match value {
        ConversationTopology::Direct => "direct",
        ConversationTopology::Group => "group",
    }
}

fn conversation_topology_from_str(value: &str) -> ConversationTopology {
    match value {
        "group" => ConversationTopology::Group,
        _ => ConversationTopology::Direct,
    }
}

fn message_role_str(role: &MessageRole) -> &'static str {
    match role {
        MessageRole::Operator => "operator",
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
        _ => MessageRole::Operator,
    }
}

fn task_kind_str(kind: &TaskKind) -> &'static str {
    match kind {
        TaskKind::Response => "response",
        TaskKind::Collaboration => "collaboration",
        TaskKind::Maintenance => "maintenance",
        TaskKind::Workflow => "workflow",
    }
}

fn task_kind_from_str(value: &str) -> TaskKind {
    match value {
        "collaboration" => TaskKind::Collaboration,
        "maintenance" => TaskKind::Maintenance,
        "workflow" => TaskKind::Workflow,
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
        ArtifactKind::Summary => "summary",
        ArtifactKind::Handoff => "handoff",
    }
}

fn artifact_kind_from_str(value: &str) -> ArtifactKind {
    match value {
        "screenshot" => ArtifactKind::Screenshot,
        "har" => ArtifactKind::Har,
        "export" => ArtifactKind::Export,
        "log" => ArtifactKind::Log,
        "summary" => ArtifactKind::Summary,
        "handoff" => ArtifactKind::Handoff,
        _ => ArtifactKind::Report,
    }
}

fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339()
}
