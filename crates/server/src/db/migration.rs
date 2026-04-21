use std::collections::BTreeSet;

use ennoia_assets::{db_sql, migrations};
use sea_query::{ColumnDef, Expr, Iden, OnConflict, Query, SqliteQueryBuilder, Table};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};

use super::now_iso;

#[derive(Iden)]
enum SchemaMigrations {
    #[iden = "schema_migrations"]
    Table,
    LogicalPath,
    AppliedAt,
}

#[derive(Iden)]
enum SqliteMaster {
    #[iden = "sqlite_master"]
    Table,
    Type,
    Name,
}

pub async fn initialize_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    if !table_exists(pool, "agents").await? {
        execute_migration_contents(pool, db_sql()).await?;
        return Ok(());
    }

    if migrations::all().is_empty() {
        return Ok(());
    }

    ensure_schema_migration_table(pool).await?;
    let applied = load_applied_migrations(pool).await?;
    for migration in migrations::all() {
        if applied.contains(migration.logical_path) {
            continue;
        }

        execute_migration_contents(pool, migration.contents).await?;
        record_applied_migration(pool, migration.logical_path).await?;
    }
    Ok(())
}

async fn ensure_schema_migration_table(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let statement = Table::create()
        .table(SchemaMigrations::Table)
        .if_not_exists()
        .col(
            ColumnDef::new(SchemaMigrations::LogicalPath)
                .string()
                .not_null()
                .primary_key(),
        )
        .col(
            ColumnDef::new(SchemaMigrations::AppliedAt)
                .string()
                .not_null(),
        )
        .to_string(SqliteQueryBuilder);

    sqlx::query(&statement).execute(pool).await?;
    Ok(())
}

async fn load_applied_migrations(pool: &SqlitePool) -> Result<BTreeSet<String>, sqlx::Error> {
    let (sql, values) = Query::select()
        .column(SchemaMigrations::LogicalPath)
        .from(SchemaMigrations::Table)
        .build_sqlx(SqliteQueryBuilder);

    let rows = sqlx::query_with(&sql, values).fetch_all(pool).await?;

    Ok(rows
        .into_iter()
        .map(|row| row.get::<String, _>("logical_path"))
        .collect())
}

async fn record_applied_migration(
    pool: &SqlitePool,
    logical_path: &str,
) -> Result<(), sqlx::Error> {
    let (sql, values) = Query::insert()
        .into_table(SchemaMigrations::Table)
        .columns([SchemaMigrations::LogicalPath, SchemaMigrations::AppliedAt])
        .values_panic([logical_path.to_string().into(), now_iso().into()])
        .on_conflict(
            OnConflict::column(SchemaMigrations::LogicalPath)
                .do_nothing()
                .to_owned(),
        )
        .build_sqlx(SqliteQueryBuilder);

    sqlx::query_with(&sql, values).execute(pool).await?;
    Ok(())
}

async fn table_exists(pool: &SqlitePool, table: &str) -> Result<bool, sqlx::Error> {
    let (sql, values) = Query::select()
        .column(SqliteMaster::Name)
        .from(SqliteMaster::Table)
        .and_where(Expr::col(SqliteMaster::Type).eq("table"))
        .and_where(Expr::col(SqliteMaster::Name).eq(table))
        .limit(1)
        .build_sqlx(SqliteQueryBuilder);

    let row = sqlx::query_with(&sql, values).fetch_optional(pool).await?;
    Ok(row.is_some())
}

async fn execute_migration_contents(pool: &SqlitePool, contents: &str) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(contents) {
        sqlx::query(&statement).execute(pool).await?;
    }
    Ok(())
}

fn split_sql_statements(contents: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut current = String::new();
    let mut in_trigger = false;

    for line in contents.lines() {
        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }
        current.push_str(line);
        current.push('\n');

        let upper = trimmed.to_ascii_uppercase();
        if upper.starts_with("CREATE TRIGGER") || upper.starts_with("CREATE TEMP TRIGGER") {
            in_trigger = true;
        }
        if in_trigger {
            if upper == "END;" {
                statements.push(current.trim().to_string());
                current.clear();
                in_trigger = false;
            }
            continue;
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    #[tokio::test]
    async fn initialize_schema_is_idempotent_for_fresh_database() {
        let pool = memory_pool().await;

        initialize_schema(&pool).await.expect("initialize schema");
        initialize_schema(&pool)
            .await
            .expect("initialize schema twice");

        assert!(!table_exists(&pool, "schema_migrations")
            .await
            .expect("schema migrations table"));
    }

    #[tokio::test]
    async fn initialize_schema_creates_current_tables() {
        let pool = memory_pool().await;

        initialize_schema(&pool)
            .await
            .expect("initialize current schema");

        assert!(table_exists(&pool, "conversations")
            .await
            .expect("conversations table"));
        assert!(table_exists(&pool, "frontend_logs")
            .await
            .expect("frontend logs table"));
    }

    async fn memory_pool() -> SqlitePool {
        let options = SqliteConnectOptions::new()
            .filename(":memory:")
            .create_if_missing(true);
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await
            .expect("sqlite memory pool")
    }
}
