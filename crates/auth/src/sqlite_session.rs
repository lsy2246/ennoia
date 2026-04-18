use async_trait::async_trait;
use chrono::{Duration, Utc};
use ennoia_kernel::{AuthError, CreateSessionRequest, Session, SessionStore};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct SqliteSessionStore {
    pool: SqlitePool,
}

impl SqliteSessionStore {
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
impl SessionStore for SqliteSessionStore {
    async fn create(&self, req: CreateSessionRequest) -> Result<Session, AuthError> {
        let id = format!("sess-{}", Uuid::new_v4());
        let now = Utc::now();
        let expires = now + Duration::seconds(req.ttl_seconds as i64);
        let record = Session {
            id,
            user_id: req.user_id,
            token_hash: req.token_hash,
            created_at: now.to_rfc3339(),
            expires_at: expires.to_rfc3339(),
            last_seen_at: None,
            user_agent: req.user_agent,
            ip: req.ip,
        };

        sqlx::query(
            "INSERT INTO sessions (id, user_id, token_hash, created_at, expires_at, last_seen_at, user_agent, ip) \
             VALUES (?, ?, ?, ?, ?, NULL, ?, ?)",
        )
        .bind(&record.id)
        .bind(&record.user_id)
        .bind(&record.token_hash)
        .bind(&record.created_at)
        .bind(&record.expires_at)
        .bind(&record.user_agent)
        .bind(&record.ip)
        .execute(&self.pool)
        .await
        .auth_err()?;

        Ok(record)
    }

    async fn find_by_token_hash(
        &self,
        token_hash: &str,
    ) -> Result<Option<Session>, AuthError> {
        let row = sqlx::query(
            "SELECT id, user_id, token_hash, created_at, expires_at, last_seen_at, user_agent, ip \
             FROM sessions WHERE token_hash = ?",
        )
        .bind(token_hash)
        .fetch_optional(&self.pool)
        .await
        .auth_err()?;
        Ok(row.map(row_to_session))
    }

    async fn touch(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        sqlx::query("UPDATE sessions SET last_seen_at = ? WHERE id = ?")
            .bind(now_iso)
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM sessions WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), AuthError> {
        sqlx::query("DELETE FROM sessions WHERE token_hash = ?")
            .bind(token_hash)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Session>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, token_hash, created_at, expires_at, last_seen_at, user_agent, ip \
             FROM sessions WHERE user_id = ? ORDER BY created_at DESC",
        )
        .bind(user_id)
        .fetch_all(&self.pool)
        .await
        .auth_err()?;
        Ok(rows.into_iter().map(row_to_session).collect())
    }

    async fn list_all(&self) -> Result<Vec<Session>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, user_id, token_hash, created_at, expires_at, last_seen_at, user_agent, ip \
             FROM sessions ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .auth_err()?;
        Ok(rows.into_iter().map(row_to_session).collect())
    }

    async fn prune_expired(&self, now_iso: &str) -> Result<u64, AuthError> {
        let result = sqlx::query("DELETE FROM sessions WHERE expires_at < ?")
            .bind(now_iso)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(result.rows_affected())
    }
}

fn row_to_session(row: sqlx::sqlite::SqliteRow) -> Session {
    Session {
        id: row.get("id"),
        user_id: row.get("user_id"),
        token_hash: row.get("token_hash"),
        created_at: row.get("created_at"),
        expires_at: row.get("expires_at"),
        last_seen_at: row.get("last_seen_at"),
        user_agent: row.get("user_agent"),
        ip: row.get("ip"),
    }
}
