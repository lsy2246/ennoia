use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub enum RealtimeEvent {
    AgentsChanged,
    ConversationsChanged,
    ConversationChanged { conversation_id: String },
    ExtensionsChanged,
    ModelEndpointsChanged,
    PermissionAgentChanged { agent_id: String },
    PermissionConversationChanged { conversation_id: String },
    SchedulesChanged,
}

#[derive(Clone)]
pub struct RealtimeHub {
    sender: broadcast::Sender<RealtimeEvent>,
}

impl RealtimeHub {
    pub fn new() -> Self {
        let (sender, _) = broadcast::channel(256);
        Self { sender }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<RealtimeEvent> {
        self.sender.subscribe()
    }

    pub fn publish(&self, event: RealtimeEvent) {
        let _ = self.sender.send(event);
    }
}
