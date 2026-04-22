use super::{RuntimeError, RuntimeStore};
use async_trait::async_trait;
use ennoia_kernel::{
    ArtifactSpec, DecisionSnapshot, GateRecord, HandoffSpec, RunSpec, RunStage, RunStageEvent,
    TaskSpec,
};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};

pub const WORKFLOW_SCHEMA_SQL: &str = include_str!("../../../../data/schema.sql");

pub async fn initialize_workflow_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in WORKFLOW_SCHEMA_SQL
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
    {
        sqlx::query(&format!("{statement};")).execute(pool).await?;
    }
    Ok(())
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
    SignalsJson,
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
    DetailsJson,
    At,
}

/// SqliteRuntimeStore persists workflow-owned runtime audit rows.
#[derive(Debug, Clone)]
pub struct SqliteRuntimeStore {
    pool: SqlitePool,
}

impl SqliteRuntimeStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

trait IntoRuntimeError<T> {
    fn rt_err(self) -> Result<T, RuntimeError>;
}

impl<T> IntoRuntimeError<T> for Result<T, sqlx::Error> {
    fn rt_err(self) -> Result<T, RuntimeError> {
        self.map_err(|e| RuntimeError::Backend(e.to_string()))
    }
}

#[async_trait]
impl RuntimeStore for SqliteRuntimeStore {
    async fn save_run_bundle(
        &self,
        run: &RunSpec,
        tasks: &[TaskSpec],
        artifacts: &[ArtifactSpec],
        handoffs: &[HandoffSpec],
    ) -> Result<(), RuntimeError> {
        let mut transaction = self.pool.begin().await.rt_err()?;
        let run_json =
            serde_json::to_string(run).map_err(|error| RuntimeError::Serde(error.to_string()))?;
        sqlx::query(
            "INSERT INTO runs (id, payload_json, stage, created_at, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET
               payload_json = excluded.payload_json,
               stage = excluded.stage,
               updated_at = excluded.updated_at",
        )
        .bind(&run.id)
        .bind(run_json)
        .bind(run.stage.as_str())
        .bind(&run.created_at)
        .bind(&run.updated_at)
        .execute(&mut *transaction)
        .await
        .rt_err()?;

        sqlx::query("DELETE FROM tasks WHERE run_id = ?1")
            .bind(&run.id)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        for task in tasks {
            let payload_json = serde_json::to_string(task)
                .map_err(|error| RuntimeError::Serde(error.to_string()))?;
            sqlx::query(
                "INSERT INTO tasks (id, run_id, payload_json, created_at, updated_at)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .bind(&task.id)
            .bind(&task.run_id)
            .bind(payload_json)
            .bind(&task.created_at)
            .bind(&task.updated_at)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        }

        sqlx::query("DELETE FROM artifacts WHERE run_id = ?1")
            .bind(&run.id)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        for artifact in artifacts {
            let payload_json = serde_json::to_string(artifact)
                .map_err(|error| RuntimeError::Serde(error.to_string()))?;
            sqlx::query(
                "INSERT INTO artifacts (id, run_id, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(&artifact.id)
            .bind(&artifact.run_id)
            .bind(payload_json)
            .bind(&artifact.created_at)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        }

        sqlx::query("DELETE FROM handoffs WHERE run_id = ?1")
            .bind(&run.id)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        for handoff in handoffs {
            let payload_json = serde_json::to_string(handoff)
                .map_err(|error| RuntimeError::Serde(error.to_string()))?;
            sqlx::query(
                "INSERT INTO handoffs (id, run_id, payload_json, created_at)
                 VALUES (?1, ?2, ?3, ?4)",
            )
            .bind(&handoff.id)
            .bind(&run.id)
            .bind(payload_json)
            .bind(&handoff.created_at)
            .execute(&mut *transaction)
            .await
            .rt_err()?;
        }

        transaction.commit().await.rt_err()?;
        Ok(())
    }

    async fn log_stage_event(&self, event: &RunStageEvent) -> Result<(), RuntimeError> {
        let (sql, values) = Query::insert()
            .into_table(RunStageEvents::Table)
            .columns([
                RunStageEvents::Id,
                RunStageEvents::RunId,
                RunStageEvents::FromStage,
                RunStageEvents::ToStage,
                RunStageEvents::PolicyRuleId,
                RunStageEvents::Reason,
                RunStageEvents::At,
            ])
            .values_panic([
                event.id.clone().into(),
                event.run_id.clone().into(),
                event.from_stage.map(|s| s.as_str().to_string()).into(),
                event.to_stage.as_str().to_string().into(),
                event.policy_rule_id.clone().into(),
                event.reason.clone().into(),
                event.at.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .rt_err()?;
        Ok(())
    }

    async fn log_decision(&self, snapshot: &DecisionSnapshot) -> Result<(), RuntimeError> {
        let (sql, values) = Query::insert()
            .into_table(Decisions::Table)
            .columns([
                Decisions::Id,
                Decisions::RunId,
                Decisions::TaskId,
                Decisions::Stage,
                Decisions::SignalsJson,
                Decisions::NextAction,
                Decisions::PolicyRuleId,
                Decisions::At,
            ])
            .values_panic([
                snapshot.id.clone().into(),
                snapshot.run_id.clone().into(),
                snapshot.task_id.clone().into(),
                snapshot.stage.clone().into(),
                snapshot.signals_json.clone().into(),
                snapshot.next_action.clone().into(),
                snapshot.policy_rule_id.clone().into(),
                snapshot.at.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .rt_err()?;
        Ok(())
    }

    async fn log_gate_verdict(&self, record: &GateRecord) -> Result<(), RuntimeError> {
        let (sql, values) = Query::insert()
            .into_table(GateVerdicts::Table)
            .columns([
                GateVerdicts::Id,
                GateVerdicts::RunId,
                GateVerdicts::TaskId,
                GateVerdicts::GateName,
                GateVerdicts::Verdict,
                GateVerdicts::Reason,
                GateVerdicts::DetailsJson,
                GateVerdicts::At,
            ])
            .values_panic([
                record.id.clone().into(),
                record.run_id.clone().into(),
                record.task_id.clone().into(),
                record.gate_name.clone().into(),
                record.verdict.clone().into(),
                record.reason.clone().into(),
                record.details_json.clone().into(),
                record.at.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .rt_err()?;
        Ok(())
    }

    async fn get_run(&self, run_id: &str) -> Result<Option<RunSpec>, RuntimeError> {
        let row = sqlx::query("SELECT payload_json FROM runs WHERE id = ?1")
            .bind(run_id)
            .fetch_optional(&self.pool)
            .await
            .rt_err()?;
        row.map(|row| row.get::<String, _>("payload_json"))
            .map(|payload| {
                serde_json::from_str::<RunSpec>(&payload)
                    .map_err(|error| RuntimeError::Serde(error.to_string()))
            })
            .transpose()
    }

    async fn list_tasks_for_run(&self, run_id: &str) -> Result<Vec<TaskSpec>, RuntimeError> {
        let rows =
            sqlx::query("SELECT payload_json FROM tasks WHERE run_id = ?1 ORDER BY updated_at ASC")
                .bind(run_id)
                .fetch_all(&self.pool)
                .await
                .rt_err()?;
        decode_payload_rows(rows, "payload_json")
    }

    async fn list_artifacts_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<ArtifactSpec>, RuntimeError> {
        let rows = sqlx::query(
            "SELECT payload_json FROM artifacts WHERE run_id = ?1 ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .rt_err()?;
        decode_payload_rows(rows, "payload_json")
    }

    async fn list_handoffs_for_run(&self, run_id: &str) -> Result<Vec<HandoffSpec>, RuntimeError> {
        let rows = sqlx::query(
            "SELECT payload_json FROM handoffs WHERE run_id = ?1 ORDER BY created_at ASC",
        )
        .bind(run_id)
        .fetch_all(&self.pool)
        .await
        .rt_err()?;
        decode_payload_rows(rows, "payload_json")
    }

    async fn list_stage_events_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<RunStageEvent>, RuntimeError> {
        let (sql, values) = Query::select()
            .columns([
                RunStageEvents::Id,
                RunStageEvents::RunId,
                RunStageEvents::FromStage,
                RunStageEvents::ToStage,
                RunStageEvents::PolicyRuleId,
                RunStageEvents::Reason,
                RunStageEvents::At,
            ])
            .from(RunStageEvents::Table)
            .and_where(Expr::col(RunStageEvents::RunId).eq(run_id))
            .order_by(RunStageEvents::At, sea_query::Order::Asc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .rt_err()?;

        Ok(rows
            .into_iter()
            .map(|row| RunStageEvent {
                id: row.get("id"),
                run_id: row.get("run_id"),
                from_stage: row
                    .get::<Option<String>, _>("from_stage")
                    .map(|s| RunStage::from_str(&s)),
                to_stage: RunStage::from_str(&row.get::<String, _>("to_stage")),
                policy_rule_id: row.get("policy_rule_id"),
                reason: row.get("reason"),
                at: row.get("at"),
            })
            .collect())
    }

    async fn list_decisions_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<DecisionSnapshot>, RuntimeError> {
        let (sql, values) = Query::select()
            .columns([
                Decisions::Id,
                Decisions::RunId,
                Decisions::TaskId,
                Decisions::Stage,
                Decisions::SignalsJson,
                Decisions::NextAction,
                Decisions::PolicyRuleId,
                Decisions::At,
            ])
            .from(Decisions::Table)
            .and_where(Expr::col(Decisions::RunId).eq(run_id))
            .order_by(Decisions::At, sea_query::Order::Asc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .rt_err()?;

        Ok(rows
            .into_iter()
            .map(|row| DecisionSnapshot {
                id: row.get("id"),
                run_id: row.get("run_id"),
                task_id: row.get("task_id"),
                stage: row.get("stage"),
                signals_json: row.get("signals_json"),
                next_action: row.get("next_action"),
                policy_rule_id: row.get("policy_rule_id"),
                at: row.get("at"),
            })
            .collect())
    }

    async fn list_gate_verdicts_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<GateRecord>, RuntimeError> {
        let (sql, values) = Query::select()
            .columns([
                GateVerdicts::Id,
                GateVerdicts::RunId,
                GateVerdicts::TaskId,
                GateVerdicts::GateName,
                GateVerdicts::Verdict,
                GateVerdicts::Reason,
                GateVerdicts::DetailsJson,
                GateVerdicts::At,
            ])
            .from(GateVerdicts::Table)
            .and_where(Expr::col(GateVerdicts::RunId).eq(run_id))
            .order_by(GateVerdicts::At, sea_query::Order::Asc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .rt_err()?;

        Ok(rows
            .into_iter()
            .map(|row| GateRecord {
                id: row.get("id"),
                run_id: row.get("run_id"),
                task_id: row.get("task_id"),
                gate_name: row.get("gate_name"),
                verdict: row.get("verdict"),
                reason: row.get("reason"),
                details_json: row.get("details_json"),
                at: row.get("at"),
            })
            .collect())
    }
}

fn decode_payload_rows<T>(
    rows: Vec<sqlx::sqlite::SqliteRow>,
    column: &str,
) -> Result<Vec<T>, RuntimeError>
where
    T: serde::de::DeserializeOwned,
{
    rows.into_iter()
        .map(|row| {
            let payload: String = row.get(column);
            serde_json::from_str::<T>(&payload)
                .map_err(|error| RuntimeError::Serde(error.to_string()))
        })
        .collect()
}
