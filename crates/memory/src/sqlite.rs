use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{
    AssembleRequest, ContextFrame, ContextLayer, ContextView, EpisodeKind, EpisodeRecord,
    EpisodeRequest, MemoryError, MemoryKind, MemoryPolicy, MemoryRecord, MemorySource,
    MemoryStatus, MemoryStore, OwnerKind, OwnerRef, RecallMode, RecallQuery, RecallResult,
    RememberReceipt, RememberRequest, ReviewAction, ReviewActionKind, ReviewReceipt, Stability,
};
use sea_query::{Expr, Iden, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Iden)]
enum Episodes {
    Table,
    Id,
    OwnerKind,
    OwnerId,
    Namespace,
    ThreadId,
    RunId,
    EpisodeKind,
    Role,
    Content,
    ContentType,
    SourceUri,
    EntitiesJson,
    TagsJson,
    Importance,
    OccurredAt,
    IngestedAt,
}

#[derive(Iden)]
enum Memories {
    Table,
    Id,
    OwnerKind,
    OwnerId,
    Namespace,
    MemoryKind,
    Stability,
    Status,
    SupersededBy,
    Title,
    Content,
    Summary,
    Confidence,
    Importance,
    ValidFrom,
    ValidTo,
    SourcesJson,
    TagsJson,
    EntitiesJson,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ContextFrames {
    #[iden = "context_frames"]
    Table,
    Id,
    OwnerKind,
    OwnerId,
    Namespace,
    Layer,
    FrameKind,
    Content,
    SourceMemoryIdsJson,
    BudgetChars,
    TtlSeconds,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum RememberReceipts {
    #[iden = "remember_receipts"]
    Table,
    Id,
    OwnerKind,
    OwnerId,
    TargetMemoryId,
    Action,
    PolicyRuleId,
    DetailsJson,
    CreatedAt,
}

#[derive(Iden)]
enum RecallReceipts {
    #[iden = "recall_receipts"]
    Table,
    Id,
    OwnerKind,
    OwnerId,
    ThreadId,
    RunId,
    QueryText,
    Mode,
    MemoryIdsJson,
    Chars,
    DetailsJson,
    CreatedAt,
}

#[derive(Iden)]
enum ReviewReceipts {
    #[iden = "review_receipts"]
    Table,
    Id,
    OwnerKind,
    OwnerId,
    TargetMemoryId,
    Action,
    OldStatus,
    NewStatus,
    Reviewer,
    DetailsJson,
    CreatedAt,
}

/// SqliteMemoryStore is the canonical MemoryStore backed by sqlx + sqlite.
#[derive(Debug, Clone)]
pub struct SqliteMemoryStore {
    pool: SqlitePool,
    policy: Arc<MemoryPolicy>,
}

impl SqliteMemoryStore {
    pub fn new(pool: SqlitePool, policy: Arc<MemoryPolicy>) -> Self {
        Self { pool, policy }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    pub fn policy(&self) -> &MemoryPolicy {
        &self.policy
    }

    fn validate_remember(&self, req: &RememberRequest) -> Result<(), MemoryError> {
        if self.policy.is_forbidden(&req.namespace) {
            return Err(MemoryError::Policy(format!(
                "namespace {} is forbidden",
                req.namespace
            )));
        }

        if req.stability == Stability::LongTerm {
            if !self.policy.is_truth_namespace(&req.namespace) {
                return Err(MemoryError::Policy(format!(
                    "long_term memories may not be written to non-truth namespace {}",
                    req.namespace
                )));
            }
            if self.policy.require_sources_for_long_term && req.sources.is_empty() {
                return Err(MemoryError::Policy(
                    "long_term memories require at least one source".to_string(),
                ));
            }
        }

        if req.content.trim().is_empty() {
            return Err(MemoryError::Invalid(
                "content must be non-empty".to_string(),
            ));
        }

        Ok(())
    }
}

trait IntoMemoryError<T> {
    fn mem_backend(self) -> Result<T, MemoryError>;
    fn mem_serde(self) -> Result<T, MemoryError>;
}

impl<T> IntoMemoryError<T> for Result<T, sqlx::Error> {
    fn mem_backend(self) -> Result<T, MemoryError> {
        self.map_err(|e| MemoryError::Backend(e.to_string()))
    }
    fn mem_serde(self) -> Result<T, MemoryError> {
        self.map_err(|e| MemoryError::Backend(e.to_string()))
    }
}

impl<T> IntoMemoryError<T> for Result<T, serde_json::Error> {
    fn mem_backend(self) -> Result<T, MemoryError> {
        self.map_err(|e| MemoryError::Serde(e.to_string()))
    }
    fn mem_serde(self) -> Result<T, MemoryError> {
        self.map_err(|e| MemoryError::Serde(e.to_string()))
    }
}

fn memory_columns_all() -> Vec<Memories> {
    vec![
        Memories::Id,
        Memories::OwnerKind,
        Memories::OwnerId,
        Memories::Namespace,
        Memories::MemoryKind,
        Memories::Stability,
        Memories::Status,
        Memories::SupersededBy,
        Memories::Title,
        Memories::Content,
        Memories::Summary,
        Memories::Confidence,
        Memories::Importance,
        Memories::ValidFrom,
        Memories::ValidTo,
        Memories::SourcesJson,
        Memories::TagsJson,
        Memories::EntitiesJson,
        Memories::CreatedAt,
        Memories::UpdatedAt,
    ]
}

fn episode_columns_all() -> Vec<Episodes> {
    vec![
        Episodes::Id,
        Episodes::OwnerKind,
        Episodes::OwnerId,
        Episodes::Namespace,
        Episodes::ThreadId,
        Episodes::RunId,
        Episodes::EpisodeKind,
        Episodes::Role,
        Episodes::Content,
        Episodes::ContentType,
        Episodes::SourceUri,
        Episodes::EntitiesJson,
        Episodes::TagsJson,
        Episodes::Importance,
        Episodes::OccurredAt,
        Episodes::IngestedAt,
    ]
}

#[async_trait]
impl MemoryStore for SqliteMemoryStore {
    async fn record_episode(&self, req: EpisodeRequest) -> Result<EpisodeRecord, MemoryError> {
        let now = now_iso();
        let occurred = req.occurred_at.clone().unwrap_or_else(now_iso);
        let record = EpisodeRecord {
            id: format!("epi-{}", Uuid::new_v4()),
            owner: req.owner.clone(),
            namespace: req.namespace.clone(),
            thread_id: req.thread_id.clone(),
            run_id: req.run_id.clone(),
            episode_kind: req.episode_kind,
            role: req.role.clone(),
            content: req.content.clone(),
            content_type: req
                .content_type
                .clone()
                .unwrap_or_else(|| "text/plain".to_string()),
            source_uri: req.source_uri.clone(),
            entities: req.entities.clone(),
            tags: req.tags.clone(),
            importance: req.importance.unwrap_or(0.2),
            occurred_at: occurred.clone(),
            ingested_at: now,
        };

        let entities_json = serde_json::to_string(&record.entities).mem_serde()?;
        let tags_json = serde_json::to_string(&record.tags).mem_serde()?;

        let (sql, values) = Query::insert()
            .into_table(Episodes::Table)
            .columns(episode_columns_all())
            .values_panic([
                record.id.clone().into(),
                owner_kind_str(&record.owner.kind).to_string().into(),
                record.owner.id.clone().into(),
                record.namespace.clone().into(),
                record.thread_id.clone().into(),
                record.run_id.clone().into(),
                record.episode_kind.as_str().to_string().into(),
                record.role.clone().into(),
                record.content.clone().into(),
                record.content_type.clone().into(),
                record.source_uri.clone().into(),
                entities_json.into(),
                tags_json.into(),
                (record.importance as f64).into(),
                record.occurred_at.clone().into(),
                record.ingested_at.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        Ok(record)
    }

    async fn remember(&self, req: RememberRequest) -> Result<RememberReceipt, MemoryError> {
        self.validate_remember(&req)?;

        let now = now_iso();
        let memory_id = format!("mem-{}", Uuid::new_v4());
        let status = match req.stability {
            Stability::LongTerm => MemoryStatus::Active,
            Stability::Working => MemoryStatus::Active,
        };

        let sources_json = serde_json::to_string(&req.sources).mem_serde()?;
        let tags_json = serde_json::to_string(&req.tags).mem_serde()?;
        let entities_json = serde_json::to_string(&req.entities).mem_serde()?;

        let (sql, values) = Query::insert()
            .into_table(Memories::Table)
            .columns(memory_columns_all())
            .values_panic([
                memory_id.clone().into(),
                owner_kind_str(&req.owner.kind).to_string().into(),
                req.owner.id.clone().into(),
                req.namespace.clone().into(),
                req.memory_kind.as_str().to_string().into(),
                req.stability.as_str().to_string().into(),
                status.as_str().to_string().into(),
                Option::<String>::None.into(),
                req.title.clone().into(),
                req.content.clone().into(),
                req.summary.clone().into(),
                (req.confidence.unwrap_or(0.6) as f64).into(),
                (req.importance.unwrap_or(0.5) as f64).into(),
                req.valid_from.clone().into(),
                req.valid_to.clone().into(),
                sources_json.into(),
                tags_json.into(),
                entities_json.into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        let receipt_id = format!("rec-rem-{}", Uuid::new_v4());
        let (sql, values) = Query::insert()
            .into_table(RememberReceipts::Table)
            .columns([
                RememberReceipts::Id,
                RememberReceipts::OwnerKind,
                RememberReceipts::OwnerId,
                RememberReceipts::TargetMemoryId,
                RememberReceipts::Action,
                RememberReceipts::PolicyRuleId,
                RememberReceipts::DetailsJson,
                RememberReceipts::CreatedAt,
            ])
            .values_panic([
                receipt_id.clone().into(),
                owner_kind_str(&req.owner.kind).to_string().into(),
                req.owner.id.clone().into(),
                memory_id.clone().into(),
                "create".to_string().into(),
                Option::<String>::None.into(),
                "{}".to_string().into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        Ok(RememberReceipt {
            receipt_id,
            memory_id,
            action: "create".to_string(),
            policy_rule_id: None,
            created_at: now,
        })
    }

    async fn recall(&self, query: RecallQuery) -> Result<RecallResult, MemoryError> {
        let memories = match query.mode {
            RecallMode::Namespace | RecallMode::Hybrid => {
                recall_by_namespace(&self.pool, &query).await?
            }
            RecallMode::Fts => recall_by_fts(&self.pool, &query).await?,
        };

        let memory_ids: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
        let chars: u32 = memories
            .iter()
            .map(|m| m.content.chars().count() as u32)
            .sum();

        let receipt_id = format!("rec-rcl-{}", Uuid::new_v4());
        let now = now_iso();
        let memory_ids_json = serde_json::to_string(&memory_ids).mem_serde()?;

        let (sql, values) = Query::insert()
            .into_table(RecallReceipts::Table)
            .columns([
                RecallReceipts::Id,
                RecallReceipts::OwnerKind,
                RecallReceipts::OwnerId,
                RecallReceipts::ThreadId,
                RecallReceipts::RunId,
                RecallReceipts::QueryText,
                RecallReceipts::Mode,
                RecallReceipts::MemoryIdsJson,
                RecallReceipts::Chars,
                RecallReceipts::DetailsJson,
                RecallReceipts::CreatedAt,
            ])
            .values_panic([
                receipt_id.clone().into(),
                owner_kind_str(&query.owner.kind).to_string().into(),
                query.owner.id.clone().into(),
                query.thread_id.clone().into(),
                query.run_id.clone().into(),
                query.query_text.clone().into(),
                query.mode.as_str().to_string().into(),
                memory_ids_json.into(),
                (chars as i64).into(),
                "{}".to_string().into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        Ok(RecallResult {
            memories,
            receipt_id,
            mode: query.mode.as_str().to_string(),
            total_chars: chars,
        })
    }

    async fn review(&self, action: ReviewAction) -> Result<ReviewReceipt, MemoryError> {
        let (sql, values) = Query::select()
            .columns([Memories::OwnerKind, Memories::OwnerId, Memories::Status])
            .from(Memories::Table)
            .and_where(Expr::col(Memories::Id).eq(action.target_memory_id.clone()))
            .build_sqlx(SqliteQueryBuilder);

        let existing = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .mem_backend()?;

        let row = existing.ok_or_else(|| MemoryError::NotFound(action.target_memory_id.clone()))?;
        let owner_kind: String = row.get("owner_kind");
        let owner_id: String = row.get("owner_id");
        let old_status: String = row.get("status");

        let new_status = match action.action {
            ReviewActionKind::Approve => MemoryStatus::Active,
            ReviewActionKind::Reject => MemoryStatus::Retired,
            ReviewActionKind::Supersede => MemoryStatus::Superseded,
            ReviewActionKind::Retire => MemoryStatus::Retired,
        };

        let now = now_iso();
        let (sql, values) = Query::update()
            .table(Memories::Table)
            .values([
                (Memories::Status, new_status.as_str().to_string().into()),
                (Memories::UpdatedAt, now.clone().into()),
            ])
            .and_where(Expr::col(Memories::Id).eq(action.target_memory_id.clone()))
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        let receipt_id = format!("rec-rev-{}", Uuid::new_v4());
        let notes_json = serde_json::to_string(&action.notes).mem_serde()?;
        let (sql, values) = Query::insert()
            .into_table(ReviewReceipts::Table)
            .columns([
                ReviewReceipts::Id,
                ReviewReceipts::OwnerKind,
                ReviewReceipts::OwnerId,
                ReviewReceipts::TargetMemoryId,
                ReviewReceipts::Action,
                ReviewReceipts::OldStatus,
                ReviewReceipts::NewStatus,
                ReviewReceipts::Reviewer,
                ReviewReceipts::DetailsJson,
                ReviewReceipts::CreatedAt,
            ])
            .values_panic([
                receipt_id.clone().into(),
                owner_kind.clone().into(),
                owner_id.clone().into(),
                action.target_memory_id.clone().into(),
                action.action.as_str().to_string().into(),
                old_status.clone().into(),
                new_status.as_str().to_string().into(),
                action.reviewer.clone().into(),
                notes_json.into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        Ok(ReviewReceipt {
            receipt_id,
            target_memory_id: action.target_memory_id,
            action: action.action.as_str().to_string(),
            old_status: Some(old_status),
            new_status: new_status.as_str().to_string(),
            reviewer: action.reviewer,
            created_at: now,
        })
    }

    async fn assemble_context(&self, req: AssembleRequest) -> Result<ContextView, MemoryError> {
        let budget = req
            .budget_chars
            .unwrap_or(self.policy.assemble_budget_chars);
        let mut view = ContextView::default();
        view.recent_messages = req.recent_messages;
        view.active_tasks = req.active_tasks;

        let (sql, values) = Query::select()
            .columns([ContextFrames::Layer, ContextFrames::Content])
            .from(ContextFrames::Table)
            .and_where(
                Expr::col(ContextFrames::OwnerKind).eq(owner_kind_str(&req.owner.kind).to_string()),
            )
            .and_where(Expr::col(ContextFrames::OwnerId).eq(req.owner.id.clone()))
            .order_by(ContextFrames::UpdatedAt, sea_query::Order::Desc)
            .build_sqlx(SqliteQueryBuilder);

        let frames = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .mem_backend()?;

        for row in frames {
            if view.total_chars >= budget {
                break;
            }
            let layer = ContextLayer::from_str(&row.get::<String, _>("layer"));
            let content: String = row.get("content");
            view.push(layer, content);
        }

        let (sql, values) = Query::select()
            .columns([
                Memories::Id,
                Memories::Namespace,
                Memories::MemoryKind,
                Memories::Stability,
                Memories::Status,
                Memories::Content,
                Memories::Summary,
                Memories::Title,
            ])
            .from(Memories::Table)
            .and_where(
                Expr::col(Memories::OwnerKind).eq(owner_kind_str(&req.owner.kind).to_string()),
            )
            .and_where(Expr::col(Memories::OwnerId).eq(req.owner.id.clone()))
            .and_where(Expr::col(Memories::Status).eq("active"))
            .order_by(Memories::UpdatedAt, sea_query::Order::Desc)
            .limit(20)
            .build_sqlx(SqliteQueryBuilder);

        let memories = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .mem_backend()?;

        for row in memories {
            if view.total_chars >= budget {
                break;
            }
            let id: String = row.get("id");
            let title: Option<String> = row.get("title");
            let content: String = row.get("content");
            let summary: Option<String> = row.get("summary");
            let memory_kind = MemoryKind::from_str(&row.get::<String, _>("memory_kind"));
            let snippet = summary
                .filter(|s| !s.trim().is_empty())
                .or_else(|| title.clone())
                .unwrap_or_else(|| content.clone());
            view.recalled_memory_ids.push(id);
            let layer = layer_for_memory_kind(memory_kind);
            view.push(layer, snippet);
        }

        Ok(view)
    }

    async fn upsert_frame(&self, frame: ContextFrame) -> Result<(), MemoryError> {
        let now = now_iso();
        let created_at = if frame.created_at.is_empty() {
            now.clone()
        } else {
            frame.created_at.clone()
        };
        let source_ids_json = serde_json::to_string(&frame.source_memory_ids).mem_serde()?;

        let (sql, values) = Query::insert()
            .into_table(ContextFrames::Table)
            .columns([
                ContextFrames::Id,
                ContextFrames::OwnerKind,
                ContextFrames::OwnerId,
                ContextFrames::Namespace,
                ContextFrames::Layer,
                ContextFrames::FrameKind,
                ContextFrames::Content,
                ContextFrames::SourceMemoryIdsJson,
                ContextFrames::BudgetChars,
                ContextFrames::TtlSeconds,
                ContextFrames::CreatedAt,
                ContextFrames::UpdatedAt,
            ])
            .values_panic([
                frame.id.clone().into(),
                owner_kind_str(&frame.owner.kind).to_string().into(),
                frame.owner.id.clone().into(),
                frame.namespace.clone().into(),
                frame.layer.as_str().to_string().into(),
                frame.frame_kind.clone().into(),
                frame.content.clone().into(),
                source_ids_json.into(),
                frame.budget_chars.map(|v| v as i64).into(),
                frame.ttl_seconds.map(|v| v as i64).into(),
                created_at.into(),
                now.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(ContextFrames::Id)
                    .update_columns([
                        ContextFrames::Namespace,
                        ContextFrames::Layer,
                        ContextFrames::FrameKind,
                        ContextFrames::Content,
                        ContextFrames::SourceMemoryIdsJson,
                        ContextFrames::BudgetChars,
                        ContextFrames::TtlSeconds,
                        ContextFrames::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .mem_backend()?;

        Ok(())
    }

    async fn list_memories(&self, limit: u32) -> Result<Vec<MemoryRecord>, MemoryError> {
        let (sql, values) = Query::select()
            .columns(memory_columns_all())
            .from(Memories::Table)
            .order_by(Memories::UpdatedAt, sea_query::Order::Desc)
            .limit(limit as u64)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .mem_backend()?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn list_episodes_for_owner(
        &self,
        owner: &OwnerRef,
        limit: u32,
    ) -> Result<Vec<EpisodeRecord>, MemoryError> {
        let (sql, values) = Query::select()
            .columns(episode_columns_all())
            .from(Episodes::Table)
            .and_where(Expr::col(Episodes::OwnerKind).eq(owner_kind_str(&owner.kind).to_string()))
            .and_where(Expr::col(Episodes::OwnerId).eq(owner.id.clone()))
            .order_by(Episodes::IngestedAt, sea_query::Order::Desc)
            .limit(limit as u64)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .mem_backend()?;

        rows.into_iter().map(row_to_episode).collect()
    }
}

async fn recall_by_namespace(
    pool: &SqlitePool,
    query: &RecallQuery,
) -> Result<Vec<MemoryRecord>, MemoryError> {
    let namespace_pattern = query
        .namespace_prefix
        .clone()
        .map(|p| format!("{p}%"))
        .unwrap_or_else(|| "%".to_string());

    let mut builder = Query::select();
    builder
        .columns(memory_columns_all())
        .from(Memories::Table)
        .and_where(Expr::col(Memories::OwnerKind).eq(owner_kind_str(&query.owner.kind).to_string()))
        .and_where(Expr::col(Memories::OwnerId).eq(query.owner.id.clone()))
        .and_where(Expr::col(Memories::Status).eq("active"))
        .and_where(Expr::col(Memories::Namespace).like(namespace_pattern))
        .order_by(Memories::UpdatedAt, sea_query::Order::Desc)
        .limit(query.limit as u64);

    if let Some(kind) = query.memory_kind {
        builder.and_where(Expr::col(Memories::MemoryKind).eq(kind.as_str().to_string()));
    }

    let (sql, values) = builder.build_sqlx(SqliteQueryBuilder);
    let rows = sqlx::query_with(&sql, values)
        .fetch_all(pool)
        .await
        .mem_backend()?;

    rows.into_iter().map(row_to_memory).collect()
}

/// FTS5 MATCH is not supported by sea-query's builder; keep this query raw.
async fn recall_by_fts(
    pool: &SqlitePool,
    query: &RecallQuery,
) -> Result<Vec<MemoryRecord>, MemoryError> {
    let Some(query_text) = query.query_text.as_ref() else {
        return recall_by_namespace(pool, query).await;
    };

    let rows = sqlx::query(
        "SELECT m.id, m.owner_kind, m.owner_id, m.namespace, m.memory_kind, m.stability, m.status, m.superseded_by, \
         m.title, m.content, m.summary, m.confidence, m.importance, m.valid_from, m.valid_to, \
         m.sources_json, m.tags_json, m.entities_json, m.created_at, m.updated_at \
         FROM memories_fts \
         JOIN memories m ON m.rowid = memories_fts.rowid \
         WHERE memories_fts MATCH ? \
         AND m.owner_kind = ? AND m.owner_id = ? AND m.status = 'active' \
         ORDER BY rank LIMIT ?",
    )
    .bind(query_text)
    .bind(owner_kind_str(&query.owner.kind))
    .bind(&query.owner.id)
    .bind(query.limit as i64)
    .fetch_all(pool)
    .await
    .mem_backend()?;

    rows.into_iter().map(row_to_memory).collect()
}

fn row_to_memory(row: sqlx::sqlite::SqliteRow) -> Result<MemoryRecord, MemoryError> {
    let sources_json: String = row.get("sources_json");
    let tags_json: String = row.get("tags_json");
    let entities_json: String = row.get("entities_json");

    Ok(MemoryRecord {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        namespace: row.get("namespace"),
        memory_kind: MemoryKind::from_str(&row.get::<String, _>("memory_kind")),
        stability: Stability::from_str(&row.get::<String, _>("stability")),
        status: MemoryStatus::from_str(&row.get::<String, _>("status")),
        superseded_by: row.get("superseded_by"),
        title: row.get("title"),
        content: row.get("content"),
        summary: row.get("summary"),
        confidence: row.get::<f64, _>("confidence") as f32,
        importance: row.get::<f64, _>("importance") as f32,
        valid_from: row.get("valid_from"),
        valid_to: row.get("valid_to"),
        sources: parse_sources(&sources_json)?,
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        entities: serde_json::from_str(&entities_json).unwrap_or_default(),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

fn row_to_episode(row: sqlx::sqlite::SqliteRow) -> Result<EpisodeRecord, MemoryError> {
    let tags_json: String = row.get("tags_json");
    let entities_json: String = row.get("entities_json");

    Ok(EpisodeRecord {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        namespace: row.get("namespace"),
        thread_id: row.get("thread_id"),
        run_id: row.get("run_id"),
        episode_kind: EpisodeKind::from_str(&row.get::<String, _>("episode_kind")),
        role: row.get("role"),
        content: row.get("content"),
        content_type: row.get("content_type"),
        source_uri: row.get("source_uri"),
        entities: serde_json::from_str(&entities_json).unwrap_or_default(),
        tags: serde_json::from_str(&tags_json).unwrap_or_default(),
        importance: row.get::<f64, _>("importance") as f32,
        occurred_at: row.get("occurred_at"),
        ingested_at: row.get("ingested_at"),
    })
}

fn parse_sources(raw: &str) -> Result<Vec<MemorySource>, MemoryError> {
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }
    serde_json::from_str(raw).map_err(|e| MemoryError::Serde(e.to_string()))
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

fn layer_for_memory_kind(kind: MemoryKind) -> ContextLayer {
    match kind {
        MemoryKind::Preference => ContextLayer::Preferences,
        MemoryKind::Procedure => ContextLayer::Execution,
        MemoryKind::Observation => ContextLayer::Evidence,
        MemoryKind::Context => ContextLayer::Core,
        MemoryKind::Fact => ContextLayer::Core,
        MemoryKind::DecisionNote => ContextLayer::Constraints,
    }
}

fn now_iso() -> String {
    Utc::now().to_rfc3339()
}
