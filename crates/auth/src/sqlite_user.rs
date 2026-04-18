use async_trait::async_trait;
use chrono::Utc;
use ennoia_kernel::{
    AuthError, CreateUserRequest, UpdateUserRequest, User, UserRole, UserStore,
};
use sqlx::{Row, SqlitePool};
use uuid::Uuid;

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

#[async_trait]
impl UserStore for SqliteUserStore {
    async fn create(&self, req: CreateUserRequest) -> Result<User, AuthError> {
        let now = Utc::now().to_rfc3339();
        let id = format!("user-{}", Uuid::new_v4());

        let existing = sqlx::query("SELECT id FROM users WHERE username = ?")
            .bind(&req.username)
            .fetch_optional(&self.pool)
            .await
            .auth_err()?;
        if existing.is_some() {
            return Err(AuthError::Duplicate(format!("username {}", req.username)));
        }

        sqlx::query(
            "INSERT INTO users \
             (id, username, display_name, password_hash, email, role, owner_kind, owner_id, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&id)
        .bind(&req.username)
        .bind(&req.display_name)
        .bind(&req.password_hash)
        .bind(&req.email)
        .bind(req.role.as_str())
        .bind(&req.owner_kind)
        .bind(&req.owner_id)
        .bind(&now)
        .bind(&now)
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
        let row = sqlx::query(
            "SELECT id, username, display_name, email, role, owner_kind, owner_id, created_at, updated_at, last_login_at \
             FROM users WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .auth_err()?;
        Ok(row.map(row_to_user))
    }

    async fn get_by_username(
        &self,
        username: &str,
    ) -> Result<Option<(User, String)>, AuthError> {
        let row = sqlx::query(
            "SELECT id, username, display_name, email, role, owner_kind, owner_id, created_at, updated_at, last_login_at, password_hash \
             FROM users WHERE username = ?",
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .auth_err()?;
        Ok(row.map(|r| {
            let password_hash: String = r.get("password_hash");
            (row_to_user(r), password_hash)
        }))
    }

    async fn list(&self) -> Result<Vec<User>, AuthError> {
        let rows = sqlx::query(
            "SELECT id, username, display_name, email, role, owner_kind, owner_id, created_at, updated_at, last_login_at \
             FROM users ORDER BY created_at ASC",
        )
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

        sqlx::query(
            "UPDATE users SET display_name = ?, email = ?, role = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&display_name)
        .bind(&email)
        .bind(role.as_str())
        .bind(&now)
        .bind(id)
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
        let affected = sqlx::query(
            "UPDATE users SET password_hash = ?, updated_at = ? WHERE id = ?",
        )
        .bind(password_hash)
        .bind(&now)
        .bind(id)
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
        sqlx::query("DELETE FROM users WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn touch_login(&self, id: &str, now_iso: &str) -> Result<(), AuthError> {
        sqlx::query("UPDATE users SET last_login_at = ? WHERE id = ?")
            .bind(now_iso)
            .bind(id)
            .execute(&self.pool)
            .await
            .auth_err()?;
        Ok(())
    }

    async fn count(&self) -> Result<u32, AuthError> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM users")
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
