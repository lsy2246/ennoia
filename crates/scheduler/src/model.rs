use ennoia_kernel::OwnerRef;
use serde::{Deserialize, Serialize};

/// ScheduleKind groups the supported job scheduling strategies.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ScheduleKind {
    DelaySeconds(u64),
    Cron(String),
    Maintenance(String),
}

/// JobStatus tracks one scheduled job lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Ready,
    Running,
    Completed,
    Failed,
}

/// ScheduledJob stores one scheduler registration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduledJob {
    pub id: String,
    pub owner: OwnerRef,
    pub schedule: ScheduleKind,
    pub description: String,
    pub status: JobStatus,
}
