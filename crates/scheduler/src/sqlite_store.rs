use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{
    EnqueueRequest, JobKind, JobRecord, JobStatus, OwnerKind, OwnerRef, ScheduleKind,
    SchedulerError, SchedulerStore,
};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

/// SqliteSchedulerStore persists jobs into the shared sqlite pool.
#[derive(Debug, Clone)]
pub struct SqliteSchedulerStore {
    pool: SqlitePool,
}

impl SqliteSchedulerStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

trait IntoSchedulerError<T> {
    fn sch_backend(self) -> Result<T, SchedulerError>;
    fn sch_serde(self) -> Result<T, SchedulerError>;
}

impl<T> IntoSchedulerError<T> for Result<T, sqlx::Error> {
    fn sch_backend(self) -> Result<T, SchedulerError> {
        self.map_err(|e| SchedulerError::Backend(e.to_string()))
    }
    fn sch_serde(self) -> Result<T, SchedulerError> {
        self.map_err(|e| SchedulerError::Backend(e.to_string()))
    }
}

impl<T> IntoSchedulerError<T> for Result<T, serde_json::Error> {
    fn sch_backend(self) -> Result<T, SchedulerError> {
        self.map_err(|e| SchedulerError::Serde(e.to_string()))
    }
    fn sch_serde(self) -> Result<T, SchedulerError> {
        self.map_err(|e| SchedulerError::Serde(e.to_string()))
    }
}

#[async_trait]
impl SchedulerStore for SqliteSchedulerStore {
    async fn enqueue(&self, req: EnqueueRequest) -> Result<JobRecord, SchedulerError> {
        let now = Utc::now().to_rfc3339();
        let payload_json = serde_json::to_string(&req.payload).sch_serde()?;
        let record = JobRecord {
            id: format!("job-{}", Uuid::new_v4()),
            owner: req.owner.clone(),
            job_kind: req.job_kind.clone(),
            schedule_kind: req.schedule_kind.clone(),
            schedule_value: req.schedule_value.clone(),
            payload_json,
            status: JobStatus::Pending,
            retry_count: 0,
            max_retries: req.max_retries.unwrap_or(3),
            last_run_at: None,
            next_run_at: Some(req.run_at.unwrap_or_else(|| now.clone())),
            error: None,
            created_at: now.clone(),
            updated_at: now,
        };

        sqlx::query(
            "INSERT INTO jobs \
             (id, owner_kind, owner_id, job_kind, schedule_kind, schedule_value, payload_json, status, \
              retry_count, max_retries, last_run_at, next_run_at, error, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(owner_kind_str(&record.owner.kind))
        .bind(&record.owner.id)
        .bind(record.job_kind.as_str())
        .bind(record.schedule_kind.as_str())
        .bind(&record.schedule_value)
        .bind(&record.payload_json)
        .bind(record.status.as_str())
        .bind(record.retry_count as i64)
        .bind(record.max_retries as i64)
        .bind(&record.last_run_at)
        .bind(&record.next_run_at)
        .bind(&record.error)
        .bind(&record.created_at)
        .bind(&record.updated_at)
        .execute(&self.pool)
        .await
        .sch_backend()?;

        Ok(record)
    }

    async fn list(&self, limit: u32) -> Result<Vec<JobRecord>, SchedulerError> {
        let rows = sqlx::query(
            "SELECT id, owner_kind, owner_id, job_kind, schedule_kind, schedule_value, payload_json, status, \
             retry_count, max_retries, last_run_at, next_run_at, error, created_at, updated_at \
             FROM jobs ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .sch_backend()?;

        Ok(rows.into_iter().map(row_to_job).collect())
    }

    async fn fetch_due(&self, now_iso: &str, limit: u32) -> Result<Vec<JobRecord>, SchedulerError> {
        let rows = sqlx::query(
            "SELECT id, owner_kind, owner_id, job_kind, schedule_kind, schedule_value, payload_json, status, \
             retry_count, max_retries, last_run_at, next_run_at, error, created_at, updated_at \
             FROM jobs \
             WHERE status = 'pending' AND (next_run_at IS NULL OR next_run_at <= ?) \
             ORDER BY next_run_at ASC LIMIT ?",
        )
        .bind(now_iso)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .sch_backend()?;

        Ok(rows.into_iter().map(row_to_job).collect())
    }

    async fn mark_running(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError> {
        sqlx::query("UPDATE jobs SET status = 'running', updated_at = ? WHERE id = ?")
            .bind(now_iso)
            .bind(id)
            .execute(&self.pool)
            .await
            .sch_backend()?;
        Ok(())
    }

    async fn mark_done(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError> {
        sqlx::query(
            "UPDATE jobs SET status = 'done', last_run_at = ?, updated_at = ?, error = NULL WHERE id = ?",
        )
        .bind(now_iso)
        .bind(now_iso)
        .bind(id)
        .execute(&self.pool)
        .await
        .sch_backend()?;
        Ok(())
    }

    async fn mark_failed(
        &self,
        id: &str,
        now_iso: &str,
        error: &str,
    ) -> Result<(), SchedulerError> {
        sqlx::query(
            "UPDATE jobs SET status = 'failed', last_run_at = ?, updated_at = ?, error = ?, retry_count = retry_count + 1 WHERE id = ?",
        )
        .bind(now_iso)
        .bind(now_iso)
        .bind(error)
        .bind(id)
        .execute(&self.pool)
        .await
        .sch_backend()?;
        Ok(())
    }
}

fn row_to_job(row: sqlx::sqlite::SqliteRow) -> JobRecord {
    JobRecord {
        id: row.get("id"),
        owner: OwnerRef {
            kind: owner_kind_from_str(&row.get::<String, _>("owner_kind")),
            id: row.get("owner_id"),
        },
        job_kind: JobKind::from_str(&row.get::<String, _>("job_kind")),
        schedule_kind: ScheduleKind::from_str(&row.get::<String, _>("schedule_kind")),
        schedule_value: row.get("schedule_value"),
        payload_json: row.get("payload_json"),
        status: JobStatus::from_str(&row.get::<String, _>("status")),
        retry_count: row.get::<i64, _>("retry_count") as u32,
        max_retries: row.get::<i64, _>("max_retries") as u32,
        last_run_at: row.get("last_run_at"),
        next_run_at: row.get("next_run_at"),
        error: row.get("error"),
        created_at: row.get("created_at"),
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
