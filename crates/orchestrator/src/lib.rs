//! Orchestrator converts conversation events into tasks, plans and runs.

pub mod model;
pub mod service;

pub use model::{PlannedRun, RunRequest, RunTrigger};
pub use service::OrchestratorService;

/// Returns the current orchestrator module name.
pub fn module_name() -> &'static str {
    "orchestrator"
}

#[cfg(test)]
mod tests {
    use ennoia_memory::ContextView;

    use crate::OrchestratorService;

    #[test]
    fn orchestrator_builds_private_run() {
        let service = OrchestratorService::new();
        let plan = service.plan_private_run("coder", "Implement shell", ContextView::default());
        assert_eq!(plan.tasks.len(), 1);
        assert_eq!(plan.run.thread_id, "thread-private-coder");
    }
}
