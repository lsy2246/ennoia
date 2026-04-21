//! Scheduler domain: job types, traits and queue error.
//!
//! The scheduler module owns these contracts and ships the built-in worker.

use async_trait::async_trait;
use ennoia_kernel::OwnerRef;
use serde::{Deserialize, Serialize};

// ========== Job kinds ==========

/// JobKind lists the canonical built-in job categories.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobKind {
    DistillEpisodes,
    ComputeEmbedding,
    RetireExpired,
    Custom(String),
}

impl JobKind {
    pub fn as_str(&self) -> &str {
        match self {
            JobKind::DistillEpisodes => "distill_episodes",
            JobKind::ComputeEmbedding => "compute_embedding",
            JobKind::RetireExpired => "retire_expired",
            JobKind::Custom(s) => s.as_str(),
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "distill_episodes" => JobKind::DistillEpisodes,
            "compute_embedding" => JobKind::ComputeEmbedding,
            "retire_expired" => JobKind::RetireExpired,
            other => JobKind::Custom(other.to_string()),
        }
    }
}

/// ScheduleKind lists supported scheduling strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScheduleKind {
    Once,
    DelaySeconds,
    Interval,
    Cron,
}

impl ScheduleKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ScheduleKind::Once => "once",
            ScheduleKind::DelaySeconds => "delay",
            ScheduleKind::Interval => "interval",
            ScheduleKind::Cron => "cron",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "interval" => ScheduleKind::Interval,
            "cron" => ScheduleKind::Cron,
            "delay" => ScheduleKind::DelaySeconds,
            _ => ScheduleKind::Once,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Pending,
    Running,
    Done,
    Failed,
}

impl JobStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Pending => "pending",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
        }
    }

    pub fn from_str(value: &str) -> Self {
        match value {
            "running" => JobStatus::Running,
            "done" => JobStatus::Done,
            "failed" => JobStatus::Failed,
            _ => JobStatus::Pending,
        }
    }
}

/// EnqueueRequest describes a job to register in the queue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EnqueueRequest {
    pub owner: OwnerRef,
    pub job_kind: JobKind,
    pub schedule_kind: ScheduleKind,
    pub schedule_value: String,
    pub payload: serde_json::Value,
    pub max_retries: Option<u32>,
    pub run_at: Option<String>,
}

/// JobRecord is the canonical scheduled job shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JobRecord {
    pub id: String,
    pub owner: OwnerRef,
    pub job_kind: JobKind,
    pub schedule_kind: ScheduleKind,
    pub schedule_value: String,
    pub payload_json: String,
    pub status: JobStatus,
    pub retry_count: u32,
    pub max_retries: u32,
    pub last_run_at: Option<String>,
    pub next_run_at: Option<String>,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

// ========== SchedulerStore ==========

#[async_trait]
pub trait SchedulerStore: Send + Sync {
    async fn enqueue(&self, req: EnqueueRequest) -> Result<JobRecord, SchedulerError>;
    async fn list(&self, limit: u32) -> Result<Vec<JobRecord>, SchedulerError>;
    async fn fetch_due(&self, now_iso: &str, limit: u32) -> Result<Vec<JobRecord>, SchedulerError>;
    async fn mark_running(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError>;
    async fn mark_done(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError>;
    async fn mark_failed(&self, id: &str, now_iso: &str, error: &str)
        -> Result<(), SchedulerError>;
}

// ========== JobHandler ==========

#[async_trait]
pub trait JobHandler: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn handle(&self, job: &JobRecord) -> Result<(), String>;
}

// ========== Error ==========

#[derive(Debug)]
pub enum SchedulerError {
    Backend(String),
    Serde(String),
    NotFound(String),
    Invalid(String),
}

impl std::fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SchedulerError::Backend(reason) => write!(f, "scheduler backend error: {reason}"),
            SchedulerError::Serde(reason) => write!(f, "scheduler serde error: {reason}"),
            SchedulerError::NotFound(key) => write!(f, "scheduler job not found: {key}"),
            SchedulerError::Invalid(reason) => write!(f, "scheduler invalid input: {reason}"),
        }
    }
}

impl std::error::Error for SchedulerError {}
