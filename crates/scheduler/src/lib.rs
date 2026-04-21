//! Scheduler module owns queue contracts plus the built-in worker implementation.

pub mod handlers;
pub mod model;
pub mod sqlite_store;
pub mod worker;

pub use handlers::RetireExpiredHandler;
pub use model::{
    EnqueueRequest, JobHandler, JobKind, JobRecord, JobStatus, ScheduleKind, SchedulerError,
    SchedulerStore,
};
pub use sqlite_store::SqliteSchedulerStore;
pub use worker::Worker;

pub fn module_name() -> &'static str {
    "scheduler"
}
