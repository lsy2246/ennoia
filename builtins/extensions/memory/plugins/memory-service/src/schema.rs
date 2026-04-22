use sqlx::SqlitePool;

pub const MEMORY_SCHEMA_SQL: &str = include_str!("../../../data/schema.sql");

pub async fn initialize_memory_schema(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    for statement in split_sql_statements(MEMORY_SCHEMA_SQL) {
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
