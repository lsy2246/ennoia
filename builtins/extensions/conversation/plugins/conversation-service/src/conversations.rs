use ennoia_kernel::{
    ConversationBranchSpec, ConversationCheckpointSpec, ConversationSpec, ConversationTopology,
    LaneSpec, MessageRole, MessageSpec, OwnerKind, OwnerRef,
};
use sqlx::{Row, SqlitePool};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct ConversationStore {
    pool: SqlitePool,
}

impl ConversationStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub async fn list_conversations(&self) -> Result<Vec<ConversationSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, topology, owner_kind, owner_id, space_id, title, active_branch_id, default_lane_id, created_at, updated_at
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
            "SELECT id, topology, owner_kind, owner_id, space_id, title, active_branch_id, default_lane_id, created_at, updated_at
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
             (id, topology, owner_kind, owner_id, space_id, title, active_branch_id, default_lane_id, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               topology = excluded.topology,
               owner_kind = excluded.owner_kind,
               owner_id = excluded.owner_id,
               space_id = excluded.space_id,
               title = excluded.title,
               active_branch_id = excluded.active_branch_id,
               default_lane_id = excluded.default_lane_id,
               updated_at = excluded.updated_at",
        )
        .bind(&conversation.id)
        .bind(topology_str(conversation.topology))
        .bind(owner_kind_str(conversation.owner.kind))
        .bind(&conversation.owner.id)
        .bind(&conversation.space_id)
        .bind(&conversation.title)
        .bind(&conversation.active_branch_id)
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
        sqlx::query(
            "DELETE FROM lane_members
             WHERE lane_id IN (
               SELECT id FROM lanes WHERE conversation_id = ?
             )",
        )
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
        sqlx::query("DELETE FROM checkpoints WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM branches WHERE conversation_id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn list_branches(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ConversationBranchSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, name, kind, status, parent_branch_id, source_message_id, source_checkpoint_id, inherit_mode, created_at, updated_at
             FROM branches WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_branch).collect())
    }

    pub async fn get_branch(
        &self,
        branch_id: &str,
    ) -> Result<Option<ConversationBranchSpec>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, conversation_id, name, kind, status, parent_branch_id, source_message_id, source_checkpoint_id, inherit_mode, created_at, updated_at
             FROM branches WHERE id = ?",
        )
        .bind(branch_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_branch))
    }

    pub async fn upsert_branch(&self, branch: &ConversationBranchSpec) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO branches
             (id, conversation_id, name, kind, status, parent_branch_id, source_message_id, source_checkpoint_id, inherit_mode, created_at, updated_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(id) DO UPDATE SET
               conversation_id = excluded.conversation_id,
               name = excluded.name,
               kind = excluded.kind,
               status = excluded.status,
               parent_branch_id = excluded.parent_branch_id,
               source_message_id = excluded.source_message_id,
               source_checkpoint_id = excluded.source_checkpoint_id,
               inherit_mode = excluded.inherit_mode,
               updated_at = excluded.updated_at",
        )
        .bind(&branch.id)
        .bind(&branch.conversation_id)
        .bind(&branch.name)
        .bind(&branch.kind)
        .bind(&branch.status)
        .bind(&branch.parent_branch_id)
        .bind(&branch.source_message_id)
        .bind(&branch.source_checkpoint_id)
        .bind(&branch.inherit_mode)
        .bind(&branch.created_at)
        .bind(&branch.updated_at)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_checkpoints(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<ConversationCheckpointSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, branch_id, message_id, kind, label, created_at
             FROM checkpoints WHERE conversation_id = ? ORDER BY created_at DESC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(map_checkpoint).collect())
    }

    pub async fn get_checkpoint(
        &self,
        checkpoint_id: &str,
    ) -> Result<Option<ConversationCheckpointSpec>, sqlx::Error> {
        let row = sqlx::query(
            "SELECT id, conversation_id, branch_id, message_id, kind, label, created_at
             FROM checkpoints WHERE id = ?",
        )
        .bind(checkpoint_id)
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(map_checkpoint))
    }

    pub async fn insert_checkpoint(
        &self,
        checkpoint: &ConversationCheckpointSpec,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO checkpoints
             (id, conversation_id, branch_id, message_id, kind, label, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&checkpoint.id)
        .bind(&checkpoint.conversation_id)
        .bind(&checkpoint.branch_id)
        .bind(&checkpoint.message_id)
        .bind(&checkpoint.kind)
        .bind(&checkpoint.label)
        .bind(&checkpoint.created_at)
        .execute(&self.pool)
        .await?;
        Ok(())
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
        branch_id: Option<&str>,
    ) -> Result<Vec<MessageSpec>, sqlx::Error> {
        let rows = sqlx::query(
            "SELECT id, conversation_id, branch_id, lane_id, sender, role, body, mentions_json, reply_to_message_id, rewrite_from_message_id, created_at
             FROM messages WHERE conversation_id = ? ORDER BY created_at ASC",
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await?;

        let all_messages = rows.into_iter().map(map_message).collect::<Vec<_>>();
        let Some(target_branch_id) = branch_id else {
            return Ok(all_messages);
        };

        let branches = self.list_branches(conversation_id).await?;
        Ok(filter_visible_messages(
            &all_messages,
            &branches,
            target_branch_id,
        ))
    }

    pub async fn insert_message(&self, message: &MessageSpec) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO messages
             (id, conversation_id, branch_id, lane_id, sender, role, body, mentions_json, reply_to_message_id, rewrite_from_message_id, created_at)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&message.id)
        .bind(&message.conversation_id)
        .bind(&message.branch_id)
        .bind(&message.lane_id)
        .bind(&message.sender)
        .bind(role_str(message.role))
        .bind(&message.body)
        .bind(serde_json::to_string(&message.mentions).unwrap_or_else(|_| "[]".to_string()))
        .bind(&message.reply_to_message_id)
        .bind(&message.rewrite_from_message_id)
        .bind(&message.created_at)
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
            active_branch_id: row.get("active_branch_id"),
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

fn map_branch(row: sqlx::sqlite::SqliteRow) -> ConversationBranchSpec {
    ConversationBranchSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        name: row.get("name"),
        kind: row.get("kind"),
        status: row.get("status"),
        parent_branch_id: row.get("parent_branch_id"),
        source_message_id: row.get("source_message_id"),
        source_checkpoint_id: row.get("source_checkpoint_id"),
        inherit_mode: row.get("inherit_mode"),
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    }
}

fn map_checkpoint(row: sqlx::sqlite::SqliteRow) -> ConversationCheckpointSpec {
    ConversationCheckpointSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        branch_id: row.get("branch_id"),
        message_id: row.get("message_id"),
        kind: row.get("kind"),
        label: row.get("label"),
        created_at: row.get("created_at"),
    }
}

fn map_message(row: sqlx::sqlite::SqliteRow) -> MessageSpec {
    MessageSpec {
        id: row.get("id"),
        conversation_id: row.get("conversation_id"),
        branch_id: row.get("branch_id"),
        lane_id: row.get("lane_id"),
        sender: row.get("sender"),
        role: role_from_str(&row.get::<String, _>("role")),
        body: row.get("body"),
        mentions: serde_json::from_str(&row.get::<String, _>("mentions_json")).unwrap_or_default(),
        reply_to_message_id: row.get("reply_to_message_id"),
        rewrite_from_message_id: row.get("rewrite_from_message_id"),
        created_at: row.get("created_at"),
    }
}

fn filter_visible_messages(
    messages: &[MessageSpec],
    branches: &[ConversationBranchSpec],
    branch_id: &str,
) -> Vec<MessageSpec> {
    let branch_map = branches
        .iter()
        .cloned()
        .map(|branch| (branch.id.clone(), branch))
        .collect::<HashMap<_, _>>();
    let messages_by_branch = messages.iter().cloned().fold(
        HashMap::<String, Vec<MessageSpec>>::new(),
        |mut acc, message| {
            let key = message
                .branch_id
                .clone()
                .or_else(|| message.lane_id.clone())
                .unwrap_or_default();
            acc.entry(key).or_default().push(message);
            acc
        },
    );

    let mut visiting = HashSet::new();
    collect_branch_messages(branch_id, &branch_map, &messages_by_branch, &mut visiting)
}

fn collect_branch_messages(
    branch_id: &str,
    branches: &HashMap<String, ConversationBranchSpec>,
    messages_by_branch: &HashMap<String, Vec<MessageSpec>>,
    visiting: &mut HashSet<String>,
) -> Vec<MessageSpec> {
    if !visiting.insert(branch_id.to_string()) {
        return Vec::new();
    }

    let Some(branch) = branches.get(branch_id) else {
        visiting.remove(branch_id);
        return messages_by_branch
            .get(branch_id)
            .cloned()
            .unwrap_or_default();
    };

    let mut inherited = if let Some(parent_id) = branch.parent_branch_id.as_deref() {
        let parent_visible =
            collect_branch_messages(parent_id, branches, messages_by_branch, visiting);
        trim_parent_messages(&parent_visible, branch)
    } else {
        Vec::new()
    };

    let mut own = messages_by_branch
        .get(branch_id)
        .cloned()
        .unwrap_or_default();
    own.sort_by(|left, right| left.created_at.cmp(&right.created_at));
    inherited.extend(own);
    visiting.remove(branch_id);
    inherited
}

fn trim_parent_messages(
    parent_messages: &[MessageSpec],
    branch: &ConversationBranchSpec,
) -> Vec<MessageSpec> {
    match branch.inherit_mode.as_str() {
        "none" => Vec::new(),
        "exclusive" => {
            if let Some(source_message_id) = branch.source_message_id.as_deref() {
                if let Some(index) = parent_messages
                    .iter()
                    .position(|message| message.id == source_message_id)
                {
                    return parent_messages[..index].to_vec();
                }
            }
            parent_messages.to_vec()
        }
        _ => {
            if let Some(source_message_id) = branch.source_message_id.as_deref() {
                if let Some(index) = parent_messages
                    .iter()
                    .position(|message| message.id == source_message_id)
                {
                    return parent_messages[..=index].to_vec();
                }
            }
            parent_messages.to_vec()
        }
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
