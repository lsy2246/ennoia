use ennoia_assets::migrations;
use ennoia_extension_host::ExtensionRegistry;
use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ExtensionManifest, MessageRole, MessageSpec,
    OwnerKind, OwnerRef, RunSpec, RunStage, SpaceSpec, TaskKind, TaskSpec, TaskStatus, ThreadKind,
    ThreadSpec, UiPreference,
};
use sea_query::{Alias, Expr, Func, Iden, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use serde::Serialize;
use sqlx::{Row, SqlitePool};

// ========== Iden enums ==========

#[derive(Iden)]
enum Agents {
    Table,
    Id,
    DisplayName,
    Kind,
    WorkspaceMode,
    DefaultModel,
    SkillsDir,
    WorkspaceDir,
    ArtifactsDir,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Spaces {
    Table,
    Id,
    DisplayName,
    MentionPolicy,
    DefaultAgentsJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Threads {
    Table,
    Id,
    ThreadKind,
    OwnerKind,
    OwnerId,
    SpaceId,
    Title,
    ParticipantsJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    ThreadId,
    Sender,
    Role,
    Body,
    MentionsJson,
    CreatedAt,
}

#[derive(Iden)]
enum Runs {
    Table,
    Id,
    ThreadId,
    OwnerKind,
    OwnerId,
    Trigger,
    Goal,
    Stage,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Tasks {
    Table,
    Id,
    RunId,
    TaskKind,
    Title,
    AssignedAgentId,
    Status,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum Artifacts {
    Table,
    Id,
    OwnerKind,
    OwnerId,
    RunId,
    ArtifactKind,
    RelativePath,
    CreatedAt,
}

#[derive(Iden)]
enum Extensions {
    Table,
    Id,
    Kind,
    Version,
    InstallDir,
    FrontendBundle,
    BackendEntry,
    PagesJson,
    PanelsJson,
    CommandsJson,
    ThemesJson,
    LocalesJson,
    HooksJson,
    ProvidersJson,
}

#[derive(Iden)]
enum Jobs {
    Table,
    Id,
    OwnerKind,
    OwnerId,
    JobKind,
    ScheduleKind,
    ScheduleValue,
    Status,
    NextRunAt,
    CreatedAt,
}

#[derive(Iden, Clone, Copy)]
enum Memories {
    Table,
    Id,
}

#[derive(Iden, Clone, Copy)]
enum Decisions {
    Table,
    Id,
}

#[derive(Debug, Clone, Copy)]
pub enum CountTable {
    Threads,
    Messages,
    Runs,
    Tasks,
    Artifacts,
    Memories,
    Jobs,
    Decisions,
}

// ========== Public types ==========

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
pub struct UiPreferenceRow {
    pub subject_id: String,
    pub preference: UiPreference,
}

// ========== Schema migration ==========

pub async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for migration in migrations::all() {
        for statement in migration
            .contents
            .split(';')
            .map(str::trim)
            .filter(|statement| !statement.is_empty())
        {
            if let Err(error) = sqlx::query(statement).execute(pool).await {
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

// ========== Upserts ==========

pub async fn upsert_agents(pool: &SqlitePool, agents: &[AgentConfig]) -> Result<(), sqlx::Error> {
    let now = now_iso();
    for agent in agents {
        let (sql, values) = Query::insert()
            .into_table(Agents::Table)
            .columns([
                Agents::Id,
                Agents::DisplayName,
                Agents::Kind,
                Agents::WorkspaceMode,
                Agents::DefaultModel,
                Agents::SkillsDir,
                Agents::WorkspaceDir,
                Agents::ArtifactsDir,
                Agents::CreatedAt,
                Agents::UpdatedAt,
            ])
            .values_panic([
                agent.id.clone().into(),
                agent.display_name.clone().into(),
                agent.kind.clone().into(),
                agent.workspace_mode.clone().into(),
                agent.default_model.clone().into(),
                agent.skills_dir.clone().into(),
                agent.workspace_dir.clone().into(),
                agent.artifacts_dir.clone().into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(Agents::Id)
                    .update_columns([
                        Agents::DisplayName,
                        Agents::Kind,
                        Agents::WorkspaceMode,
                        Agents::DefaultModel,
                        Agents::SkillsDir,
                        Agents::WorkspaceDir,
                        Agents::ArtifactsDir,
                        Agents::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values).execute(pool).await?;
    }
    Ok(())
}

pub async fn upsert_spaces(pool: &SqlitePool, spaces: &[SpaceSpec]) -> Result<(), sqlx::Error> {
    let now = now_iso();
    for space in spaces {
        let default_agents =
            serde_json::to_string(&space.default_agents).unwrap_or_else(|_| "[]".into());
        let (sql, values) = Query::insert()
            .into_table(Spaces::Table)
            .columns([
                Spaces::Id,
                Spaces::DisplayName,
                Spaces::MentionPolicy,
                Spaces::DefaultAgentsJson,
                Spaces::CreatedAt,
                Spaces::UpdatedAt,
            ])
            .values_panic([
                space.id.clone().into(),
                space.display_name.clone().into(),
                space.mention_policy.clone().into(),
                default_agents.into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(Spaces::Id)
                    .update_columns([
                        Spaces::DisplayName,
                        Spaces::MentionPolicy,
                        Spaces::DefaultAgentsJson,
                        Spaces::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values).execute(pool).await?;
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
    let participants =
        serde_json::to_string(&thread.participants).unwrap_or_else(|_| "[]".to_string());
    let (sql, values) = Query::insert()
        .into_table(Threads::Table)
        .columns([
            Threads::Id,
            Threads::OwnerKind,
            Threads::OwnerId,
            Threads::SpaceId,
            Threads::ThreadKind,
            Threads::Title,
            Threads::ParticipantsJson,
            Threads::CreatedAt,
            Threads::UpdatedAt,
        ])
        .values_panic([
            thread.id.clone().into(),
            owner_kind_str(&thread.owner.kind).to_string().into(),
            thread.owner.id.clone().into(),
            thread.space_id.clone().into(),
            thread_kind_str(&thread.kind).to_string().into(),
            thread.title.clone().into(),
            participants.into(),
            thread.created_at.clone().into(),
            thread.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Threads::Id)
                .update_columns([
                    Threads::OwnerKind,
                    Threads::OwnerId,
                    Threads::SpaceId,
                    Threads::ThreadKind,
                    Threads::Title,
                    Threads::ParticipantsJson,
                    Threads::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn insert_message(pool: &SqlitePool, message: &MessageSpec) -> Result<(), sqlx::Error> {
    let mentions = serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string());
    let (sql, values) = Query::insert()
        .into_table(Messages::Table)
        .columns([
            Messages::Id,
            Messages::ThreadId,
            Messages::Sender,
            Messages::Role,
            Messages::Body,
            Messages::MentionsJson,
            Messages::CreatedAt,
        ])
        .values_panic([
            message.id.clone().into(),
            message.thread_id.clone().into(),
            message.sender.clone().into(),
            message_role_str(&message.role).to_string().into(),
            message.body.clone().into(),
            mentions.into(),
            message.created_at.clone().into(),
        ])
        .on_conflict(OnConflict::column(Messages::Id).do_nothing().to_owned())
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn upsert_run(pool: &SqlitePool, run: &RunSpec) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Runs::Table)
        .columns([
            Runs::Id,
            Runs::ThreadId,
            Runs::OwnerKind,
            Runs::OwnerId,
            Runs::Trigger,
            Runs::Goal,
            Runs::Stage,
            Runs::CreatedAt,
            Runs::UpdatedAt,
        ])
        .values_panic([
            run.id.clone().into(),
            run.thread_id.clone().into(),
            owner_kind_str(&run.owner.kind).to_string().into(),
            run.owner.id.clone().into(),
            run.trigger.clone().into(),
            run.goal.clone().into(),
            run.stage.as_str().to_string().into(),
            run.created_at.clone().into(),
            run.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Runs::Id)
                .update_columns([Runs::Stage, Runs::Goal, Runs::UpdatedAt])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn upsert_task(pool: &SqlitePool, task: &TaskSpec) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Tasks::Table)
        .columns([
            Tasks::Id,
            Tasks::RunId,
            Tasks::TaskKind,
            Tasks::Title,
            Tasks::AssignedAgentId,
            Tasks::Status,
            Tasks::CreatedAt,
            Tasks::UpdatedAt,
        ])
        .values_panic([
            task.id.clone().into(),
            task.run_id.clone().into(),
            task_kind_str(&task.task_kind).to_string().into(),
            task.title.clone().into(),
            task.assigned_agent_id.clone().into(),
            task_status_str(&task.status).to_string().into(),
            task.created_at.clone().into(),
            task.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Tasks::Id)
                .update_columns([
                    Tasks::TaskKind,
                    Tasks::Title,
                    Tasks::AssignedAgentId,
                    Tasks::Status,
                    Tasks::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn insert_artifact(
    pool: &SqlitePool,
    artifact: &ArtifactSpec,
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Artifacts::Table)
        .columns([
            Artifacts::Id,
            Artifacts::OwnerKind,
            Artifacts::OwnerId,
            Artifacts::RunId,
            Artifacts::ArtifactKind,
            Artifacts::RelativePath,
            Artifacts::CreatedAt,
        ])
        .values_panic([
            artifact.id.clone().into(),
            owner_kind_str(&artifact.owner.kind).to_string().into(),
            artifact.owner.id.clone().into(),
            artifact.run_id.clone().into(),
            artifact_kind_str(&artifact.kind).to_string().into(),
            artifact.relative_path.clone().into(),
            artifact.created_at.clone().into(),
        ])
        .on_conflict(OnConflict::column(Artifacts::Id).do_nothing().to_owned())
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

async fn persist_extension(
    pool: &SqlitePool,
    manifest: &ExtensionManifest,
    install_dir: &str,
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Extensions::Table)
        .columns([
            Extensions::Id,
            Extensions::Kind,
            Extensions::Version,
            Extensions::InstallDir,
            Extensions::FrontendBundle,
            Extensions::BackendEntry,
            Extensions::PagesJson,
            Extensions::PanelsJson,
            Extensions::CommandsJson,
            Extensions::ThemesJson,
            Extensions::LocalesJson,
            Extensions::HooksJson,
            Extensions::ProvidersJson,
        ])
        .values_panic([
            manifest.id.clone().into(),
            format!("{:?}", manifest.kind).into(),
            manifest.version.clone().into(),
            install_dir.to_string().into(),
            manifest.frontend_bundle.clone().into(),
            manifest.backend_entry.clone().into(),
            serde_json::to_string(&manifest.contributes.pages)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.panels)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.commands)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.themes)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.locales)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.hooks)
                .unwrap_or_else(|_| "[]".into())
                .into(),
            serde_json::to_string(&manifest.contributes.providers)
                .unwrap_or_else(|_| "[]".into())
                .into(),
        ])
        .on_conflict(
            OnConflict::column(Extensions::Id)
                .update_columns([
                    Extensions::Kind,
                    Extensions::Version,
                    Extensions::InstallDir,
                    Extensions::FrontendBundle,
                    Extensions::BackendEntry,
                    Extensions::PagesJson,
                    Extensions::PanelsJson,
                    Extensions::CommandsJson,
                    Extensions::ThemesJson,
                    Extensions::LocalesJson,
                    Extensions::HooksJson,
                    Extensions::ProvidersJson,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

// ========== Lists ==========

pub async fn list_threads(pool: &SqlitePool) -> Result<Vec<ThreadSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Threads::Id,
            Threads::OwnerKind,
            Threads::OwnerId,
            Threads::SpaceId,
            Threads::ThreadKind,
            Threads::Title,
            Threads::ParticipantsJson,
            Threads::CreatedAt,
            Threads::UpdatedAt,
        ])
        .from(Threads::Table)
        .order_by(Threads::UpdatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_thread).collect())
}

pub async fn list_messages_for_thread(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<MessageSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Messages::Id,
            Messages::ThreadId,
            Messages::Sender,
            Messages::Role,
            Messages::Body,
            Messages::MentionsJson,
            Messages::CreatedAt,
        ])
        .from(Messages::Table)
        .and_where(Expr::col(Messages::ThreadId).eq(thread_id))
        .order_by(Messages::CreatedAt, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_message).collect())
}

pub async fn list_runs(pool: &SqlitePool) -> Result<Vec<RunSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Runs::Id,
            Runs::ThreadId,
            Runs::OwnerKind,
            Runs::OwnerId,
            Runs::Trigger,
            Runs::Goal,
            Runs::Stage,
            Runs::CreatedAt,
            Runs::UpdatedAt,
        ])
        .from(Runs::Table)
        .order_by(Runs::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_runs_for_thread(
    pool: &SqlitePool,
    thread_id: &str,
) -> Result<Vec<RunSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Runs::Id,
            Runs::ThreadId,
            Runs::OwnerKind,
            Runs::OwnerId,
            Runs::Trigger,
            Runs::Goal,
            Runs::Stage,
            Runs::CreatedAt,
            Runs::UpdatedAt,
        ])
        .from(Runs::Table)
        .and_where(Expr::col(Runs::ThreadId).eq(thread_id))
        .order_by(Runs::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_run).collect())
}

pub async fn list_tasks(pool: &SqlitePool) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Tasks::Id,
            Tasks::RunId,
            Tasks::TaskKind,
            Tasks::Title,
            Tasks::AssignedAgentId,
            Tasks::Status,
            Tasks::CreatedAt,
            Tasks::UpdatedAt,
        ])
        .from(Tasks::Table)
        .order_by(Tasks::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_tasks_for_run(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Tasks::Id,
            Tasks::RunId,
            Tasks::TaskKind,
            Tasks::Title,
            Tasks::AssignedAgentId,
            Tasks::Status,
            Tasks::CreatedAt,
            Tasks::UpdatedAt,
        ])
        .from(Tasks::Table)
        .and_where(Expr::col(Tasks::RunId).eq(run_id))
        .order_by(Tasks::CreatedAt, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_active_tasks_for_owner(
    pool: &SqlitePool,
    owner: &OwnerRef,
    limit: usize,
) -> Result<Vec<TaskSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            (Tasks::Table, Tasks::Id),
            (Tasks::Table, Tasks::RunId),
            (Tasks::Table, Tasks::TaskKind),
            (Tasks::Table, Tasks::Title),
            (Tasks::Table, Tasks::AssignedAgentId),
            (Tasks::Table, Tasks::Status),
            (Tasks::Table, Tasks::CreatedAt),
            (Tasks::Table, Tasks::UpdatedAt),
        ])
        .from(Tasks::Table)
        .inner_join(
            Runs::Table,
            Expr::col((Tasks::Table, Tasks::RunId)).equals((Runs::Table, Runs::Id)),
        )
        .and_where(Expr::col((Runs::Table, Runs::OwnerKind)).eq(owner_kind_str(&owner.kind)))
        .and_where(Expr::col((Runs::Table, Runs::OwnerId)).eq(owner.id.clone()))
        .order_by((Tasks::Table, Tasks::CreatedAt), sea_query::Order::Desc)
        .limit(limit as u64)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_task).collect())
}

pub async fn list_artifacts(pool: &SqlitePool) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Artifacts::Id,
            Artifacts::OwnerKind,
            Artifacts::OwnerId,
            Artifacts::RunId,
            Artifacts::ArtifactKind,
            Artifacts::RelativePath,
            Artifacts::CreatedAt,
        ])
        .from(Artifacts::Table)
        .order_by(Artifacts::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_artifact).collect())
}

pub async fn list_artifacts_for_run(
    pool: &SqlitePool,
    run_id: &str,
) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Artifacts::Id,
            Artifacts::OwnerKind,
            Artifacts::OwnerId,
            Artifacts::RunId,
            Artifacts::ArtifactKind,
            Artifacts::RelativePath,
            Artifacts::CreatedAt,
        ])
        .from(Artifacts::Table)
        .and_where(Expr::col(Artifacts::RunId).eq(run_id))
        .order_by(Artifacts::CreatedAt, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows.into_iter().map(map_artifact).collect())
}

pub async fn count_rows(pool: &SqlitePool, table: CountTable) -> Result<i64, sqlx::Error> {
    macro_rules! count_query {
        ($table:expr, $column:expr) => {
            Query::select()
                .expr_as(
                    Func::count(Expr::col(($table, $column))),
                    Alias::new("count"),
                )
                .from($table)
                .build_sqlx(SqliteQueryBuilder)
        };
    }

    let (sql, values) = match table {
        CountTable::Threads => count_query!(Threads::Table, Threads::Id),
        CountTable::Messages => count_query!(Messages::Table, Messages::Id),
        CountTable::Runs => count_query!(Runs::Table, Runs::Id),
        CountTable::Tasks => count_query!(Tasks::Table, Tasks::Id),
        CountTable::Artifacts => count_query!(Artifacts::Table, Artifacts::Id),
        CountTable::Memories => count_query!(Memories::Table, Memories::Id),
        CountTable::Jobs => count_query!(Jobs::Table, Jobs::Id),
        CountTable::Decisions => count_query!(Decisions::Table, Decisions::Id),
    };
    let row = sqlx::query_with(&sql, values).fetch_one(pool).await?;
    Ok(row.get::<i64, _>("count"))
}

pub async fn list_jobs(pool: &SqlitePool) -> Result<Vec<JobRow>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Jobs::Id,
            Jobs::OwnerKind,
            Jobs::OwnerId,
            Jobs::JobKind,
            Jobs::ScheduleKind,
            Jobs::ScheduleValue,
            Jobs::Status,
            Jobs::NextRunAt,
            Jobs::CreatedAt,
        ])
        .from(Jobs::Table)
        .order_by(Jobs::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
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

pub async fn get_user_ui_preference(
    pool: &SqlitePool,
    user_id: &str,
) -> Result<Option<UiPreferenceRow>, sqlx::Error> {
    let row = sqlx::query(
        r#"
        SELECT user_id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        FROM user_ui_preferences
        WHERE user_id = ?
        "#,
    )
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(row.map(|row| UiPreferenceRow {
        subject_id: row.get("user_id"),
        preference: map_ui_preference_row(&row),
    }))
}

pub async fn upsert_user_ui_preference(
    pool: &SqlitePool,
    user_id: &str,
    preference: &UiPreference,
) -> Result<UiPreferenceRow, sqlx::Error> {
    let updated_at = if preference.updated_at.is_empty() {
        now_iso()
    } else {
        preference.updated_at.clone()
    };
    sqlx::query(
        r#"
        INSERT INTO user_ui_preferences (
            user_id, locale, theme_id, time_zone, date_style, density, motion, version, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        ON CONFLICT(user_id) DO UPDATE SET
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
    .bind(user_id)
    .bind(preference.locale.clone())
    .bind(preference.theme_id.clone())
    .bind(preference.time_zone.clone())
    .bind(preference.date_style.clone())
    .bind(preference.density.clone())
    .bind(preference.motion.clone())
    .bind(preference.version as i64)
    .bind(updated_at.clone())
    .execute(pool)
    .await?;

    Ok(UiPreferenceRow {
        subject_id: user_id.to_string(),
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
    .bind(preference.locale.clone())
    .bind(preference.theme_id.clone())
    .bind(preference.time_zone.clone())
    .bind(preference.date_style.clone())
    .bind(preference.density.clone())
    .bind(preference.motion.clone())
    .bind(preference.version as i64)
    .bind(updated_at.clone())
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
    let user_max =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM user_ui_preferences")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);
    let space_max =
        sqlx::query_scalar::<_, Option<i64>>("SELECT MAX(version) FROM space_ui_preferences")
            .fetch_one(pool)
            .await?
            .unwrap_or(0);

    Ok(user_max.max(space_max) as u64)
}

// ========== Row mappers ==========

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
