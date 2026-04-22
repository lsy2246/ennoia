use sqlx::SqlitePool;

pub const SESSION_SCHEMA_SQL: &str = include_str!("../../../data/schema.sql");

pub async fn initialize_session_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(SESSION_SCHEMA_SQL) {
        sqlx::query(&statement).execute(pool).await?;
    }
    Ok(())
}

fn split_sql_statements(contents: &str) -> Vec<String> {
    contents
        .split(';')
        .map(str::trim)
        .filter(|statement| !statement.is_empty())
        .map(|statement| format!("{statement};"))
        .collect()
}
