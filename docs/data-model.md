# Ennoia 数据模型

## 1. 核心实体

### Agent

- `id`
- `display_name`
- `kind`
- `default_model`
- `workspace_mode`
- `enabled_skills`

### Space

- `id`
- `display_name`
- `kind`
- `mention_policy`
- `default_agents`

### Thread

- `id`
- `space_id`
- `thread_kind`
- `participants`

### Message

- `id`
- `thread_id`
- `sender`
- `body`
- `mentions`
- `created_at`

### Run

- `id`
- `owner_kind`
- `owner_id`
- `thread_id`
- `trigger`
- `status`

### Task

- `id`
- `run_id`
- `task_kind`
- `title`
- `status`
- `assigned_agent_id`

### Artifact

- `id`
- `owner_kind`
- `owner_id`
- `run_id`
- `artifact_kind`
- `relative_path`

## 2. 记忆实体

### MemoryRecord

- `id`
- `owner_kind`
- `owner_id`
- `memory_kind`
- `source`
- `content`
- `summary`
- `created_at`

### ContextView

- `thread_facts`
- `recent_messages`
- `active_tasks`
- `recalled_memories`
- `workspace_summary`

## 3. 扩展实体

### ExtensionManifest

- `id`
- `kind`
- `version`
- `frontend_bundle`
- `backend_entry`
- `contributes`

### SkillSpec

- `id`
- `entry`
- `input_contract`
- `output_contract`
- `capabilities`

## 4. 调度实体

### ScheduledJob

- `id`
- `job_kind`
- `schedule_kind`
- `owner_kind`
- `owner_id`
- `status`

## 5. 归属模型

所有业务对象遵循统一 owner 模型：

- `Global`
- `Agent(<agent_id>)`
- `Space(<space_id>)`

这个模型同时用于：

- run 归属
- artifact 归属
- memory 归属
- scheduler job 归属
