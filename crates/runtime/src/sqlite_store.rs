use async_trait::async_trait;
use ennoia_kernel::{
    DecisionSnapshot, GateRecord, RunStage, RunStageEvent, RuntimeError, RuntimeStore,
};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};

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

/// SqliteRuntimeStore persists runtime audit rows into the shared sqlite pool.
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
