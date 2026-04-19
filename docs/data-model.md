# Ennoia 数据模型

## 核心实体

### WorkspaceProfile

- `id`
- `display_name`
- `locale`
- `time_zone`
- `default_space_id`
- `created_at`
- `updated_at`

### Space

- `id`
- `display_name`
- `description`
- `primary_goal`
- `mention_policy`
- `default_agents`

### Conversation

- `id`
- `topology`
- `owner_kind`
- `owner_id`
- `space_id`
- `title`
- `participants`
- `default_lane_id`
- `created_at`
- `updated_at`

### Lane

- `id`
- `conversation_id`
- `space_id`
- `name`
- `lane_type`
- `status`
- `goal`
- `participants`
- `created_at`
- `updated_at`

### Message

- `id`
- `conversation_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`
- `created_at`

### Handoff

- `id`
- `from_lane_id`
- `to_lane_id`
- `from_agent_id`
- `to_agent_id`
- `summary`
- `instructions`
- `status`
- `created_at`

### Run

- `id`
- `owner_kind`
- `owner_id`
- `conversation_id`
- `lane_id`
- `trigger`
- `stage`
- `goal`
- `created_at`
- `updated_at`

### Task

- `id`
- `run_id`
- `conversation_id`
- `lane_id`
- `task_kind`
- `title`
- `assigned_agent_id`
- `status`
- `created_at`
- `updated_at`

### Artifact

- `id`
- `owner_kind`
- `owner_id`
- `run_id`
- `conversation_id`
- `lane_id`
- `artifact_kind`
- `relative_path`
- `created_at`

## UI 偏好

### InstanceUiPreference

- `locale`
- `theme_id`
- `time_zone`
- `date_style`
- `density`
- `motion`
- `version`
- `updated_at`

### SpaceUiPreference

- 与实例级偏好字段一致
- 按 `space_id` 作用

## SQLite 当前主表

- `workspace_profile`
- `instance_ui_preferences`
- `space_ui_preferences`
- `spaces`
- `conversations`
- `conversation_participants`
- `lanes`
- `lane_members`
- `messages`
- `handoffs`
- `runs`
- `tasks`
- `artifacts`
- `memories`
- `jobs`
- `extensions`
