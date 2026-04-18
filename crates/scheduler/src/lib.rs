//! Scheduler runs delayed, cron and maintenance jobs.

pub mod error;
pub mod handlers;
pub mod model;
pub mod sqlite_store;
pub mod store;
pub mod worker;

pub use error::SchedulerError;
pub use handlers::{JobHandler, RetireExpiredHandler};
pub use model::{EnqueueRequest, JobKind, JobRecord, JobStatus, ScheduleKind};
pub use sqlite_store::SqliteSchedulerStore;
pub use store::SchedulerStore;
pub use worker::Worker;

/// Returns the current scheduler module name.
pub fn module_name() -> &'static str {
    "scheduler"
}
