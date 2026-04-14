use ennoia_kernel::OwnerRef;

use crate::model::{ContextView, MemoryRecord, ReviewWorkbench};

/// MemoryService stores and assembles owner-scoped memory views.
#[derive(Debug, Clone, Default)]
pub struct MemoryService {
    records: Vec<MemoryRecord>,
}

impl MemoryService {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn remember(&mut self, record: MemoryRecord) {
        self.records.push(record);
    }

    pub fn recall_for_owner(&self, owner: &OwnerRef, limit: usize) -> Vec<MemoryRecord> {
        self.records
            .iter()
            .filter(|record| &record.owner == owner)
            .take(limit)
            .cloned()
            .collect()
    }

    pub fn build_context(
        &self,
        owner: &OwnerRef,
        recent_messages: Vec<String>,
        active_tasks: Vec<String>,
    ) -> ContextView {
        let recalled_memories = self
            .recall_for_owner(owner, 5)
            .into_iter()
            .map(|record| record.summary)
            .collect();

        ContextView {
            thread_facts: vec![format!("owner:{}:{}", owner_kind_label(owner), owner.id)],
            recent_messages,
            active_tasks,
            recalled_memories,
            workspace_summary: vec![format!("workspace for {}", owner.id)],
        }
    }

    pub fn open_review_workbench(&self, owner: OwnerRef) -> ReviewWorkbench {
        ReviewWorkbench {
            owner: Some(owner),
            open_findings: Vec::new(),
            review_snapshots: Vec::new(),
        }
    }
}

fn owner_kind_label(owner: &OwnerRef) -> &'static str {
    match owner.kind {
        ennoia_kernel::OwnerKind::Global => "global",
        ennoia_kernel::OwnerKind::Agent => "agent",
        ennoia_kernel::OwnerKind::Space => "space",
    }
}
