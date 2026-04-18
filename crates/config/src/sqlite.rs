use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{ConfigChangeRecord, ConfigEntry, ConfigError, ConfigStore};
use sea_query::{Expr, Iden, OnConflict, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Iden)]
enum SystemConfigTbl {
    #[iden = "system_config"]
    Table,
    Key,
    PayloadJson,
    Enabled,
    Version,
    UpdatedBy,
    UpdatedAt,
}

#[derive(Iden)]
enum SystemConfigHistory {
    #[iden = "system_config_history"]
    Table,
    Id,
    ConfigKey,
    OldPayloadJson,
    NewPayloadJson,
    ChangedBy,
    ChangedAt,
}

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
        let (sql, values) = Query::select()
            .columns([
                SystemConfigTbl::Key,
                SystemConfigTbl::PayloadJson,
                SystemConfigTbl::Enabled,
                SystemConfigTbl::Version,
                SystemConfigTbl::UpdatedBy,
                SystemConfigTbl::UpdatedAt,
            ])
            .from(SystemConfigTbl::Table)
            .order_by(SystemConfigTbl::Key, sea_query::Order::Asc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .cfg_backend()?;
        Ok(rows.into_iter().map(row_to_entry).collect())
    }

    async fn get(&self, key: &str) -> Result<Option<ConfigEntry>, ConfigError> {
        let (sql, values) = Query::select()
            .columns([
                SystemConfigTbl::Key,
                SystemConfigTbl::PayloadJson,
                SystemConfigTbl::Enabled,
                SystemConfigTbl::Version,
                SystemConfigTbl::UpdatedBy,
                SystemConfigTbl::UpdatedAt,
            ])
            .from(SystemConfigTbl::Table)
            .and_where(Expr::col(SystemConfigTbl::Key).eq(key))
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
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

        let (sql, values) = Query::insert()
            .into_table(SystemConfigTbl::Table)
            .columns([
                SystemConfigTbl::Key,
                SystemConfigTbl::PayloadJson,
                SystemConfigTbl::Enabled,
                SystemConfigTbl::Version,
                SystemConfigTbl::UpdatedBy,
                SystemConfigTbl::UpdatedAt,
            ])
            .values_panic([
                key.to_string().into(),
                payload_json.clone().into(),
                1i64.into(),
                (new_version as i64).into(),
                updated_by.map(String::from).into(),
                now.clone().into(),
            ])
            .on_conflict(
                OnConflict::column(SystemConfigTbl::Key)
                    .update_columns([
                        SystemConfigTbl::PayloadJson,
                        SystemConfigTbl::Version,
                        SystemConfigTbl::UpdatedBy,
                        SystemConfigTbl::UpdatedAt,
                    ])
                    .to_owned(),
            )
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .cfg_backend()?;

        let history_id = format!("cfgh-{}", Uuid::new_v4());
        let (sql, values) = Query::insert()
            .into_table(SystemConfigHistory::Table)
            .columns([
                SystemConfigHistory::Id,
                SystemConfigHistory::ConfigKey,
                SystemConfigHistory::OldPayloadJson,
                SystemConfigHistory::NewPayloadJson,
                SystemConfigHistory::ChangedBy,
                SystemConfigHistory::ChangedAt,
            ])
            .values_panic([
                history_id.into(),
                key.to_string().into(),
                old_payload_json.into(),
                payload_json.clone().into(),
                updated_by.map(String::from).into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
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
        let (sql, values) = Query::select()
            .columns([
                SystemConfigHistory::Id,
                SystemConfigHistory::ConfigKey,
                SystemConfigHistory::OldPayloadJson,
                SystemConfigHistory::NewPayloadJson,
                SystemConfigHistory::ChangedBy,
                SystemConfigHistory::ChangedAt,
            ])
            .from(SystemConfigHistory::Table)
            .and_where(Expr::col(SystemConfigHistory::ConfigKey).eq(key))
            .order_by(SystemConfigHistory::ChangedAt, sea_query::Order::Desc)
            .limit(limit as u64)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
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
