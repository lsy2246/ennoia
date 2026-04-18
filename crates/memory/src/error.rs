use std::fmt;

/// MemoryError unifies sqlx, serde and policy failures emitted by MemoryStore.
#[derive(Debug)]
pub enum MemoryError {
    Sqlx(sqlx::Error),
    Serde(serde_json::Error),
    Policy(String),
    NotFound(String),
    Invalid(String),
}

impl fmt::Display for MemoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MemoryError::Sqlx(error) => write!(f, "memory sqlx error: {error}"),
            MemoryError::Serde(error) => write!(f, "memory serde error: {error}"),
            MemoryError::Policy(reason) => write!(f, "memory policy violation: {reason}"),
            MemoryError::NotFound(key) => write!(f, "memory record not found: {key}"),
            MemoryError::Invalid(reason) => write!(f, "memory invalid input: {reason}"),
        }
    }
}

impl std::error::Error for MemoryError {}

impl From<sqlx::Error> for MemoryError {
    fn from(error: sqlx::Error) -> Self {
        MemoryError::Sqlx(error)
    }
}

impl From<serde_json::Error> for MemoryError {
    fn from(error: serde_json::Error) -> Self {
        MemoryError::Serde(error)
    }
}
