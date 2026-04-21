use crate::{
    EnqueueRequest, JobKind, JobRecord, JobStatus, ScheduleKind, SchedulerError, SchedulerStore,
};
use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{OwnerKind, OwnerRef};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

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

fn job_columns() -> Vec<Jobs> {
    vec![
        Jobs::Id,
        Jobs::OwnerKind,
        Jobs::OwnerId,
        Jobs::JobKind,
        Jobs::ScheduleKind,
        Jobs::ScheduleValue,
        Jobs::PayloadJson,
        Jobs::Status,
        Jobs::RetryCount,
        Jobs::MaxRetries,
        Jobs::LastRunAt,
        Jobs::NextRunAt,
        Jobs::Error,
        Jobs::CreatedAt,
        Jobs::UpdatedAt,
    ]
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

        let (sql, values) = Query::insert()
            .into_table(Jobs::Table)
            .columns(job_columns())
            .values_panic([
                record.id.clone().into(),
                owner_kind_str(&record.owner.kind).into(),
                record.owner.id.clone().into(),
                record.job_kind.as_str().to_string().into(),
                record.schedule_kind.as_str().to_string().into(),
                record.schedule_value.clone().into(),
                record.payload_json.clone().into(),
                record.status.as_str().to_string().into(),
                (record.retry_count as i64).into(),
                (record.max_retries as i64).into(),
                record.last_run_at.clone().into(),
                record.next_run_at.clone().into(),
                record.error.clone().into(),
                record.created_at.clone().into(),
                record.updated_at.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .sch_backend()?;

        Ok(record)
    }

    async fn list(&self, limit: u32) -> Result<Vec<JobRecord>, SchedulerError> {
        let (sql, values) = Query::select()
            .columns(job_columns())
            .from(Jobs::Table)
            .order_by(Jobs::CreatedAt, sea_query::Order::Desc)
            .limit(limit as u64)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .sch_backend()?;
        Ok(rows.into_iter().map(row_to_job).collect())
    }

    async fn fetch_due(&self, now_iso: &str, limit: u32) -> Result<Vec<JobRecord>, SchedulerError> {
        let (sql, values) = Query::select()
            .columns(job_columns())
            .from(Jobs::Table)
            .and_where(Expr::col(Jobs::Status).eq("pending"))
            .and_where(
                Expr::col(Jobs::NextRunAt)
                    .is_null()
                    .or(Expr::col(Jobs::NextRunAt).lte(now_iso.to_string())),
            )
            .order_by(Jobs::NextRunAt, sea_query::Order::Asc)
            .limit(limit as u64)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .sch_backend()?;
        Ok(rows.into_iter().map(row_to_job).collect())
    }

    async fn mark_running(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError> {
        let (sql, values) = Query::update()
            .table(Jobs::Table)
            .values([
                (Jobs::Status, "running".into()),
                (Jobs::UpdatedAt, now_iso.to_string().into()),
            ])
            .and_where(Expr::col(Jobs::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .sch_backend()?;
        Ok(())
    }

    async fn mark_done(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError> {
        let (sql, values) = Query::update()
            .table(Jobs::Table)
            .values([
                (Jobs::Status, "done".into()),
                (Jobs::LastRunAt, now_iso.to_string().into()),
                (Jobs::UpdatedAt, now_iso.to_string().into()),
                (Jobs::Error, sea_query::Value::String(None).into()),
            ])
            .and_where(Expr::col(Jobs::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
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
        let (sql, values) = Query::update()
            .table(Jobs::Table)
            .values([
                (Jobs::Status, "failed".into()),
                (Jobs::LastRunAt, now_iso.to_string().into()),
                (Jobs::UpdatedAt, now_iso.to_string().into()),
                (Jobs::Error, error.to_string().into()),
            ])
            .value(Jobs::RetryCount, Expr::col(Jobs::RetryCount).add(1))
            .and_where(Expr::col(Jobs::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
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
