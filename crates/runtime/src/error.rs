use std::fmt;

/// RuntimeError wraps sqlx and serde failures in the runtime layer.
#[derive(Debug)]
pub enum RuntimeError {
    Sqlx(sqlx::Error),
    Serde(serde_json::Error),
    Invalid(String),
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::Sqlx(error) => write!(f, "runtime sqlx error: {error}"),
            RuntimeError::Serde(error) => write!(f, "runtime serde error: {error}"),
            RuntimeError::Invalid(reason) => write!(f, "runtime invalid input: {reason}"),
        }
    }
}

impl std::error::Error for RuntimeError {}

impl From<sqlx::Error> for RuntimeError {
    fn from(error: sqlx::Error) -> Self {
        RuntimeError::Sqlx(error)
    }
}

impl From<serde_json::Error> for RuntimeError {
    fn from(error: serde_json::Error) -> Self {
        RuntimeError::Serde(error)
    }
}
