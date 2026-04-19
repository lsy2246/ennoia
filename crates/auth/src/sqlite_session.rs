use async_trait::async_trait;
use chrono::{Duration, Utc};
use ennoia_kernel::{AuthError, CreateSessionRequest, Session, SessionStore};
use sea_query::{Expr, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Iden)]
enum Sessions {
    Table,
    Id,
    UserId,
    TokenHash,
    CreatedAt,
    ExpiresAt,
    LastSeenAt,
    UserAgent,
    Ip,
}

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

fn session_columns() -> Vec<Sessions> {
    vec![
        Sessions::Id,
        Sessions::UserId,
        Sessions::TokenHash,
        Sessions::CreatedAt,
        Sessions::ExpiresAt,
        Sessions::LastSeenAt,
        Sessions::UserAgent,
        Sessions::Ip,
    ]
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

        let (sql, values) = Query::insert()
            .into_table(Sessions::Table)
            .columns(session_columns())
            .values_panic([
                record.id.clone().into(),
                record.user_id.clone().into(),
                record.token_hash.clone().into(),
                record.created_at.clone().into(),
                record.expires_at.clone().into(),
                record.last_seen_at.clone().into(),
                record.user_agent.clone().into(),
                record.ip.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;

        Ok(record)
    }

    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, AuthError> {
        let (sql, values) = Query::select()
            .columns(session_columns())
            .from(Sessions::Table)
            .and_where(Expr::col(Sessions::TokenHash).eq(token_hash))
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        Ok(row.map(row_to_session))
    }

    async fn touch(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::update()
            .table(Sessions::Table)
            .values([(Sessions::LastSeenAt, now_iso.to_string().into())])
            .and_where(Expr::col(Sessions::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::delete()
            .from_table(Sessions::Table)
            .and_where(Expr::col(Sessions::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn delete_by_token_hash(&self, token_hash: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::delete()
            .from_table(Sessions::Table)
            .and_where(Expr::col(Sessions::TokenHash).eq(token_hash))
            .build_sqlx(SqliteQueryBuilder);
        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Session>, AuthError> {
        let (sql, values) = Query::select()
            .columns(session_columns())
            .from(Sessions::Table)
            .and_where(Expr::col(Sessions::UserId).eq(user_id))
            .order_by(Sessions::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(SqliteQueryBuilder);
        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .auth_err()?;
        Ok(rows.into_iter().map(row_to_session).collect())
    }

    async fn list_all(&self) -> Result<Vec<Session>, AuthError> {
        let (sql, values) = Query::select()
            .columns(session_columns())
            .from(Sessions::Table)
            .order_by(Sessions::CreatedAt, sea_query::Order::Desc)
            .build_sqlx(SqliteQueryBuilder);
        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .auth_err()?;
        Ok(rows.into_iter().map(row_to_session).collect())
    }

    async fn prune_expired(&self, now_iso: &str) -> Result<u64, AuthError> {
        let (sql, values) = Query::delete()
            .from_table(Sessions::Table)
            .and_where(Expr::col(Sessions::ExpiresAt).lt(now_iso.to_string()))
            .build_sqlx(SqliteQueryBuilder);
        let result = sqlx::query_with(&sql, values)
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
