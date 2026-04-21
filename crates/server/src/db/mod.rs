mod migration;

pub use migration::initialize_schema;
mod jobs;
mod logs;

pub use jobs::{delete_job, get_job, list_jobs, run_job_now, set_job_status, update_job};
pub use logs::{insert_frontend_log, list_recent_logs};

use ennoia_extension_host::{ExtensionRuntimeSnapshot, ResolvedExtensionSnapshot};
use ennoia_kernel::{
    AgentConfig, ArtifactKind, ArtifactSpec, ConversationSpec, ConversationTopology, HandoffSpec,
    LaneSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef, RunSpec, RunStage, SpaceSpec,
    TaskKind, TaskSpec, TaskStatus, UiPreference, WorkspaceProfile,
};
use sea_query::{Asterisk, Expr, Func, Iden, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use serde::Serialize;
use sqlx::{Row, SqlitePool};

const INSTANCE_PREFERENCE_ID: &str = "instance";
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
    Description,
    PrimaryGoal,
    MentionPolicy,
    DefaultAgentsJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum WorkspaceProfileTbl {
    #[iden = "workspace_profile"]
    Table,
    Id,
    DisplayName,
    Locale,
    TimeZone,
    DefaultSpaceId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum InstanceUiPreferences {
    #[iden = "instance_ui_preferences"]
    Table,
    Id,
    Locale,
    ThemeId,
    TimeZone,
    DateStyle,
    Density,
    Motion,
    Version,
    UpdatedAt,
}

#[derive(Iden)]
enum SpaceUiPreferences {
    #[iden = "space_ui_preferences"]
    Table,
    SpaceId,
    Locale,
    ThemeId,
    TimeZone,
    DateStyle,
    Density,
    Motion,
    Version,
    UpdatedAt,
}

#[derive(Iden)]
enum Conversations {
    Table,
    Id,
    Topology,
    OwnerKind,
    OwnerId,
    SpaceId,
    Title,
    DefaultLaneId,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ConversationParticipants {
    #[iden = "conversation_participants"]
    Table,
    ConversationId,
    ParticipantId,
    ParticipantKind,
    Position,
}

#[derive(Iden)]
enum Messages {
    Table,
    Id,
    ConversationId,
    LaneId,
    Sender,
    Role,
    Body,
    MentionsJson,
    CreatedAt,
}

#[derive(Iden)]
enum Lanes {
    Table,
    Id,
    ConversationId,
    SpaceId,
    Name,
    LaneType,
    Status,
    Goal,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum LaneMembers {
    #[iden = "lane_members"]
    Table,
    LaneId,
    ParticipantId,
    ParticipantKind,
    Position,
}

#[derive(Iden)]
enum Handoffs {
    Table,
    Id,
    FromLaneId,
    ToLaneId,
    FromAgentId,
    ToAgentId,
    Summary,
    Instructions,
    Status,
    CreatedAt,
}

#[derive(Iden)]
enum Runs {
    Table,
    Id,
    ConversationId,
    LaneId,
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
    ConversationId,
    LaneId,
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
    ConversationId,
    LaneId,
    ArtifactKind,
    RelativePath,
    CreatedAt,
}

#[derive(Iden)]
enum RunStageEvents {
    #[iden = "run_stage_events"]
    Table,
    Id,
    RunId,
    FromStage,
    ToStage,
    PolicyRuleId,
    Reason,
    At,
}

#[derive(Iden)]
enum Decisions {
    Table,
    Id,
    RunId,
    TaskId,
    Stage,
    NextAction,
    PolicyRuleId,
    At,
}

#[derive(Iden)]
enum GateVerdicts {
    #[iden = "gate_verdicts"]
    Table,
    Id,
    RunId,
    TaskId,
    GateName,
    Verdict,
    Reason,
    At,
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
    PayloadJson,
    Status,
    RetryCount,
    MaxRetries,
    LastRunAt,
    NextRunAt,
    Error,
    CreatedAt,
    UpdatedAt,
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
enum Memories {
    Table,
}

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
pub struct JobDetailRow {
    pub id: String,
    pub owner_kind: String,
    pub owner_id: String,
    pub job_kind: String,
    pub schedule_kind: String,
    pub schedule_value: String,
    pub payload_json: String,
    pub status: String,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LogRecordRow {
    pub id: String,
    pub kind: String,
    pub source: String,
    pub level: String,
    pub title: String,
    pub summary: String,
    pub details: Option<String>,
    pub run_id: Option<String>,
    pub task_id: Option<String>,
    pub at: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct UiPreferenceRow {
    pub subject_id: String,
    pub preference: UiPreference,
}

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
            serde_json::to_string(&space.default_agents).unwrap_or_else(|_| "[]".to_string());
        let (sql, values) = Query::insert()
            .into_table(Spaces::Table)
            .columns([
                Spaces::Id,
                Spaces::DisplayName,
                Spaces::Description,
                Spaces::PrimaryGoal,
                Spaces::MentionPolicy,
                Spaces::DefaultAgentsJson,
                Spaces::CreatedAt,
                Spaces::UpdatedAt,
            ])
            .values_panic([
                space.id.clone().into(),
                space.display_name.clone().into(),
                space.description.clone().into(),
                space.primary_goal.clone().into(),
                space.mention_policy.clone().into(),
                default_agents.into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(Spaces::Id)
                    .update_columns([
                        Spaces::DisplayName,
                        Spaces::Description,
                        Spaces::PrimaryGoal,
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

pub async fn upsert_extensions_runtime(
    pool: &SqlitePool,
    snapshot: &ExtensionRuntimeSnapshot,
) -> Result<(), sqlx::Error> {
    for item in &snapshot.extensions {
        persist_extension(pool, item).await?;
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
    let (sql, values) = Query::select()
        .columns([
            WorkspaceProfileTbl::Id,
            WorkspaceProfileTbl::DisplayName,
            WorkspaceProfileTbl::Locale,
            WorkspaceProfileTbl::TimeZone,
            WorkspaceProfileTbl::DefaultSpaceId,
            WorkspaceProfileTbl::CreatedAt,
            WorkspaceProfileTbl::UpdatedAt,
        ])
        .from(WorkspaceProfileTbl::Table)
        .limit(1)
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

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

    let (sql, values) = Query::insert()
        .into_table(WorkspaceProfileTbl::Table)
        .columns([
            WorkspaceProfileTbl::Id,
            WorkspaceProfileTbl::DisplayName,
            WorkspaceProfileTbl::Locale,
            WorkspaceProfileTbl::TimeZone,
            WorkspaceProfileTbl::DefaultSpaceId,
            WorkspaceProfileTbl::CreatedAt,
            WorkspaceProfileTbl::UpdatedAt,
        ])
        .values_panic([
            profile.id.clone().into(),
            profile.display_name.clone().into(),
            profile.locale.clone().into(),
            profile.time_zone.clone().into(),
            profile.default_space_id.clone().into(),
            created_at.clone().into(),
            updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(WorkspaceProfileTbl::Id)
                .update_columns([
                    WorkspaceProfileTbl::DisplayName,
                    WorkspaceProfileTbl::Locale,
                    WorkspaceProfileTbl::TimeZone,
                    WorkspaceProfileTbl::DefaultSpaceId,
                    WorkspaceProfileTbl::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    Ok(WorkspaceProfile {
        created_at,
        updated_at,
        ..profile.clone()
    })
}

pub async fn get_instance_ui_preference(
    pool: &SqlitePool,
) -> Result<Option<UiPreferenceRow>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            InstanceUiPreferences::Id,
            InstanceUiPreferences::Locale,
            InstanceUiPreferences::ThemeId,
            InstanceUiPreferences::TimeZone,
            InstanceUiPreferences::DateStyle,
            InstanceUiPreferences::Density,
            InstanceUiPreferences::Motion,
            InstanceUiPreferences::Version,
            InstanceUiPreferences::UpdatedAt,
        ])
        .from(InstanceUiPreferences::Table)
        .and_where(Expr::col(InstanceUiPreferences::Id).eq(INSTANCE_PREFERENCE_ID))
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

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

    let (sql, values) = Query::insert()
        .into_table(InstanceUiPreferences::Table)
        .columns([
            InstanceUiPreferences::Id,
            InstanceUiPreferences::Locale,
            InstanceUiPreferences::ThemeId,
            InstanceUiPreferences::TimeZone,
            InstanceUiPreferences::DateStyle,
            InstanceUiPreferences::Density,
            InstanceUiPreferences::Motion,
            InstanceUiPreferences::Version,
            InstanceUiPreferences::UpdatedAt,
        ])
        .values_panic([
            INSTANCE_PREFERENCE_ID.to_string().into(),
            preference.locale.clone().into(),
            preference.theme_id.clone().into(),
            preference.time_zone.clone().into(),
            preference.date_style.clone().into(),
            preference.density.clone().into(),
            preference.motion.clone().into(),
            (preference.version as i64).into(),
            updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(InstanceUiPreferences::Id)
                .update_columns([
                    InstanceUiPreferences::Locale,
                    InstanceUiPreferences::ThemeId,
                    InstanceUiPreferences::TimeZone,
                    InstanceUiPreferences::DateStyle,
                    InstanceUiPreferences::Density,
                    InstanceUiPreferences::Motion,
                    InstanceUiPreferences::Version,
                    InstanceUiPreferences::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

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
    let (sql, values) = Query::select()
        .columns([
            SpaceUiPreferences::SpaceId,
            SpaceUiPreferences::Locale,
            SpaceUiPreferences::ThemeId,
            SpaceUiPreferences::TimeZone,
            SpaceUiPreferences::DateStyle,
            SpaceUiPreferences::Density,
            SpaceUiPreferences::Motion,
            SpaceUiPreferences::Version,
            SpaceUiPreferences::UpdatedAt,
        ])
        .from(SpaceUiPreferences::Table)
        .and_where(Expr::col(SpaceUiPreferences::SpaceId).eq(space_id))
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

    Ok(row.map(|row| UiPreferenceRow {
        subject_id: row.get("space_id"),
        preference: map_ui_preference_row(&row),
    }))
}

pub async fn list_space_ui_preferences(
    pool: &SqlitePool,
) -> Result<Vec<UiPreferenceRow>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            SpaceUiPreferences::SpaceId,
            SpaceUiPreferences::Locale,
            SpaceUiPreferences::ThemeId,
            SpaceUiPreferences::TimeZone,
            SpaceUiPreferences::DateStyle,
            SpaceUiPreferences::Density,
            SpaceUiPreferences::Motion,
            SpaceUiPreferences::Version,
            SpaceUiPreferences::UpdatedAt,
        ])
        .from(SpaceUiPreferences::Table)
        .order_by(SpaceUiPreferences::SpaceId, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

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

    let (sql, values) = Query::insert()
        .into_table(SpaceUiPreferences::Table)
        .columns([
            SpaceUiPreferences::SpaceId,
            SpaceUiPreferences::Locale,
            SpaceUiPreferences::ThemeId,
            SpaceUiPreferences::TimeZone,
            SpaceUiPreferences::DateStyle,
            SpaceUiPreferences::Density,
            SpaceUiPreferences::Motion,
            SpaceUiPreferences::Version,
            SpaceUiPreferences::UpdatedAt,
        ])
        .values_panic([
            space_id.to_string().into(),
            preference.locale.clone().into(),
            preference.theme_id.clone().into(),
            preference.time_zone.clone().into(),
            preference.date_style.clone().into(),
            preference.density.clone().into(),
            preference.motion.clone().into(),
            (preference.version as i64).into(),
            updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(SpaceUiPreferences::SpaceId)
                .update_columns([
                    SpaceUiPreferences::Locale,
                    SpaceUiPreferences::ThemeId,
                    SpaceUiPreferences::TimeZone,
                    SpaceUiPreferences::DateStyle,
                    SpaceUiPreferences::Density,
                    SpaceUiPreferences::Motion,
                    SpaceUiPreferences::Version,
                    SpaceUiPreferences::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    Ok(UiPreferenceRow {
        subject_id: space_id.to_string(),
        preference: UiPreference {
            updated_at,
            ..preference.clone()
        },
    })
}

pub async fn max_ui_preference_version(pool: &SqlitePool) -> Result<u64, sqlx::Error> {
    let (sql, values) = Query::select()
        .expr(Func::max(Expr::col(InstanceUiPreferences::Version)))
        .from(InstanceUiPreferences::Table)
        .build_sqlx(SqliteQueryBuilder);
    let instance_max = sqlx::query_scalar_with::<_, Option<i64>, _>(&sql, values)
        .fetch_one(pool)
        .await?
        .unwrap_or(0);

    let (sql, values) = Query::select()
        .expr(Func::max(Expr::col(SpaceUiPreferences::Version)))
        .from(SpaceUiPreferences::Table)
        .build_sqlx(SqliteQueryBuilder);
    let space_max = sqlx::query_scalar_with::<_, Option<i64>, _>(&sql, values)
        .fetch_one(pool)
        .await?
        .unwrap_or(0);
    Ok(instance_max.max(space_max) as u64)
}

pub async fn list_conversations(pool: &SqlitePool) -> Result<Vec<ConversationSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Conversations::Id,
            Conversations::Topology,
            Conversations::OwnerKind,
            Conversations::OwnerId,
            Conversations::SpaceId,
            Conversations::Title,
            Conversations::DefaultLaneId,
            Conversations::CreatedAt,
            Conversations::UpdatedAt,
        ])
        .from(Conversations::Table)
        .order_by(Conversations::UpdatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

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
    let (sql, values) = Query::select()
        .columns([
            Conversations::Id,
            Conversations::Topology,
            Conversations::OwnerKind,
            Conversations::OwnerId,
            Conversations::SpaceId,
            Conversations::Title,
            Conversations::DefaultLaneId,
            Conversations::CreatedAt,
            Conversations::UpdatedAt,
        ])
        .from(Conversations::Table)
        .and_where(Expr::col(Conversations::Id).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

    match row {
        Some(row) => Ok(Some(map_conversation(pool, row).await?)),
        None => Ok(None),
    }
}

pub async fn upsert_conversation(
    pool: &SqlitePool,
    conversation: &ConversationSpec,
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Conversations::Table)
        .columns([
            Conversations::Id,
            Conversations::Topology,
            Conversations::OwnerKind,
            Conversations::OwnerId,
            Conversations::SpaceId,
            Conversations::Title,
            Conversations::DefaultLaneId,
            Conversations::CreatedAt,
            Conversations::UpdatedAt,
        ])
        .values_panic([
            conversation.id.clone().into(),
            conversation_topology_str(&conversation.topology).into(),
            owner_kind_str(&conversation.owner.kind).into(),
            conversation.owner.id.clone().into(),
            conversation.space_id.clone().into(),
            conversation.title.clone().into(),
            conversation.default_lane_id.clone().into(),
            conversation.created_at.clone().into(),
            conversation.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Conversations::Id)
                .update_columns([
                    Conversations::Topology,
                    Conversations::OwnerKind,
                    Conversations::OwnerId,
                    Conversations::SpaceId,
                    Conversations::Title,
                    Conversations::DefaultLaneId,
                    Conversations::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    replace_conversation_participants(pool, &conversation.id, &conversation.participants).await?;
    Ok(())
}

pub async fn list_messages_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<MessageSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Messages::Id,
            Messages::ConversationId,
            Messages::LaneId,
            Messages::Sender,
            Messages::Role,
            Messages::Body,
            Messages::MentionsJson,
            Messages::CreatedAt,
        ])
        .from(Messages::Table)
        .and_where(Expr::col(Messages::ConversationId).eq(conversation_id))
        .order_by(Messages::CreatedAt, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    Ok(rows.into_iter().map(map_message).collect())
}

pub async fn insert_message(pool: &SqlitePool, message: &MessageSpec) -> Result<(), sqlx::Error> {
    let mentions = serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string());
    let (sql, values) = Query::insert()
        .into_table(Messages::Table)
        .columns([
            Messages::Id,
            Messages::ConversationId,
            Messages::LaneId,
            Messages::Sender,
            Messages::Role,
            Messages::Body,
            Messages::MentionsJson,
            Messages::CreatedAt,
        ])
        .values_panic([
            message.id.clone().into(),
            message.conversation_id.clone().into(),
            message.lane_id.clone().into(),
            message.sender.clone().into(),
            message_role_str(&message.role).into(),
            message.body.clone().into(),
            mentions.into(),
            message.created_at.clone().into(),
        ])
        .on_conflict(OnConflict::column(Messages::Id).do_nothing().to_owned())
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn list_lanes_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<LaneSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Lanes::Id,
            Lanes::ConversationId,
            Lanes::SpaceId,
            Lanes::Name,
            Lanes::LaneType,
            Lanes::Status,
            Lanes::Goal,
            Lanes::CreatedAt,
            Lanes::UpdatedAt,
        ])
        .from(Lanes::Table)
        .and_where(Expr::col(Lanes::ConversationId).eq(conversation_id))
        .order_by(Lanes::UpdatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        items.push(map_lane(pool, row).await?);
    }
    Ok(items)
}

pub async fn get_lane(pool: &SqlitePool, lane_id: &str) -> Result<Option<LaneSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Lanes::Id,
            Lanes::ConversationId,
            Lanes::SpaceId,
            Lanes::Name,
            Lanes::LaneType,
            Lanes::Status,
            Lanes::Goal,
            Lanes::CreatedAt,
            Lanes::UpdatedAt,
        ])
        .from(Lanes::Table)
        .and_where(Expr::col(Lanes::Id).eq(lane_id))
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;

    match row {
        Some(row) => Ok(Some(map_lane(pool, row).await?)),
        None => Ok(None),
    }
}

pub async fn insert_lane(pool: &SqlitePool, lane: &LaneSpec) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Lanes::Table)
        .columns([
            Lanes::Id,
            Lanes::ConversationId,
            Lanes::SpaceId,
            Lanes::Name,
            Lanes::LaneType,
            Lanes::Status,
            Lanes::Goal,
            Lanes::CreatedAt,
            Lanes::UpdatedAt,
        ])
        .values_panic([
            lane.id.clone().into(),
            lane.conversation_id.clone().into(),
            lane.space_id.clone().into(),
            lane.name.clone().into(),
            lane.lane_type.clone().into(),
            lane.status.clone().into(),
            lane.goal.clone().into(),
            lane.created_at.clone().into(),
            lane.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Lanes::Id)
                .update_columns([
                    Lanes::ConversationId,
                    Lanes::SpaceId,
                    Lanes::Name,
                    Lanes::LaneType,
                    Lanes::Status,
                    Lanes::Goal,
                    Lanes::UpdatedAt,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;

    replace_lane_members(pool, &lane.id, &lane.participants).await?;
    Ok(())
}

pub async fn list_handoffs_for_lane(
    pool: &SqlitePool,
    lane_id: &str,
) -> Result<Vec<HandoffSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Handoffs::Id,
            Handoffs::FromLaneId,
            Handoffs::ToLaneId,
            Handoffs::FromAgentId,
            Handoffs::ToAgentId,
            Handoffs::Summary,
            Handoffs::Instructions,
            Handoffs::Status,
            Handoffs::CreatedAt,
        ])
        .from(Handoffs::Table)
        .and_where(
            Expr::col(Handoffs::FromLaneId)
                .eq(lane_id)
                .or(Expr::col(Handoffs::ToLaneId).eq(lane_id)),
        )
        .order_by(Handoffs::CreatedAt, sea_query::Order::Desc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    Ok(rows.into_iter().map(map_handoff).collect())
}

pub async fn insert_handoff(pool: &SqlitePool, handoff: &HandoffSpec) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Handoffs::Table)
        .columns([
            Handoffs::Id,
            Handoffs::FromLaneId,
            Handoffs::ToLaneId,
            Handoffs::FromAgentId,
            Handoffs::ToAgentId,
            Handoffs::Summary,
            Handoffs::Instructions,
            Handoffs::Status,
            Handoffs::CreatedAt,
        ])
        .values_panic([
            handoff.id.clone().into(),
            handoff.from_lane_id.clone().into(),
            handoff.to_lane_id.clone().into(),
            handoff.from_agent_id.clone().into(),
            handoff.to_agent_id.clone().into(),
            handoff.summary.clone().into(),
            handoff.instructions.clone().into(),
            handoff.status.clone().into(),
            handoff.created_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Handoffs::Id)
                .update_columns([
                    Handoffs::FromLaneId,
                    Handoffs::ToLaneId,
                    Handoffs::FromAgentId,
                    Handoffs::ToAgentId,
                    Handoffs::Summary,
                    Handoffs::Instructions,
                    Handoffs::Status,
                ])
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn upsert_run(pool: &SqlitePool, run: &RunSpec) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(Runs::Table)
        .columns([
            Runs::Id,
            Runs::ConversationId,
            Runs::LaneId,
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
            run.conversation_id.clone().into(),
            run.lane_id.clone().into(),
            owner_kind_str(&run.owner.kind).into(),
            run.owner.id.clone().into(),
            run.trigger.clone().into(),
            run.goal.clone().into(),
            run.stage.as_str().into(),
            run.created_at.clone().into(),
            run.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Runs::Id)
                .update_columns([
                    Runs::ConversationId,
                    Runs::LaneId,
                    Runs::OwnerKind,
                    Runs::OwnerId,
                    Runs::Trigger,
                    Runs::Goal,
                    Runs::Stage,
                    Runs::UpdatedAt,
                ])
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
            Tasks::ConversationId,
            Tasks::LaneId,
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
            task.conversation_id.clone().into(),
            task.lane_id.clone().into(),
            task_kind_str(&task.task_kind).into(),
            task.title.clone().into(),
            task.assigned_agent_id.clone().into(),
            task_status_str(&task.status).into(),
            task.created_at.clone().into(),
            task.updated_at.clone().into(),
        ])
        .on_conflict(
            OnConflict::column(Tasks::Id)
                .update_columns([
                    Tasks::RunId,
                    Tasks::ConversationId,
                    Tasks::LaneId,
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
            Artifacts::ConversationId,
            Artifacts::LaneId,
            Artifacts::ArtifactKind,
            Artifacts::RelativePath,
            Artifacts::CreatedAt,
        ])
        .values_panic([
            artifact.id.clone().into(),
            owner_kind_str(&artifact.owner.kind).into(),
            artifact.owner.id.clone().into(),
            artifact.run_id.clone().into(),
            artifact.conversation_id.clone().into(),
            artifact.lane_id.clone().into(),
            artifact_kind_str(&artifact.kind).into(),
            artifact.relative_path.clone().into(),
            artifact.created_at.clone().into(),
        ])
        .on_conflict(OnConflict::column(Artifacts::Id).do_nothing().to_owned())
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn list_runs(pool: &SqlitePool) -> Result<Vec<RunSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Runs::Id,
            Runs::ConversationId,
            Runs::LaneId,
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

pub async fn list_runs_for_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<RunSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Runs::Id,
            Runs::ConversationId,
            Runs::LaneId,
            Runs::OwnerKind,
            Runs::OwnerId,
            Runs::Trigger,
            Runs::Goal,
            Runs::Stage,
            Runs::CreatedAt,
            Runs::UpdatedAt,
        ])
        .from(Runs::Table)
        .and_where(Expr::col(Runs::ConversationId).eq(conversation_id))
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
            Tasks::ConversationId,
            Tasks::LaneId,
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
            Tasks::ConversationId,
            Tasks::LaneId,
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

pub async fn list_artifacts(pool: &SqlitePool) -> Result<Vec<ArtifactSpec>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
            Artifacts::Id,
            Artifacts::OwnerKind,
            Artifacts::OwnerId,
            Artifacts::RunId,
            Artifacts::ConversationId,
            Artifacts::LaneId,
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
            Artifacts::ConversationId,
            Artifacts::LaneId,
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
    let count_expr = Func::count(Expr::col(Asterisk));
    let (sql, values) = match table {
        CountTable::Conversations => Query::select()
            .expr(count_expr)
            .from(Conversations::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Messages => Query::select()
            .expr(count_expr)
            .from(Messages::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Runs => Query::select()
            .expr(count_expr)
            .from(Runs::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Tasks => Query::select()
            .expr(count_expr)
            .from(Tasks::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Artifacts => Query::select()
            .expr(count_expr)
            .from(Artifacts::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Memories => Query::select()
            .expr(count_expr)
            .from(Memories::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Jobs => Query::select()
            .expr(count_expr)
            .from(Jobs::Table)
            .build_sqlx(SqliteQueryBuilder),
        CountTable::Decisions => Query::select()
            .expr(count_expr)
            .from(Decisions::Table)
            .build_sqlx(SqliteQueryBuilder),
    };
    let count = sqlx::query_scalar_with::<_, i64, _>(&sql, values)
        .fetch_one(pool)
        .await?;
    Ok(count)
}

pub async fn delete_conversation(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<bool, sqlx::Error> {
    let mut transaction = pool.begin().await?;
    let (sql, values) = Query::select()
        .column(Conversations::Id)
        .from(Conversations::Table)
        .and_where(Expr::col(Conversations::Id).eq(conversation_id))
        .limit(1)
        .build_sqlx(SqliteQueryBuilder);
    let exists = sqlx::query_with(&sql, values)
        .fetch_optional(&mut *transaction)
        .await?
        .is_some();

    if !exists {
        transaction.rollback().await?;
        return Ok(false);
    }

    let (sql, values) = Query::select()
        .column(Lanes::Id)
        .from(Lanes::Table)
        .and_where(Expr::col(Lanes::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    let lane_ids = sqlx::query_with(&sql, values)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    let (sql, values) = Query::select()
        .column(Runs::Id)
        .from(Runs::Table)
        .and_where(Expr::col(Runs::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    let run_ids = sqlx::query_with(&sql, values)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    let (sql, values) = Query::select()
        .column(Tasks::Id)
        .from(Tasks::Table)
        .and_where(Expr::col(Tasks::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    let task_ids = sqlx::query_with(&sql, values)
        .fetch_all(&mut *transaction)
        .await?
        .into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect::<Vec<_>>();

    for lane_id in &lane_ids {
        let (sql, values) = Query::delete()
            .from_table(Handoffs::Table)
            .and_where(
                Expr::col(Handoffs::FromLaneId)
                    .eq(lane_id)
                    .or(Expr::col(Handoffs::ToLaneId).eq(lane_id)),
            )
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(LaneMembers::Table)
            .and_where(Expr::col(LaneMembers::LaneId).eq(lane_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(Artifacts::Table)
            .and_where(Expr::col(Artifacts::LaneId).eq(lane_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;
    }

    for run_id in &run_ids {
        let (sql, values) = Query::delete()
            .from_table(RunStageEvents::Table)
            .and_where(Expr::col(RunStageEvents::RunId).eq(run_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(Decisions::Table)
            .and_where(Expr::col(Decisions::RunId).eq(run_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(GateVerdicts::Table)
            .and_where(Expr::col(GateVerdicts::RunId).eq(run_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(Artifacts::Table)
            .and_where(Expr::col(Artifacts::RunId).eq(run_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;
    }

    for task_id in &task_ids {
        let (sql, values) = Query::delete()
            .from_table(Decisions::Table)
            .and_where(Expr::col(Decisions::TaskId).eq(task_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;

        let (sql, values) = Query::delete()
            .from_table(GateVerdicts::Table)
            .and_where(Expr::col(GateVerdicts::TaskId).eq(task_id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&mut *transaction)
            .await?;
    }

    let (sql, values) = Query::delete()
        .from_table(Messages::Table)
        .and_where(Expr::col(Messages::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(Tasks::Table)
        .and_where(Expr::col(Tasks::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(Runs::Table)
        .and_where(Expr::col(Runs::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(Artifacts::Table)
        .and_where(Expr::col(Artifacts::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(Lanes::Table)
        .and_where(Expr::col(Lanes::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(ConversationParticipants::Table)
        .and_where(Expr::col(ConversationParticipants::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    let (sql, values) = Query::delete()
        .from_table(Conversations::Table)
        .and_where(Expr::col(Conversations::Id).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values)
        .execute(&mut *transaction)
        .await?;

    transaction.commit().await?;
    Ok(true)
}

async fn persist_extension(
    pool: &SqlitePool,
    extension: &ResolvedExtensionSnapshot,
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
            extension.id.clone().into(),
            format!("{:?}", extension.kind).into(),
            extension.version.clone().into(),
            extension.install_dir.clone().into(),
            extension
                .frontend
                .as_ref()
                .map(|item| item.entry.clone())
                .into(),
            extension
                .backend
                .as_ref()
                .map(|item| item.entry.clone())
                .into(),
            serde_json::to_string(&extension.pages)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.panels)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.commands)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.themes)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.locales)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.hooks)
                .unwrap_or_else(|_| "[]".to_string())
                .into(),
            serde_json::to_string(&extension.providers)
                .unwrap_or_else(|_| "[]".to_string())
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

fn map_job_detail(row: sqlx::sqlite::SqliteRow) -> JobDetailRow {
    JobDetailRow {
        id: row.get("id"),
        owner_kind: row.get("owner_kind"),
        owner_id: row.get("owner_id"),
        job_kind: row.get("job_kind"),
        schedule_kind: row.get("schedule_kind"),
        schedule_value: row.get("schedule_value"),
        payload_json: row.get("payload_json"),
        status: row.get("status"),
        retry_count: row.get::<i64, _>("retry_count") as u32,
        max_retries: row.get::<i64, _>("max_retries") as u32,
        last_run_at: row.get("last_run_at"),
        next_run_at: row.get("next_run_at"),
        error: row.get("error"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

async fn fetch_conversation_participants(
    pool: &SqlitePool,
    conversation_id: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let (sql, values) = Query::select()
        .column(ConversationParticipants::ParticipantId)
        .from(ConversationParticipants::Table)
        .and_where(Expr::col(ConversationParticipants::ConversationId).eq(conversation_id))
        .order_by(ConversationParticipants::Position, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    Ok(rows
        .into_iter()
        .map(|row| row.get("participant_id"))
        .collect())
}

async fn fetch_lane_members(pool: &SqlitePool, lane_id: &str) -> Result<Vec<String>, sqlx::Error> {
    let (sql, values) = Query::select()
        .column(LaneMembers::ParticipantId)
        .from(LaneMembers::Table)
        .and_where(Expr::col(LaneMembers::LaneId).eq(lane_id))
        .order_by(LaneMembers::Position, sea_query::Order::Asc)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
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
    let (sql, values) = Query::delete()
        .from_table(ConversationParticipants::Table)
        .and_where(Expr::col(ConversationParticipants::ConversationId).eq(conversation_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;

    for (index, participant_id) in participants.iter().enumerate() {
        let (sql, values) = Query::insert()
            .into_table(ConversationParticipants::Table)
            .columns([
                ConversationParticipants::ConversationId,
                ConversationParticipants::ParticipantId,
                ConversationParticipants::ParticipantKind,
                ConversationParticipants::Position,
            ])
            .values_panic([
                conversation_id.to_string().into(),
                participant_id.clone().into(),
                participant_kind_for_id(participant_id).into(),
                (index as i64).into(),
            ])
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values).execute(pool).await?;
    }

    Ok(())
}

async fn replace_lane_members(
    pool: &SqlitePool,
    lane_id: &str,
    participants: &[String],
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::delete()
        .from_table(LaneMembers::Table)
        .and_where(Expr::col(LaneMembers::LaneId).eq(lane_id))
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;

    for (index, participant_id) in participants.iter().enumerate() {
        let (sql, values) = Query::insert()
            .into_table(LaneMembers::Table)
            .columns([
                LaneMembers::LaneId,
                LaneMembers::ParticipantId,
                LaneMembers::ParticipantKind,
                LaneMembers::Position,
            ])
            .values_panic([
                lane_id.to_string().into(),
                participant_id.clone().into(),
                participant_kind_for_id(participant_id).into(),
                (index as i64).into(),
            ])
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values).execute(pool).await?;
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
