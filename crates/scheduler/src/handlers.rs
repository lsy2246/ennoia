use crate::{JobHandler, JobRecord};
use async_trait::async_trait;

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
