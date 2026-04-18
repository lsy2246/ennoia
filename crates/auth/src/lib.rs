//! Ennoia auth: password hashing, token minting, sqlite stores, auth service.

pub mod hashing;
pub mod service;
pub mod sqlite_api_key;
pub mod sqlite_session;
pub mod sqlite_user;
pub mod tokens;

pub use hashing::{hash_password, verify_password};
pub use service::{AuthService, LoginOutcome};
pub use sqlite_api_key::SqliteApiKeyStore;
pub use sqlite_session::SqliteSessionStore;
pub use sqlite_user::SqliteUserStore;
pub use tokens::{generate_api_key, generate_session_token, hash_token, mint_jwt, verify_jwt};

pub fn module_name() -> &'static str {
    "auth"
}
