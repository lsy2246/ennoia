use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{ApiKey, ApiKeyStore, AuthError, CreateApiKeyRequest};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteApiKeyStore {
    pool: SqlitePool,
}

impl SqliteApiKeyStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

trait IntoAuthError<T> {
    fn auth_err(self) -> Result<T, AuthError>;
}

impl<T> IntoAuthError<T> for Result<T, sqlx::Error> {
    fn auth_err(self) -> Result<T, AuthError> {
        self.map_err(|e| AuthError::Backend(e.to_string()))
    }
}

#[async_trait]
impl ApiKeyStore for SqliteApiKeyStore {
    async fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKey, AuthError> {
        let id = format!("apikey-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        let scopes_json =
            serde_json::to_string(&req.scopes).map_err(|e| AuthError::Serde(e.to_string()))?;

        sqlx::query(
            "INSERT INTO api_keys (id, user_id, key_hash, label, scopes_json, created_at, expires_at, last_used_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, NULL)",
        )
        .bind(&id)
        .bind(&req.user_id)
        .bind(&req.key_hash)
        .bind(&req.label)
        .bind(&scopes_json)
        .bind(&now)
        .bind(&req.expires_at)
        .execute(&self.pool)
        .await
        .auth_err()?;

        Ok(ApiKey {
            id,
            user_id: req.user_id,
            key_hash: req.key_hash,
            label: req.label,
            scopes: req.scopes,
            created_at: now,
            expires_at: req.expires_at,
            last_used_at: None,
        })
    }

    async fn find_by_key_hash(&self, key_hash: &str) -> Result<Option<ApiKey>, AuthError> {
        let row = sqlx::query(
            "SELECT id, user_id, key_hash, label, scopes_json, created_at, expires_at, last_used_at \
             FROM api_keys WHERE key_hash = ?",
        )
        .bind(key_hash)
        .fetch_optional(&self.pool)
        .await
        .auth_err()?;
        Ok(row.map(row_to_api_key))
    }

    async fn list(&self) -> Result<Vec<ApiKey>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, key_hash, label, scopes_json, created_at, expires_at, last_used_at \
             FROM api_keys ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .auth_err()?;
        Ok(rows.into_iter().map(row_to_api_key).collect())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKey>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, key_hash, label, scopes_json, created_at, expires_at, last_used_at \
             FROM api_keys WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .auth_err()?;
        Ok(rows.into_iter().map(row_to_api_key).collect())
    }

    async fn touch_used(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        sqlx::query("UPDATE api_keys SET last_used_at = ? WHERE id = ?")
            .bind(now_iso)
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM api_keys WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }
}

fn row_to_api_key(row: sqlx::sqlite::SqliteRow) -> ApiKey {
    let scopes_json: String = row.get("scopes_json");
    ApiKey {
        id: row.get("id"),
        user_id: row.get("user_id"),
        key_hash: row.get("key_hash"),
        label: row.get("label"),
        scopes: serde_json::from_str(&scopes_json).unwrap_or_default(),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        last_used_at: row.get("last_used_at"),
    }
}
