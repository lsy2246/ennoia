use sqlx::SqlitePool;

pub const CONVERSATION_SCHEMA_SQL: &str = include_str!("../../../data/schema.sql");

pub async fn initialize_conversation_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(CONVERSATION_SCHEMA_SQL) {
        sqlx::query(&statement).execute(pool).await?;
    }
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
