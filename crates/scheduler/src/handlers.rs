use async_trait::async_trait;

use crate::model::JobRecord;

/// JobHandler is the contract a runtime must implement for each job kind.
#[async_trait]
pub trait JobHandler: Send + Sync {
    fn kind(&self) -> &'static str;
    async fn handle(&self, job: &JobRecord) -> Result<(), String>;
}

/// RetireExpiredHandler is a placeholder no-op that succeeds.
///
/// Real retire logic lives inside the memory crate; this handler exists so the worker
/// can run end-to-end without extra wiring.
#[derive(Debug, Default, Clone, Copy)]
pub struct RetireExpiredHandler;

#[async_trait]
impl JobHandler for RetireExpiredHandler {
    fn kind(&self) -> &'static str {
        "retire_expired"
    }

    async fn handle(&self, _job: &JobRecord) -> Result<(), String> {
        Ok(())
    }
}
