use super::*;

#[derive(Iden)]
enum FrontendLogs {
    #[iden = "frontend_logs"]
    Table,
    Id,
    Level,
    Source,
    Title,
    Summary,
    Details,
    At,
}

pub async fn insert_frontend_log(
    pool: &SqlitePool,
    id: &str,
    level: &str,
    source: &str,
    title: &str,
    summary: &str,
    details: Option<&str>,
    at: &str,
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(FrontendLogs::Table)
        .columns([
            FrontendLogs::Id,
            FrontendLogs::Level,
            FrontendLogs::Source,
            FrontendLogs::Title,
            FrontendLogs::Summary,
            FrontendLogs::Details,
            FrontendLogs::At,
        ])
        .values_panic([
            id.into(),
            level.into(),
            source.into(),
            title.into(),
            summary.into(),
            details.map(ToOwned::to_owned).into(),
            at.into(),
        ])
        .build_sqlx(SqliteQueryBuilder);
    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

pub async fn list_recent_logs(
    pool: &SqlitePool,
    limit: u32,
    q: Option<&str>,
    level: Option<&str>,
    source: Option<&str>,
) -> Result<Vec<LogRecordRow>, sqlx::Error> {
    let limit = i64::from(limit.max(1));

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
        .order_by(RunStageEvents::At, sea_query::Order::Desc)
        .limit(limit as u64)
        .build_sqlx(SqliteQueryBuilder);
    let stage_rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    let (sql, values) = Query::select()
        .columns([
            Decisions::Id,
            Decisions::RunId,
            Decisions::TaskId,
            Decisions::Stage,
            Decisions::NextAction,
            Decisions::PolicyRuleId,
            Decisions::At,
        ])
        .from(Decisions::Table)
        .order_by(Decisions::At, sea_query::Order::Desc)
        .limit(limit as u64)
        .build_sqlx(SqliteQueryBuilder);
    let decision_rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    let (sql, values) = Query::select()
        .columns([
            GateVerdicts::Id,
            GateVerdicts::RunId,
            GateVerdicts::TaskId,
            GateVerdicts::GateName,
            GateVerdicts::Verdict,
            GateVerdicts::Reason,
            GateVerdicts::At,
        ])
        .from(GateVerdicts::Table)
        .order_by(GateVerdicts::At, sea_query::Order::Desc)
        .limit(limit as u64)
        .build_sqlx(SqliteQueryBuilder);
    let gate_rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    let mut logs = Vec::new();
    logs.extend(stage_rows.into_iter().map(|row| {
        let to_stage: String = row.get("to_stage");
        let from_stage: Option<String> = row.get("from_stage");
        let reason: Option<String> = row.get("reason");
        let policy_rule_id: Option<String> = row.get("policy_rule_id");

        LogRecordRow {
            id: row.get("id"),
            kind: "stage".to_string(),
            source: "backend".to_string(),
            level: "info".to_string(),
            title: format!(
                "{} → {}",
                from_stage.unwrap_or_else(|| "start".to_string()),
                to_stage
            ),
            summary: match (reason, policy_rule_id) {
                (Some(reason), Some(rule)) if !reason.is_empty() => {
                    format!("{reason} · rule {rule}")
                }
                (Some(reason), _) if !reason.is_empty() => reason,
                (_, Some(rule)) => format!("rule {rule}"),
                _ => "stage transition".to_string(),
            },
            details: None,
            run_id: row.get("run_id"),
            task_id: None,
            at: row.get("at"),
        }
    }));

    logs.extend(decision_rows.into_iter().map(|row| LogRecordRow {
        id: row.get("id"),
        kind: "decision".to_string(),
        source: "backend".to_string(),
        level: "info".to_string(),
        title: format!("decision @ {}", row.get::<String, _>("stage")),
        summary: format!(
            "{} · rule {}",
            row.get::<String, _>("next_action"),
            row.get::<String, _>("policy_rule_id")
        ),
        details: None,
        run_id: row.get("run_id"),
        task_id: row.get("task_id"),
        at: row.get("at"),
    }));

    logs.extend(gate_rows.into_iter().map(|row| {
        let verdict: String = row.get("verdict");
        let level = match verdict.as_str() {
            "deny" | "failed" => "error",
            "warn" | "pending" => "warn",
            _ => "info",
        };
        let reason: Option<String> = row.get("reason");
        LogRecordRow {
            id: row.get("id"),
            kind: "gate".to_string(),
            source: "backend".to_string(),
            level: level.to_string(),
            title: format!("gate {}", row.get::<String, _>("gate_name")),
            summary: match reason {
                Some(reason) if !reason.is_empty() => format!("{verdict} · {reason}"),
                _ => verdict,
            },
            details: None,
            run_id: row.get("run_id"),
            task_id: row.get("task_id"),
            at: row.get("at"),
        }
    }));

    let (sql, values) = Query::select()
        .columns([
            FrontendLogs::Id,
            FrontendLogs::Level,
            FrontendLogs::Source,
            FrontendLogs::Title,
            FrontendLogs::Summary,
            FrontendLogs::Details,
            FrontendLogs::At,
        ])
        .from(FrontendLogs::Table)
        .order_by(FrontendLogs::At, sea_query::Order::Desc)
        .limit(limit as u64)
        .build_sqlx(SqliteQueryBuilder);
    let frontend_rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;
    logs.extend(frontend_rows.into_iter().map(|row| LogRecordRow {
        id: row.get("id"),
        kind: "frontend".to_string(),
        source: row.get("source"),
        level: row.get("level"),
        title: row.get("title"),
        summary: row.get("summary"),
        details: row.get("details"),
        run_id: None,
        task_id: None,
        at: row.get("at"),
    }));

    logs.retain(|log| matches_log_filter(log, q, level, source));
    logs.sort_by(|left, right| right.at.cmp(&left.at));
    logs.truncate(limit as usize);
    Ok(logs)
}

fn matches_log_filter(
    log: &LogRecordRow,
    q: Option<&str>,
    level: Option<&str>,
    source: Option<&str>,
) -> bool {
    let level_match = level
        .filter(|value| !value.trim().is_empty())
        .map(|value| log.level.eq_ignore_ascii_case(value.trim()))
        .unwrap_or(true);
    let source_match = source
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            log.source.eq_ignore_ascii_case(value.trim())
                || log.kind.eq_ignore_ascii_case(value.trim())
        })
        .unwrap_or(true);
    let query_match = q
        .filter(|value| !value.trim().is_empty())
        .map(|value| {
            let needle = value.trim().to_lowercase();
            log.title.to_lowercase().contains(&needle)
                || log.summary.to_lowercase().contains(&needle)
                || log
                    .details
                    .as_deref()
                    .unwrap_or_default()
                    .to_lowercase()
                    .contains(&needle)
        })
        .unwrap_or(true);
    level_match && source_match && query_match
}
