use sqlx::{Row, SqlitePool};

pub const CONVERSATION_SCHEMA_SQL: &str = include_str!("../../../data/schema.sql");

pub async fn initialize_conversation_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(CONVERSATION_SCHEMA_SQL) {
        sqlx::query(&statement).execute(pool).await?;
    }
    ensure_column(pool, "conversations", "active_branch_id", "TEXT").await?;
    ensure_column(pool, "messages", "branch_id", "TEXT").await?;
    ensure_column(pool, "messages", "reply_to_message_id", "TEXT").await?;
    ensure_column(pool, "messages", "rewrite_from_message_id", "TEXT").await?;
    backfill_branch_rows(pool).await?;
    Ok(())
}

fn split_sql_statements(contents: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();

    for line in contents.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        current.push_str(line);
        current.push('\n');

        if trimmed.ends_with(';') {
            statements.push(current.trim().to_string());
            current.clear();
        }
    }

    if !current.trim().is_empty() {
        statements.push(current.trim().to_string());
    }

    statements
}

async fn ensure_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
    definition: &str,
) -> Result<(), sqlx::Error> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;
    let exists = rows
        .iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .any(|name| name == column);
    if exists {
        return Ok(());
    }
    let statement = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

async fn backfill_branch_rows(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO branches
         (id, conversation_id, name, kind, status, parent_branch_id, source_message_id, source_checkpoint_id, inherit_mode, created_at, updated_at)
         SELECT l.id,
                l.conversation_id,
                l.name,
                CASE WHEN l.lane_type = 'primary' THEN 'main' ELSE l.lane_type END,
                l.status,
                NULL,
                NULL,
                NULL,
                'inclusive',
                l.created_at,
                l.updated_at
         FROM lanes l",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE conversations
         SET active_branch_id = COALESCE(active_branch_id, default_lane_id)
         WHERE active_branch_id IS NULL",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "UPDATE messages
         SET branch_id = COALESCE(branch_id, lane_id)
         WHERE branch_id IS NULL",
    )
    .execute(pool)
    .await?;

    Ok(())
}
