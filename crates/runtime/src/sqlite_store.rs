use async_trait::async_trait;
use ennoia_kernel::{DecisionSnapshot, GateRecord, RunStage, RunStageEvent, RuntimeError, RuntimeStore};
use sqlx::{Row, SqlitePool};

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
        sqlx::query(
            "INSERT INTO run_stage_events \
             (id, run_id, from_stage, to_stage, policy_rule_id, reason, at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&event.id)
        .bind(&event.run_id)
        .bind(event.from_stage.map(|s| s.as_str().to_string()))
        .bind(event.to_stage.as_str())
        .bind(&event.policy_rule_id)
        .bind(&event.reason)
        .bind(&event.at)
        .execute(&self.pool)
        .await
        .rt_err()?;
        Ok(())
    }

    async fn log_decision(&self, snapshot: &DecisionSnapshot) -> Result<(), RuntimeError> {
        sqlx::query(
            "INSERT INTO decisions \
             (id, run_id, task_id, stage, signals_json, next_action, policy_rule_id, at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&snapshot.id)
        .bind(&snapshot.run_id)
        .bind(&snapshot.task_id)
        .bind(&snapshot.stage)
        .bind(&snapshot.signals_json)
        .bind(&snapshot.next_action)
        .bind(&snapshot.policy_rule_id)
        .bind(&snapshot.at)
        .execute(&self.pool)
        .await
        .rt_err()?;
        Ok(())
    }

    async fn log_gate_verdict(&self, record: &GateRecord) -> Result<(), RuntimeError> {
        sqlx::query(
            "INSERT INTO gate_verdicts \
             (id, run_id, task_id, gate_name, verdict, reason, details_json, at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&record.id)
        .bind(&record.run_id)
        .bind(&record.task_id)
        .bind(&record.gate_name)
        .bind(&record.verdict)
        .bind(&record.reason)
        .bind(&record.details_json)
        .bind(&record.at)
        .execute(&self.pool)
        .await
        .rt_err()?;
        Ok(())
    }

    async fn list_stage_events_for_run(
        &self,
        run_id: &str,
    ) -> Result<Vec<RunStageEvent>, RuntimeError> {
        let rows = sqlx::query(
            "SELECT id, run_id, from_stage, to_stage, policy_rule_id, reason, at \
             FROM run_stage_events WHERE run_id = ? ORDER BY at ASC",
        )
        .bind(run_id)
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
        let rows = sqlx::query(
            "SELECT id, run_id, task_id, stage, signals_json, next_action, policy_rule_id, at \
             FROM decisions WHERE run_id = ? ORDER BY at ASC",
        )
        .bind(run_id)
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
        let rows = sqlx::query(
            "SELECT id, run_id, task_id, gate_name, verdict, reason, details_json, at \
             FROM gate_verdicts WHERE run_id = ? ORDER BY at ASC",
        )
        .bind(run_id)
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
