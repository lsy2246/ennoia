use sqlx::{Row, SqlitePool};

pub const CONVERSATION_SCHEMA_SQL: &str = include_str!("../../../data/schema.sql");

pub async fn initialize_conversation_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let statements = split_sql_statements(CONVERSATION_SCHEMA_SQL);
    let (table_statements, other_statements): (Vec<_>, Vec<_>) =
        statements.into_iter().partition(|statement| {
            statement
                .trim_start()
                .to_ascii_uppercase()
                .starts_with("CREATE TABLE")
        });

    for statement in table_statements {
        sqlx::query(&statement).execute(pool).await?;
    }
    ensure_column(pool, "conversations", "active_branch_id", "TEXT").await?;
    ensure_column(pool, "messages", "branch_id", "TEXT").await?;
    ensure_column(pool, "messages", "reply_to_message_id", "TEXT").await?;
    ensure_column(pool, "messages", "rewrite_from_message_id", "TEXT").await?;
    migrate_legacy_branch_schema(pool).await?;
    for statement in other_statements {
        sqlx::query(&statement).execute(pool).await?;
    }
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
    if table_has_column(pool, table, column).await? {
        return Ok(());
    }
    let statement = format!("ALTER TABLE {table} ADD COLUMN {column} {definition}");
    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

async fn migrate_legacy_branch_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    if table_has_column(pool, "branches", "source_checkpoint_id").await? {
        rebuild_branches_without_legacy_columns(pool).await?;
    }

    if table_exists(pool, "checkpoints").await? {
        sqlx::query("DROP TABLE checkpoints").execute(pool).await?;
    }

    Ok(())
}

async fn rebuild_branches_without_legacy_columns(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query("ALTER TABLE branches RENAME TO branches_legacy")
        .execute(pool)
        .await?;

    sqlx::query(
        "CREATE TABLE branches (
           id TEXT PRIMARY KEY,
           conversation_id TEXT NOT NULL,
           name TEXT NOT NULL,
           kind TEXT NOT NULL,
           status TEXT NOT NULL,
           parent_branch_id TEXT,
           source_message_id TEXT,
           inherit_mode TEXT NOT NULL,
           created_at TEXT NOT NULL,
           updated_at TEXT NOT NULL
         )",
    )
    .execute(pool)
    .await?;

    sqlx::query(
        "INSERT INTO branches
         (id, conversation_id, name, kind, status, parent_branch_id, source_message_id, inherit_mode, created_at, updated_at)
         SELECT id, conversation_id, name, kind, status, parent_branch_id, source_message_id, inherit_mode, created_at, updated_at
         FROM branches_legacy",
    )
    .execute(pool)
    .await?;

    sqlx::query("DROP TABLE branches_legacy")
        .execute(pool)
        .await?;

    Ok(())
}

async fn table_has_column(
    pool: &SqlitePool,
    table: &str,
    column: &str,
) -> Result<bool, sqlx::Error> {
    let pragma = format!("PRAGMA table_info({table})");
    let rows = sqlx::query(&pragma).fetch_all(pool).await?;
    Ok(rows
        .iter()
        .filter_map(|row| row.try_get::<String, _>("name").ok())
        .any(|name| name == column))
}

async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, sqlx::Error> {
    let row = sqlx::query(
        "SELECT 1
         FROM sqlite_master
         WHERE type = 'table' AND name = ?
         LIMIT 1",
    )
    .bind(table)
    .fetch_optional(pool)
    .await?;
    Ok(row.is_some())
}

async fn backfill_branch_rows(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT OR IGNORE INTO branches
         (id, conversation_id, name, kind, status, parent_branch_id, source_message_id, inherit_mode, created_at, updated_at)
         SELECT l.id,
                l.conversation_id,
                l.name,
                CASE WHEN l.lane_type = 'primary' THEN 'main' ELSE l.lane_type END,
                l.status,
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
