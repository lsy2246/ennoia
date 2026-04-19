//! Shared logging and request-correlation bootstrap for Ennoia.

use std::path::Path;

use serde::{Deserialize, Serialize};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

pub const REQUEST_ID_HEADER: &str = "x-request-id";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestContext {
    pub request_id: String,
}

#[derive(Debug)]
pub struct ObservabilityGuard {
    _file_guard: WorkerGuard,
}

pub fn init(
    service: &'static str,
    level: &str,
    log_dir: impl AsRef<Path>,
) -> Result<ObservabilityGuard, Box<dyn std::error::Error + Send + Sync>> {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    let log_dir = log_dir.as_ref();
    std::fs::create_dir_all(log_dir)?;

    let file_appender = tracing_appender::rolling::never(log_dir, format!("{service}.log"));
    let (file_writer, file_guard) = tracing_appender::non_blocking(file_appender);

    let stdout_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .with_thread_names(false)
        .with_ansi(true)
        .compact();

    let file_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_ansi(false)
        .json()
        .flatten_event(true)
        .with_writer(file_writer);

    let _ = tracing_subscriber::registry()
        .with(filter)
        .with(stdout_layer)
        .with(file_layer)
        .try_init();

    Ok(ObservabilityGuard {
        _file_guard: file_guard,
    })
}

pub fn next_request_id() -> String {
    format!("req_{}", uuid::Uuid::new_v4().simple())
}
