use std::fmt;

#[derive(Debug)]
pub enum SchedulerError {
    Sqlx(sqlx::Error),
    Serde(serde_json::Error),
    NotFound(String),
    Invalid(String),
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SchedulerError::Sqlx(error) => write!(f, "scheduler sqlx error: {error}"),
            SchedulerError::Serde(error) => write!(f, "scheduler serde error: {error}"),
            SchedulerError::NotFound(key) => write!(f, "scheduler job not found: {key}"),
            SchedulerError::Invalid(reason) => write!(f, "scheduler invalid input: {reason}"),
        }
    }
}

impl std::error::Error for SchedulerError {}

impl From<sqlx::Error> for SchedulerError {
    fn from(error: sqlx::Error) -> Self {
        SchedulerError::Sqlx(error)
    }
}

impl From<serde_json::Error> for SchedulerError {
    fn from(error: serde_json::Error) -> Self {
        SchedulerError::Serde(error)
    }
}
