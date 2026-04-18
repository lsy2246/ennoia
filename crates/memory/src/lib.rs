//! Memory owns truth, working state, context views and review flows.

pub mod model;
pub mod service;

pub use model::{ContextView, MemoryKind, MemoryRecord, ReviewWorkbench};
pub use service::MemoryService;

/// Returns the current memory module name.
pub fn module_name() -> &'static str {
    "memory"
}

#[cfg(test)]
mod tests {
    use ennoia_kernel::{OwnerKind, OwnerRef};

    use crate::{MemoryKind, MemoryRecord, MemoryService};

    #[test]
    fn memory_service_recalls_by_owner() {
        let mut service = MemoryService::new();
        service.remember(MemoryRecord {
            id: "m-1".to_string(),
            owner: OwnerRef {
                kind: OwnerKind::Agent,
                id: "coder".to_string(),
            },
            thread_id: Some("thread-private-coder".to_string()),
            run_id: Some("run-thread-private-coder-1".to_string()),
            kind: MemoryKind::Working,
            source: "test".to_string(),
            content: "remember this".to_string(),
            summary: "remember".to_string(),
            created_at: "2026-04-15T00:00:00Z".to_string(),
        });

        let owner = OwnerRef {
            kind: OwnerKind::Agent,
            id: "coder".to_string(),
        };
        assert_eq!(service.recall_for_owner(&owner, 10).len(), 1);
        assert_eq!(
            service.recall_for_thread("thread-private-coder", 10).len(),
            1
        );
    }
}
