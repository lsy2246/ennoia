//! Scheduler runs delayed, cron and maintenance jobs.

pub mod model;
pub mod service;

pub use model::{JobStatus, ScheduleKind, ScheduledJob};
pub use service::SchedulerService;

/// Returns the current scheduler module name.
pub fn module_name() -> &'static str {
    "scheduler"
}

#[cfg(test)]
mod tests {
    use ennoia_kernel::{OwnerKind, OwnerRef};

    use crate::{ScheduleKind, SchedulerService};

    #[test]
    fn scheduler_registers_job() {
        let mut scheduler = SchedulerService::new();
        let job = scheduler.register(
            OwnerRef {
                kind: OwnerKind::Space,
                id: "studio".to_string(),
            },
            ScheduleKind::DelaySeconds(30),
            "nightly review".to_string(),
        );

        assert_eq!(job.id, "job-1");
        assert_eq!(scheduler.jobs().len(), 1);
    }
}
