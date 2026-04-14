use ennoia_kernel::OwnerRef;

use crate::model::{JobStatus, ScheduleKind, ScheduledJob};

/// SchedulerService stores and exposes job registrations.
#[derive(Debug, Clone, Default)]
pub struct SchedulerService {
    jobs: Vec<ScheduledJob>,
}

impl SchedulerService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        owner: OwnerRef,
        schedule: ScheduleKind,
        description: String,
    ) -> ScheduledJob {
        let job = ScheduledJob {
            id: format!("job-{}", self.jobs.len() + 1),
            owner,
            schedule,
            description,
            status: JobStatus::Pending,
        };
        self.jobs.push(job.clone());
        job
    }

    pub fn jobs(&self) -> &[ScheduledJob] {
        &self.jobs
    }
}
