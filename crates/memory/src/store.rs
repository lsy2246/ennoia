use async_trait::async_trait;
use ennoia_kernel::{ContextFrame, ContextView, EpisodeRecord, MemoryRecord, OwnerRef, ReviewAction};

use crate::error::MemoryError;
use crate::model::{RememberReceipt, ReviewReceipt};
use crate::requests::{AssembleRequest, EpisodeRequest, RecallQuery, RecallResult, RememberRequest};

/// MemoryStore is the persistence contract for the memory system.
#[async_trait]
pub trait MemoryStore: Send + Sync {
    async fn record_episode(&self, req: EpisodeRequest) -> Result<EpisodeRecord, MemoryError>;
    async fn remember(&self, req: RememberRequest) -> Result<RememberReceipt, MemoryError>;
    async fn recall(&self, query: RecallQuery) -> Result<RecallResult, MemoryError>;
    async fn review(&self, action: ReviewAction) -> Result<ReviewReceipt, MemoryError>;
    async fn assemble_context(&self, req: AssembleRequest) -> Result<ContextView, MemoryError>;
    async fn upsert_frame(&self, frame: ContextFrame) -> Result<(), MemoryError>;
    async fn list_memories(&self, limit: u32) -> Result<Vec<MemoryRecord>, MemoryError>;
    async fn list_episodes_for_owner(
        &self,
        owner: &OwnerRef,
        limit: u32,
    ) -> Result<Vec<EpisodeRecord>, MemoryError>;
}
