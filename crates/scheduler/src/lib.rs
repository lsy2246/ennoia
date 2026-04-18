//! Scheduler implementations: sqlite store, built-in handlers, worker loop.

pub mod handlers;
pub mod sqlite_store;
pub mod worker;

pub use handlers::RetireExpiredHandler;
pub use sqlite_store::SqliteSchedulerStore;
pub use worker::Worker;

pub fn module_name() -> &'static str {
    "scheduler"
}
