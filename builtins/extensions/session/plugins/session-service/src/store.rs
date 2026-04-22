use ennoia_kernel::{
    ConversationSpec, ConversationTopology, HandoffSpec, LaneSpec, MessageRole, MessageSpec,
    OwnerKind, OwnerRef,
};
use sqlx::{Row, SqlitePool};

#[derive(Debug, Clone)]
pub struct SessionStore {
    pool: SqlitePool,
}

impl SessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_conversations(&self) -> Result<Vec<ConversationSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at
             FROM conversations ORDER BY updated_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut conversations = Vec::with_capacity(rows.len());
        for row in rows {
            conversations.push(self.map_conversation(row).await?);
        }
        Ok(conversations)
    }

    pub async fn get_conversation(
        &self,
        conversation_id: &str,
    ) -> Result<Option<ConversationSpec>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at
             FROM conversations WHERE id = ?",
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => self.map_conversation(row).await.map(Some),
            None => Ok(None),
        }
    }

    pub async fn upsert_conversation(
        &self,
        conversation: &ConversationSpec,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO conversations
             (id, topology, owner_kind, owner_id, space_id, title, default_lane_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               topology = excluded.topology,
               owner_kind = excluded.owner_kind,
               owner_id = excluded.owner_id,
               space_id = excluded.space_id,
               title = excluded.title,
               default_lane_id = excluded.default_lane_id,
               updated_at = excluded.updated_at",
        )
        .bind(&conversation.id)
        .bind(topology_str(conversation.topology))
        .bind(owner_kind_str(conversation.owner.kind))
        .bind(&conversation.owner.id)
        .bind(&conversation.space_id)
        .bind(&conversation.title)
        .bind(&conversation.default_lane_id)
        .bind(&conversation.created_at)
        .bind(&conversation.updated_at)
        .execute(&self.pool)
        .await?;

        self.replace_participants(&conversation.id, &conversation.participants)
            .await
    }

    pub async fn delete_conversation(&self, conversation_id: &str) -> Result<bool, sqlx::Error> {
        let result = sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM lanes WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM messages WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_lanes(&self, conversation_id: &str) -> Result<Vec<LaneSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, space_id, name, lane_type, status, goal, created_at, updated_at
             FROM lanes WHERE conversation_id = ? ORDER BY updated_at DESC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let mut lanes = Vec::with_capacity(rows.len());
        for row in rows {
            lanes.push(self.map_lane(row).await?);
        }
        Ok(lanes)
    }

    pub async fn upsert_lane(&self, lane: &LaneSpec) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO lanes
             (id, conversation_id, space_id, name, lane_type, status, goal, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               conversation_id = excluded.conversation_id,
               space_id = excluded.space_id,
               name = excluded.name,
               lane_type = excluded.lane_type,
               status = excluded.status,
               goal = excluded.goal,
               updated_at = excluded.updated_at",
        )
        .bind(&lane.id)
        .bind(&lane.conversation_id)
        .bind(&lane.space_id)
        .bind(&lane.name)
        .bind(&lane.lane_type)
        .bind(&lane.status)
        .bind(&lane.goal)
        .bind(&lane.created_at)
        .bind(&lane.updated_at)
        .execute(&self.pool)
        .await?;

        self.replace_lane_members(&lane.id, &lane.participants)
            .await
    }

    pub async fn list_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, lane_id, sender, role, body, mentions_json, created_at
             FROM messages WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_message).collect())
    }

    pub async fn insert_message(&self, message: &MessageSpec) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO messages
             (id, conversation_id, lane_id, sender, role, body, mentions_json, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.lane_id)
        .bind(&message.sender)
        .bind(role_str(message.role))
        .bind(&message.body)
        .bind(serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string()))
        .bind(&message.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_handoffs(&self, lane_id: &str) -> Result<Vec<HandoffSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, from_lane_id, to_lane_id, from_agent_id, to_agent_id, summary, instructions, status, created_at
             FROM handoffs WHERE from_lane_id = ? ORDER BY created_at DESC",
        )
        .bind(lane_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(map_handoff).collect())
    }

    pub async fn insert_handoff(&self, handoff: &HandoffSpec) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO handoffs
             (id, from_lane_id, to_lane_id, from_agent_id, to_agent_id, summary, instructions, status, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&handoff.id)
        .bind(&handoff.from_lane_id)
        .bind(&handoff.to_lane_id)
        .bind(&handoff.from_agent_id)
        .bind(&handoff.to_agent_id)
        .bind(&handoff.summary)
        .bind(&handoff.instructions)
        .bind(&handoff.status)
        .bind(&handoff.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn map_conversation(
        &self,
        row: sqlx::sqlite::SqliteRow,
    ) -> Result<ConversationSpec, sqlx::Error> {
        let id: String = row.get("id");
        Ok(ConversationSpec {
            id: id.clone(),
            topology: topology_from_str(&row.get::<String, _>("topology")),
            owner: OwnerRef::new(
                owner_kind_from_str(&row.get::<String, _>("owner_kind")),
                row.get::<String, _>("owner_id"),
            ),
            space_id: row.get("space_id"),
            title: row.get("title"),
            participants: self.list_participants(&id).await?,
            default_lane_id: row.get("default_lane_id"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn map_lane(&self, row: sqlx::sqlite::SqliteRow) -> Result<LaneSpec, sqlx::Error> {
        let id: String = row.get("id");
        Ok(LaneSpec {
            id: id.clone(),
            conversation_id: row.get("conversation_id"),
            space_id: row.get("space_id"),
            name: row.get("name"),
            lane_type: row.get("lane_type"),
            status: row.get("status"),
            goal: row.get("goal"),
            participants: self.list_lane_members(&id).await?,
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    async fn replace_participants(
        &self,
        conversation_id: &str,
        participants: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM conversation_participants WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        for (position, participant) in participants.iter().enumerate() {
            sqlx::query(
                "INSERT INTO conversation_participants (conversation_id, participant_id, position)
                 VALUES (?, ?, ?)",
            )
            .bind(conversation_id)
            .bind(participant)
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn replace_lane_members(
        &self,
        lane_id: &str,
        participants: &[String],
    ) -> Result<(), sqlx::Error> {
        sqlx::query("DELETE FROM lane_members WHERE lane_id = ?")
            .bind(lane_id)
            .execute(&self.pool)
            .await?;
        for (position, participant) in participants.iter().enumerate() {
            sqlx::query(
                "INSERT INTO lane_members (lane_id, participant_id, position) VALUES (?, ?, ?)",
            )
            .bind(lane_id)
            .bind(participant)
            .bind(position as i64)
            .execute(&self.pool)
            .await?;
        }
        Ok(())
    }

    async fn list_participants(&self, conversation_id: &str) -> Result<Vec<String>, sqlx::Error> {
        list_ordered_strings(
            &self.pool,
            "SELECT participant_id FROM conversation_participants WHERE conversation_id = ? ORDER BY position",
            conversation_id,
        )
        .await
    }

    async fn list_lane_members(&self, lane_id: &str) -> Result<Vec<String>, sqlx::Error> {
        list_ordered_strings(
            &self.pool,
            "SELECT participant_id FROM lane_members WHERE lane_id = ? ORDER BY position",
            lane_id,
        )
        .await
    }
}

async fn list_ordered_strings(
    pool: &SqlitePool,
    sql: &str,
    value: &str,
) -> Result<Vec<String>, sqlx::Error> {
    let rows = sqlx::query(sql).bind(value).fetch_all(pool).await?;
    Ok(rows.into_iter().map(|row| row.get(0)).collect())
}

fn map_message(row: sqlx::sqlite::SqliteRow) -> MessageSpec {
    MessageSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        lane_id: row.get("lane_id"),
        sender: row.get("sender"),
        role: role_from_str(&row.get::<String, _>("role")),
        body: row.get("body"),
        mentions: serde_json::from_str(&row.get::<String, _>("mentions_json")).unwrap_or_default(),
        created_at: row.get("created_at"),
    }
}

fn map_handoff(row: sqlx::sqlite::SqliteRow) -> HandoffSpec {
    HandoffSpec {
        id: row.get("id"),
        from_lane_id: row.get("from_lane_id"),
        to_lane_id: row.get("to_lane_id"),
        from_agent_id: row.get("from_agent_id"),
        to_agent_id: row.get("to_agent_id"),
        summary: row.get("summary"),
        instructions: row.get("instructions"),
        status: row.get("status"),
        created_at: row.get("created_at"),
    }
}

fn topology_str(value: ConversationTopology) -> &'static str {
    match value {
        ConversationTopology::Direct => "direct",
        ConversationTopology::Group => "group",
    }
}

fn topology_from_str(value: &str) -> ConversationTopology {
    match value {
        "group" => ConversationTopology::Group,
        _ => ConversationTopology::Direct,
    }
}

fn owner_kind_str(value: OwnerKind) -> &'static str {
    match value {
        OwnerKind::Agent => "agent",
        OwnerKind::Space => "space",
        OwnerKind::Global => "global",
    }
}

fn owner_kind_from_str(value: &str) -> OwnerKind {
    match value {
        "agent" => OwnerKind::Agent,
        "space" => OwnerKind::Space,
        _ => OwnerKind::Global,
    }
}

fn role_str(value: MessageRole) -> &'static str {
    match value {
        MessageRole::Agent => "agent",
        MessageRole::System => "system",
        MessageRole::Tool => "tool",
        MessageRole::Operator => "operator",
    }
}

fn role_from_str(value: &str) -> MessageRole {
    match value {
        "agent" => MessageRole::Agent,
        "system" => MessageRole::System,
        "tool" => MessageRole::Tool,
        _ => MessageRole::Operator,
    }
}
