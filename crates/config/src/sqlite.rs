use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{ConfigChangeRecord, ConfigEntry, ConfigError, ConfigStore};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteConfigStore {
    pool: SqlitePool,
}

impl SqliteConfigStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

trait IntoConfigError<T> {
    fn cfg_backend(self) -> Result<T, ConfigError>;
    fn cfg_serde(self) -> Result<T, ConfigError>;
}

impl<T> IntoConfigError<T> for Result<T, sqlx::Error> {
    fn cfg_backend(self) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::Backend(e.to_string()))
    }
    fn cfg_serde(self) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::Backend(e.to_string()))
    }
}

impl<T> IntoConfigError<T> for Result<T, serde_json::Error> {
    fn cfg_backend(self) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::Serde(e.to_string()))
    }
    fn cfg_serde(self) -> Result<T, ConfigError> {
        self.map_err(|e| ConfigError::Serde(e.to_string()))
    }
}

#[async_trait]
impl ConfigStore for SqliteConfigStore {
    async fn list(&self) -> Result<Vec<ConfigEntry>, ConfigError> {
        let rows = sqlx::query(
            "SELECT key, payload_json, enabled, version, updated_by, updated_at \
             FROM system_config ORDER BY key",
        )
        .fetch_all(&self.pool)
        .await
        .cfg_backend()?;

        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigError> {
        let row = sqlx::query(
            "SELECT key, payload_json, enabled, version, updated_by, updated_at \
             FROM system_config WHERE key = ?",
        )
        .bind(key)
        .fetch_optional(&self.pool)
        .await
        .cfg_backend()?;

        Ok(row.map(row_to_entry))
    }

    async fn put(
        &self,
        key: &str,
        payload: &serde_json::Value,
        updated_by: Option<&str>,
    ) -> Result<ConfigEntry, ConfigError> {
        let now = Utc::now().to_rfc3339();
        let payload_json = serde_json::to_string(payload).cfg_serde()?;

        let existing = self.get(key).await?;
        let old_payload_json = existing.as_ref().map(|e| e.payload_json.clone());
        let new_version = existing.as_ref().map(|e| e.version + 1).unwrap_or(1);

        sqlx::query(
            "INSERT INTO system_config (key, payload_json, enabled, version, updated_by, updated_at) \
             VALUES (?, ?, 1, ?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET \
               payload_json = excluded.payload_json, \
               version = excluded.version, \
               updated_by = excluded.updated_by, \
               updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(&payload_json)
        .bind(new_version as i64)
        .bind(updated_by)
        .bind(&now)
        .execute(&self.pool)
        .await
        .cfg_backend()?;

        let history_id = format!("cfgh-{}", Uuid::new_v4());
        sqlx::query(
            "INSERT INTO system_config_history \
             (id, config_key, old_payload_json, new_payload_json, changed_by, changed_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&history_id)
        .bind(key)
        .bind(&old_payload_json)
        .bind(&payload_json)
        .bind(updated_by)
        .bind(&now)
        .execute(&self.pool)
        .await
        .cfg_backend()?;

        Ok(ConfigEntry {
            key: key.to_string(),
            payload_json,
            enabled: true,
            version: new_version,
            updated_by: updated_by.map(String::from),
            updated_at: now,
        })
    }

    async fn history(
        &self,
        key: &str,
        limit: u32,
    ) -> Result<Vec<ConfigChangeRecord>, ConfigError> {
        let rows = sqlx::query(
            "SELECT id, config_key, old_payload_json, new_payload_json, changed_by, changed_at \
             FROM system_config_history WHERE config_key = ? \
             ORDER BY changed_at DESC LIMIT ?",
        )
        .bind(key)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .cfg_backend()?;

        Ok(rows
            .into_iter()
            .map(|row| ConfigChangeRecord {
                id: row.get("id"),
                config_key: row.get("config_key"),
                old_payload_json: row.get("old_payload_json"),
                new_payload_json: row.get("new_payload_json"),
                changed_by: row.get("changed_by"),
                changed_at: row.get("changed_at"),
            })
            .collect())
    }
}

fn row_to_entry(row: sqlx::sqlite::SqliteRow) -> ConfigEntry {
    let enabled: i64 = row.get("enabled");
    let version: i64 = row.get("version");
    ConfigEntry {
        key: row.get("key"),
        payload_json: row.get("payload_json"),
        enabled: enabled != 0,
        version: version as u32,
        updated_by: row.get("updated_by"),
        updated_at: row.get("updated_at"),
    }
}
