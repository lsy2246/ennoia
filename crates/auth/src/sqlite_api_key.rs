use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{ApiKey, ApiKeyStore, AuthError, CreateApiKeyRequest};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Iden)]
enum ApiKeys {
    #[iden = "api_keys"]
    Table,
    Id,
    UserId,
    KeyHash,
    Label,
    ScopesJson,
    CreatedAt,
    ExpiresAt,
    LastUsedAt,
}

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

fn api_key_columns() -> Vec<ApiKeys> {
    vec![
        ApiKeys::Id,
        ApiKeys::UserId,
        ApiKeys::KeyHash,
        ApiKeys::Label,
        ApiKeys::ScopesJson,
        ApiKeys::CreatedAt,
        ApiKeys::ExpiresAt,
        ApiKeys::LastUsedAt,
    ]
}

#[async_trait]
impl ApiKeyStore for SqliteApiKeyStore {
    async fn create(&self, req: CreateApiKeyRequest) -> Result<ApiKey, AuthError> {
        let id = format!("apikey-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        let scopes_json =
            serde_json::to_string(&req.scopes).map_err(|e| AuthError::Serde(e.to_string()))?;

        let (sql, values) = Query::insert()
            .into_table(ApiKeys::Table)
            .columns(api_key_columns())
            .values_panic([
                id.clone().into(),
                req.user_id.clone().into(),
                req.key_hash.clone().into(),
                req.label.clone().into(),
                scopes_json.clone().into(),
                now.clone().into(),
                req.expires_at.clone().into(),
                Option::<String>::None.into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
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
        let (sql, values) = Query::select()
            .columns(api_key_columns())
            .from(ApiKeys::Table)
            .and_where(Expr::col(ApiKeys::KeyHash).eq(key_hash))
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        Ok(row.map(row_to_api_key))
    }

    async fn list(&self) -> Result<Vec<ApiKey>, AuthError> {
        let (sql, values) = Query::select()
            .columns(api_key_columns())
            .from(ApiKeys::Table)
            .order_by(ApiKeys::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .auth_err()?;
        Ok(rows.into_iter().map(row_to_api_key).collect())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiKey>, AuthError> {
        let (sql, values) = Query::select()
            .columns(api_key_columns())
            .from(ApiKeys::Table)
            .and_where(Expr::col(ApiKeys::UserId).eq(user_id))
            .order_by(ApiKeys::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .auth_err()?;
        Ok(rows.into_iter().map(row_to_api_key).collect())
    }

    async fn touch_used(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::update()
            .table(ApiKeys::Table)
            .values([(ApiKeys::LastUsedAt, now_iso.to_string().into())])
            .and_where(Expr::col(ApiKeys::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::delete()
            .from_table(ApiKeys::Table)
            .and_where(Expr::col(ApiKeys::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
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
