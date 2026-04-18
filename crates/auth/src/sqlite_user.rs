use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{
    AuthError, CreateUserRequest, UpdateUserRequest, User, UserRole, UserStore,
};
use sea_query::{Expr, Func, Iden, Query, SqliteQueryBuilder};
use sea_query_binder::SqlxBinder;
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

#[derive(Iden)]
enum Users {
    Table,
    Id,
    Username,
    DisplayName,
    PasswordHash,
    Email,
    Role,
    OwnerKind,
    OwnerId,
    CreatedAt,
    UpdatedAt,
    LastLoginAt,
}

#[derive(Debug, Clone)]
pub struct SqliteUserStore {
    pool: SqlitePool,
}

impl SqliteUserStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
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

fn user_columns_readonly() -> Vec<Users> {
    vec![
        Users::Id,
        Users::Username,
        Users::DisplayName,
        Users::Email,
        Users::Role,
        Users::OwnerKind,
        Users::OwnerId,
        Users::CreatedAt,
        Users::UpdatedAt,
        Users::LastLoginAt,
    ]
}

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn create(&self, req: CreateUserRequest) -> Result<User, AuthError> {
        let now = Utc::now().to_rfc3339();
        let id = format!("user-{}", Uuid::new_v4());

        let (sql, values) = Query::select()
            .column(Users::Id)
            .from(Users::Table)
            .and_where(Expr::col(Users::Username).eq(req.username.clone()))
            .build_sqlx(SqliteQueryBuilder);
        let existing = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        if existing.is_some() {
            return Err(AuthError::Duplicate(format!("username {}", req.username)));
        }

        let (sql, values) = Query::insert()
            .into_table(Users::Table)
            .columns([
                Users::Id,
                Users::Username,
                Users::DisplayName,
                Users::PasswordHash,
                Users::Email,
                Users::Role,
                Users::OwnerKind,
                Users::OwnerId,
                Users::CreatedAt,
                Users::UpdatedAt,
            ])
            .values_panic([
                id.clone().into(),
                req.username.clone().into(),
                req.display_name.clone().into(),
                req.password_hash.clone().into(),
                req.email.clone().into(),
                req.role.as_str().to_string().into(),
                req.owner_kind.clone().into(),
                req.owner_id.clone().into(),
                now.clone().into(),
                now.clone().into(),
            ])
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;

        Ok(User {
            id,
            username: req.username,
            display_name: req.display_name,
            email: req.email,
            role: req.role,
            owner_kind: req.owner_kind,
            owner_id: req.owner_id,
            created_at: now.clone(),
            updated_at: now,
            last_login_at: None,
        })
    }

    async fn get(&self, id: &str) -> Result<Option<User>, AuthError> {
        let (sql, values) = Query::select()
            .columns(user_columns_readonly())
            .from(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        Ok(row.map(row_to_user))
    }

    async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(User, String)>, AuthError> {
        let mut cols = user_columns_readonly();
        cols.push(Users::PasswordHash);
        let (sql, values) = Query::select()
            .columns(cols)
            .from(Users::Table)
            .and_where(Expr::col(Users::Username).eq(username))
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        Ok(row.map(|r| {
            let password_hash: String = r.get("password_hash");
            (row_to_user(r), password_hash)
        }))
    }

    async fn list(&self) -> Result<Vec<User>, AuthError> {
        let (sql, values) = Query::select()
            .columns(user_columns_readonly())
            .from(Users::Table)
            .order_by(Users::CreatedAt, sea_query::Order::Asc)
            .build_sqlx(SqliteQueryBuilder);

        let rows = sqlx::query_with(&sql, values)
            .fetch_all(&self.pool)
            .await
            .auth_err()?;
        Ok(rows.into_iter().map(row_to_user).collect())
    }

    async fn update(&self, id: &str, update: UpdateUserRequest) -> Result<User, AuthError> {
        let existing = self
            .get(id)
            .await?
            .ok_or_else(|| AuthError::NotFound(format!("user {id}")))?;
        let now = Utc::now().to_rfc3339();
        let display_name = update.display_name.or(existing.display_name);
        let email = update.email.or(existing.email);
        let role = update.role.unwrap_or(existing.role);

        let (sql, values) = Query::update()
            .table(Users::Table)
            .values([
                (Users::DisplayName, display_name.clone().into()),
                (Users::Email, email.clone().into()),
                (Users::Role, role.as_str().to_string().into()),
                (Users::UpdatedAt, now.clone().into()),
            ])
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;

        Ok(User {
            id: existing.id,
            username: existing.username,
            display_name,
            email,
            role,
            owner_kind: existing.owner_kind,
            owner_id: existing.owner_id,
            created_at: existing.created_at,
            updated_at: now,
            last_login_at: existing.last_login_at,
        })
    }

    async fn set_password(&self, id: &str, password_hash: &str) -> Result<(), AuthError> {
        let now = Utc::now().to_rfc3339();
        let (sql, values) = Query::update()
            .table(Users::Table)
            .values([
                (Users::PasswordHash, password_hash.to_string().into()),
                (Users::UpdatedAt, now.into()),
            ])
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);

        let affected = sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?
            .rows_affected();
        if affected == 0 {
            return Err(AuthError::NotFound(format!("user {id}")));
        }
        Ok(())
    }

    async fn delete(&self, id: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::delete()
            .from_table(Users::Table)
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn touch_login(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        let (sql, values) = Query::update()
            .table(Users::Table)
            .values([(Users::LastLoginAt, now_iso.to_string().into())])
            .and_where(Expr::col(Users::Id).eq(id))
            .build_sqlx(SqliteQueryBuilder);

        sqlx::query_with(&sql, values)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn count(&self) -> Result<u32, AuthError> {
        let (sql, values) = Query::select()
            .expr_as(Func::count(Expr::col(Users::Id)), sea_query::Alias::new("n"))
            .from(Users::Table)
            .build_sqlx(SqliteQueryBuilder);

        let row = sqlx::query_with(&sql, values)
            .fetch_one(&self.pool)
            .await
            .auth_err()?;
        Ok(row.get::<i64, _>("n") as u32)
    }
}

fn row_to_user(row: sqlx::sqlite::SqliteRow) -> User {
    User {
        id: row.get("id"),
        username: row.get("username"),
        display_name: row.get("display_name"),
        email: row.get("email"),
        role: UserRole::from_str(&row.get::<String, _>("role")),
        owner_kind: row.get("owner_kind"),
        owner_id: row.get("owner_id"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
        last_login_at: row.get("last_login_at"),
    }
}
