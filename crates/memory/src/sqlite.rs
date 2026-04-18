use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{
    ContextFrame, ContextLayer, ContextView, EpisodeKind, EpisodeRecord, MemoryKind, MemoryRecord,
    MemorySource, MemoryStatus, OwnerKind, OwnerRef, ReviewAction, ReviewActionKind, Stability,
};
use ennoia_policy::MemoryPolicy;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

use crate::error::MemoryError;
use crate::model::{RememberReceipt, ReviewReceipt};
use crate::requests::{
    AssembleRequest, EpisodeRequest, RecallMode, RecallQuery, RecallResult, RememberRequest,
};
use crate::store::MemoryStore;

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
            return Err(MemoryError::Invalid("content must be non-empty".to_string()));
        }

        Ok(())
    }
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

        sqlx::query(
            "INSERT INTO episodes \
             (id, owner_kind, owner_id, namespace, thread_id, run_id, episode_kind, role, \
              content, content_type, source_uri, entities_json, tags_json, importance, \
              occurred_at, ingested_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(owner_kind_str(&record.owner.kind))
        .bind(&record.owner.id)
        .bind(&record.namespace)
        .bind(&record.thread_id)
        .bind(&record.run_id)
        .bind(record.episode_kind.as_str())
        .bind(&record.role)
        .bind(&record.content)
        .bind(&record.content_type)
        .bind(&record.source_uri)
        .bind(serde_json::to_string(&record.entities)?)
        .bind(serde_json::to_string(&record.tags)?)
        .bind(record.importance as f64)
        .bind(&record.occurred_at)
        .bind(&record.ingested_at)
        .execute(&self.pool)
        .await?;

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

        sqlx::query(
            "INSERT INTO memories \
             (id, owner_kind, owner_id, namespace, memory_kind, stability, status, superseded_by, \
              title, content, summary, confidence, importance, valid_from, valid_to, \
              sources_json, tags_json, entities_json, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&memory_id)
        .bind(owner_kind_str(&req.owner.kind))
        .bind(&req.owner.id)
        .bind(&req.namespace)
        .bind(req.memory_kind.as_str())
        .bind(req.stability.as_str())
        .bind(status.as_str())
        .bind(&req.title)
        .bind(&req.content)
        .bind(&req.summary)
        .bind(req.confidence.unwrap_or(0.6) as f64)
        .bind(req.importance.unwrap_or(0.5) as f64)
        .bind(&req.valid_from)
        .bind(&req.valid_to)
        .bind(serde_json::to_string(&req.sources)?)
        .bind(serde_json::to_string(&req.tags)?)
        .bind(serde_json::to_string(&req.entities)?)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let receipt_id = format!("rec-rem-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO remember_receipts \
             (id, owner_kind, owner_id, target_memory_id, action, policy_rule_id, details_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&receipt_id)
        .bind(owner_kind_str(&req.owner.kind))
        .bind(&req.owner.id)
        .bind(&memory_id)
        .bind("create")
        .bind::<Option<&str>>(None)
        .bind("{}")
        .bind(&now)
        .execute(&self.pool)
        .await?;

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
            RecallMode::Namespace | RecallMode::Hybrid => recall_by_namespace(&self.pool, &query).await?,
            RecallMode::Fts => recall_by_fts(&self.pool, &query).await?,
        };

        let memory_ids: Vec<String> = memories.iter().map(|m| m.id.clone()).collect();
        let chars: u32 = memories
            .iter()
            .map(|m| m.content.chars().count() as u32)
            .sum();

        let receipt_id = format!("rec-rcl-{}", Uuid::new_v4());
        let now = now_iso();

        sqlx::query(
            "INSERT INTO recall_receipts \
             (id, owner_kind, owner_id, thread_id, run_id, query_text, mode, memory_ids_json, chars, details_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&receipt_id)
        .bind(owner_kind_str(&query.owner.kind))
        .bind(&query.owner.id)
        .bind(&query.thread_id)
        .bind(&query.run_id)
        .bind(&query.query_text)
        .bind(query.mode.as_str())
        .bind(serde_json::to_string(&memory_ids)?)
        .bind(chars as i64)
        .bind("{}")
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(RecallResult {
            memories,
            receipt_id,
            mode: query.mode.as_str().to_string(),
            total_chars: chars,
        })
    }

    async fn review(&self, action: ReviewAction) -> Result<ReviewReceipt, MemoryError> {
        let existing = sqlx::query(
            "SELECT owner_kind, owner_id, status FROM memories WHERE id = ?",
        )
        .bind(&action.target_memory_id)
        .fetch_optional(&self.pool)
        .await?;

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
        sqlx::query("UPDATE memories SET status = ?, updated_at = ? WHERE id = ?")
            .bind(new_status.as_str())
            .bind(&now)
            .bind(&action.target_memory_id)
            .execute(&self.pool)
            .await?;

        let receipt_id = format!("rec-rev-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO review_receipts \
             (id, owner_kind, owner_id, target_memory_id, action, old_status, new_status, reviewer, details_json, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&receipt_id)
        .bind(&owner_kind)
        .bind(&owner_id)
        .bind(&action.target_memory_id)
        .bind(action.action.as_str())
        .bind(&old_status)
        .bind(new_status.as_str())
        .bind(&action.reviewer)
        .bind(serde_json::to_string(&action.notes)?)
        .bind(&now)
        .execute(&self.pool)
        .await?;

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

        let frames = sqlx::query(
            "SELECT layer, content FROM context_frames \
             WHERE owner_kind = ? AND owner_id = ? \
             ORDER BY updated_at DESC",
        )
        .bind(owner_kind_str(&req.owner.kind))
        .bind(&req.owner.id)
        .fetch_all(&self.pool)
        .await?;

        for row in frames {
            if view.total_chars >= budget {
                break;
            }
            let layer = ContextLayer::from_str(&row.get::<String, _>("layer"));
            let content: String = row.get("content");
            view.push(layer, content);
        }

        let memories = sqlx::query(
            "SELECT id, namespace, memory_kind, stability, status, content, summary, title \
             FROM memories \
             WHERE owner_kind = ? AND owner_id = ? AND status = 'active' \
             ORDER BY updated_at DESC LIMIT 20",
        )
        .bind(owner_kind_str(&req.owner.kind))
        .bind(&req.owner.id)
        .fetch_all(&self.pool)
        .await?;

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
        sqlx::query(
            "INSERT INTO context_frames \
             (id, owner_kind, owner_id, namespace, layer, frame_kind, content, source_memory_ids_json, budget_chars, ttl_seconds, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(id) DO UPDATE SET \
               namespace = excluded.namespace, \
               layer = excluded.layer, \
               frame_kind = excluded.frame_kind, \
               content = excluded.content, \
               source_memory_ids_json = excluded.source_memory_ids_json, \
               budget_chars = excluded.budget_chars, \
               ttl_seconds = excluded.ttl_seconds, \
               updated_at = excluded.updated_at",
        )
        .bind(&frame.id)
        .bind(owner_kind_str(&frame.owner.kind))
        .bind(&frame.owner.id)
        .bind(&frame.namespace)
        .bind(frame.layer.as_str())
        .bind(&frame.frame_kind)
        .bind(&frame.content)
        .bind(serde_json::to_string(&frame.source_memory_ids)?)
        .bind(frame.budget_chars.map(|v| v as i64))
        .bind(frame.ttl_seconds.map(|v| v as i64))
        .bind(&created_at)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    async fn list_memories(&self, limit: u32) -> Result<Vec<MemoryRecord>, MemoryError> {
        let rows = sqlx::query(
            "SELECT id, owner_kind, owner_id, namespace, memory_kind, stability, status, superseded_by, \
             title, content, summary, confidence, importance, valid_from, valid_to, \
             sources_json, tags_json, entities_json, created_at, updated_at \
             FROM memories ORDER BY updated_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.into_iter().map(row_to_memory).collect()
    }

    async fn list_episodes_for_owner(
        &self,
        owner: &OwnerRef,
        limit: u32,
    ) -> Result<Vec<EpisodeRecord>, MemoryError> {
        let rows = sqlx::query(
            "SELECT id, owner_kind, owner_id, namespace, thread_id, run_id, episode_kind, role, \
             content, content_type, source_uri, entities_json, tags_json, importance, occurred_at, ingested_at \
             FROM episodes WHERE owner_kind = ? AND owner_id = ? ORDER BY ingested_at DESC LIMIT ?",
        )
        .bind(owner_kind_str(&owner.kind))
        .bind(&owner.id)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

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
    let memory_kind = query.memory_kind.map(|k| k.as_str().to_string());

    let rows = sqlx::query(
        "SELECT id, owner_kind, owner_id, namespace, memory_kind, stability, status, superseded_by, \
         title, content, summary, confidence, importance, valid_from, valid_to, \
         sources_json, tags_json, entities_json, created_at, updated_at \
         FROM memories \
         WHERE owner_kind = ? AND owner_id = ? AND status = 'active' \
         AND namespace LIKE ? \
         AND (? IS NULL OR memory_kind = ?) \
         ORDER BY updated_at DESC LIMIT ?",
    )
    .bind(owner_kind_str(&query.owner.kind))
    .bind(&query.owner.id)
    .bind(namespace_pattern)
    .bind(&memory_kind)
    .bind(&memory_kind)
    .bind(query.limit as i64)
    .fetch_all(pool)
    .await?;

    rows.into_iter().map(row_to_memory).collect()
}

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
    .await?;

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
    serde_json::from_str(raw).map_err(MemoryError::from)
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
