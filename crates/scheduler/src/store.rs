use async_trait::async_trait;

use crate::error::SchedulerError;
use crate::model::{EnqueueRequest, JobRecord};

/// SchedulerStore handles persistence for the job queue.
#[async_trait]
pub trait SchedulerStore: Send + Sync {
    async fn enqueue(&self, req: EnqueueRequest) -> Result<JobRecord, SchedulerError>;
    async fn list(&self, limit: u32) -> Result<Vec<JobRecord>, SchedulerError>;
    async fn fetch_due(&self, now_iso: &str, limit: u32) -> Result<Vec<JobRecord>, SchedulerError>;
    async fn mark_running(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError>;
    async fn mark_done(&self, id: &str, now_iso: &str) -> Result<(), SchedulerError>;
    async fn mark_failed(
        &self,
        id: &str,
        now_iso: &str,
        error: &str,
    ) -> Result<(), SchedulerError>;
}
