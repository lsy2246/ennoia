# Ennoia 数据模型

## 核心产品模型

当前一等模型：

- `Session`
- `Message`
- `Run / Task / Artifact`
- `AgentConfig`
- `SkillConfig`
- `ProviderConfig`
- `ExtensionRuntimeState`
- `MemoryRecord`
- `Job`
- `SystemLog`
- `WorkspaceProfile`

## Session 域

### Session

- `id`
- `topology`
- `owner`
- `title`
- `participants`
- `default_lane_id`
- `created_at`
- `updated_at`

说明：

- `1 Agent = direct`
- `2+ Agents = group`
- 每个 Session 都有唯一 `id`

### Message

- `id`
- `conversation_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`
- `created_at`

补充路由字段：

- `addressed_agents`

## Agent 域

### AgentConfig

- `id`
- `display_name`
- `description`
- `system_prompt`
- `provider_id`
- `model_id`
- `reasoning_effort`
- `skills`
- `enabled`

说明：

- `provider_id` 的产品语义是“API 上游渠道”
- 不再把 `role` 作为固定职责字段
- Session 的产物目录与临时目录不属于 Agent 字段

## Skill 域

### SkillConfig

- `id`
- `display_name`
- `description`
- `source`
- `entry`
- `enabled`

说明：

- Skill 是能力包目录
- Skill 与 Extension 严格分离
- 是否启用由 Agent 自己的配置决定

## API 上游渠道域

### ProviderConfig

- `id`
- `display_name`
- `kind`
- `description`
- `base_url`
- `api_key_env`
- `default_model`
- `available_models`
- `enabled`

说明：

- 产品名称统一为“API 上游渠道”
- `kind` 表示接口类型

## Extension 域

### ExtensionRuntimeState

- `id`
- `name`
- `enabled`
- `status`
- `version`
- `kind`
- `source_mode`
- `install_dir`
- `source_root`
- `diagnostics`

说明：

- 扩展以扩展包为分区标准
- 扩展可以贡献视图、面板、主题、语言、命令和 API 上游渠道实现

## Memory 域

### MemoryRecord

- `id`
- `owner`
- `namespace`
- `memory_kind`
- `stability`
- `status`
- `title`
- `content`
- `summary`
- `confidence`
- `importance`
- `sources`
- `tags`
- `entities`
- `created_at`
- `updated_at`

说明：

- 记忆通过数据库管理
- Web 负责可视化、recall 与 review

## 计划任务域

### Job

- `id`
- `owner_kind`
- `owner_id`
- `job_kind`
- `schedule_kind`
- `schedule_value`
- `payload_json`
- `status`
- `retry_count`
- `max_retries`
- `last_run_at`
- `next_run_at`
- `error`
- `created_at`
- `updated_at`

## 日志域

### SystemLog

- `id`
- `kind`
- `source`
- `level`
- `title`
- `summary`
- `details`
- `run_id`
- `task_id`
- `at`

说明：

- 统一日志流聚合前端、后端、扩展事件与运行摘要
