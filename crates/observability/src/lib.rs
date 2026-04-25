//! Shared logging and request-correlation bootstrap for Ennoia.

use std::path::Path;

use http::HeaderMap;
use serde::{Deserialize, Serialize};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

pub const REQUEST_ID_HEADER: &str = "x-request-id";
pub const TRACE_ID_HEADER: &str = "x-trace-id";
pub const SPAN_ID_HEADER: &str = "x-span-id";
pub const TRACEPARENT_HEADER: &str = "traceparent";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceContext {
    pub request_id: String,
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    pub sampled: bool,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestContext {
    pub request_id: String,
    pub trace_id: String,
    pub span_id: String,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    pub sampled: bool,
    pub source: String,
}

impl RequestContext {
    pub fn from_headers(headers: &HeaderMap) -> Self {
        let request_id = headers
            .get(REQUEST_ID_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(str::to_string)
            .unwrap_or_else(next_request_id);
        let propagated = parse_trace_headers(headers);
        let trace_id = propagated
            .as_ref()
            .map(|item| item.trace_id.clone())
            .unwrap_or_else(next_trace_id);
        let parent_span_id = propagated.as_ref().map(|item| item.parent_span_id.clone());
        let sampled = propagated.as_ref().map(|item| item.sampled).unwrap_or(true);

        Self {
            request_id,
            trace_id,
            span_id: next_span_id(),
            parent_span_id,
            sampled,
            source: "http".to_string(),
        }
    }

    pub fn trace_context(&self) -> TraceContext {
        TraceContext {
            request_id: self.request_id.clone(),
            trace_id: self.trace_id.clone(),
            span_id: self.span_id.clone(),
            parent_span_id: self.parent_span_id.clone(),
            sampled: self.sampled,
            source: self.source.clone(),
        }
    }

    pub fn child_trace(&self, source: impl Into<String>) -> TraceContext {
        TraceContext {
            request_id: self.request_id.clone(),
            trace_id: self.trace_id.clone(),
            span_id: next_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
            source: source.into(),
        }
    }
}

impl TraceContext {
    pub fn child(&self, source: impl Into<String>) -> Self {
        Self {
            request_id: self.request_id.clone(),
            trace_id: self.trace_id.clone(),
            span_id: next_span_id(),
            parent_span_id: Some(self.span_id.clone()),
            sampled: self.sampled,
            source: source.into(),
        }
    }

    pub fn to_traceparent(&self) -> String {
        format!(
            "00-{}-{}-{}",
            self.trace_id,
            self.span_id,
            if self.sampled { "01" } else { "00" }
        )
    }
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
    format!("req_{}", Uuid::new_v4().simple())
}

pub fn next_trace_id() -> String {
    Uuid::new_v4().simple().to_string()
}

pub fn next_span_id() -> String {
    let value = Uuid::new_v4().simple().to_string();
    value[..16].to_string()
}

#[derive(Debug, Clone)]
struct PropagatedTrace {
    trace_id: String,
    parent_span_id: String,
    sampled: bool,
}

fn parse_trace_headers(headers: &HeaderMap) -> Option<PropagatedTrace> {
    if let Some(traceparent) = headers
        .get(TRACEPARENT_HEADER)
        .and_then(|value| value.to_str().ok())
    {
        let parts = traceparent.trim().split('-').collect::<Vec<_>>();
        if parts.len() == 4
            && is_valid_trace_id(parts[1])
            && is_valid_span_id(parts[2])
            && parts[3].len() == 2
        {
            let sampled = u8::from_str_radix(parts[3], 16)
                .map(|flags| flags & 0x01 == 0x01)
                .unwrap_or(true);
            return Some(PropagatedTrace {
                trace_id: parts[1].to_string(),
                parent_span_id: parts[2].to_string(),
                sampled,
            });
        }
    }

    let trace_id = headers
        .get(TRACE_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_valid_trace_id(value))?;
    let parent_span_id = headers
        .get(SPAN_ID_HEADER)
        .and_then(|value| value.to_str().ok())
        .filter(|value| is_valid_span_id(value))
        .map(str::to_string)
        .unwrap_or_else(next_span_id);

    Some(PropagatedTrace {
        trace_id: trace_id.to_string(),
        parent_span_id,
        sampled: true,
    })
}

fn is_valid_trace_id(value: &str) -> bool {
    value.len() == 32 && value.chars().all(|item| item.is_ascii_hexdigit())
}

fn is_valid_span_id(value: &str) -> bool {
    value.len() == 16 && value.chars().all(|item| item.is_ascii_hexdigit())
}
