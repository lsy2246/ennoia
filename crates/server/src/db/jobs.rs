use super::*;

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

pub async fn get_job(pool: &SqlitePool, job_id: &str) -> Result<Option<JobDetailRow>, sqlx::Error> {
    let (sql, values) = Query::select()
        .columns([
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
        ])
        .from(Jobs::Table)
        .and_where(Expr::col(Jobs::Id).eq(job_id))
        .limit(1)
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;
    Ok(row.map(map_job_detail))
}

pub async fn update_job(
    pool: &SqlitePool,
    job_id: &str,
    job_kind: &str,
    schedule_kind: &str,
    schedule_value: &str,
    payload_json: &str,
    next_run_at: Option<String>,
    max_retries: u32,
) -> Result<Option<JobDetailRow>, sqlx::Error> {
    let updated_at = now_iso();
    let (sql, values) = Query::update()
        .table(Jobs::Table)
        .values([
            (Jobs::JobKind, job_kind.to_string().into()),
            (Jobs::ScheduleKind, schedule_kind.to_string().into()),
            (Jobs::ScheduleValue, schedule_value.to_string().into()),
            (Jobs::PayloadJson, payload_json.to_string().into()),
            (Jobs::NextRunAt, next_run_at.clone().into()),
            (Jobs::MaxRetries, i64::from(max_retries).into()),
            (Jobs::UpdatedAt, updated_at.into()),
        ])
        .and_where(Expr::col(Jobs::Id).eq(job_id))
        .build_sqlx(SqliteQueryBuilder);
    let result = sqlx::query_with(&sql, values).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_job(pool, job_id).await
}

pub async fn set_job_status(
    pool: &SqlitePool,
    job_id: &str,
    status: &str,
) -> Result<Option<JobDetailRow>, sqlx::Error> {
    let updated_at = now_iso();
    let next_run_at = if status == "disabled" {
        None
    } else {
        get_job(pool, job_id)
            .await?
            .and_then(|job| job.next_run_at)
            .or(Some(updated_at.clone()))
    };
    let (sql, values) = Query::update()
        .table(Jobs::Table)
        .values([
            (Jobs::Status, status.to_string().into()),
            (Jobs::NextRunAt, next_run_at.into()),
            (Jobs::UpdatedAt, updated_at.into()),
        ])
        .and_where(Expr::col(Jobs::Id).eq(job_id))
        .build_sqlx(SqliteQueryBuilder);
    let result = sqlx::query_with(&sql, values).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_job(pool, job_id).await
}

pub async fn run_job_now(
    pool: &SqlitePool,
    job_id: &str,
) -> Result<Option<JobDetailRow>, sqlx::Error> {
    let now = now_iso();
    let (sql, values) = Query::update()
        .table(Jobs::Table)
        .values([
            (Jobs::Status, "pending".into()),
            (Jobs::NextRunAt, now.clone().into()),
            (Jobs::UpdatedAt, now.into()),
        ])
        .and_where(Expr::col(Jobs::Id).eq(job_id))
        .build_sqlx(SqliteQueryBuilder);
    let result = sqlx::query_with(&sql, values).execute(pool).await?;
    if result.rows_affected() == 0 {
        return Ok(None);
    }
    get_job(pool, job_id).await
}

pub async fn delete_job(pool: &SqlitePool, job_id: &str) -> Result<bool, sqlx::Error> {
    let (sql, values) = Query::delete()
        .from_table(Jobs::Table)
        .and_where(Expr::col(Jobs::Id).eq(job_id))
        .build_sqlx(SqliteQueryBuilder);
    let result = sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(result.rows_affected() > 0)
}
