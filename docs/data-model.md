# Ennoia 数据模型

## 核心模型

- `WorkspaceProfile`
- `ConversationSpec`
- `LaneSpec`
- `MessageSpec`
- `RunSpec / TaskSpec / ArtifactSpec`
- `AgentConfig`
- `SkillConfig`
- `ProviderConfig`
- `ExtensionRuntimeState`
- `MemoryRecord`
- `Job`
- `SystemLog`

## Conversation 域

`ConversationSpec` 字段：

- `id`
- `topology`
- `owner`
- `space_id`
- `title`
- `participants`
- `default_lane_id`
- `created_at`
- `updated_at`

约定：

- `agent_ids.len() == 1` 创建 `direct`。
- `agent_ids.len() >= 2` 创建 `group`。
- 产品文案可以称为“会话”，代码与数据库统一使用 `conversation`。

## Message 域

`MessageSpec` 字段：

- `id`
- `conversation_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`
- `created_at`

## Agent 域

`AgentConfig` 字段：

- `id`
- `display_name`
- `description`
- `system_prompt`
- `provider_id`
- `model_id`
- `reasoning_effort`
- `workspace_root`
- `skills`
- `enabled`

`kind`、`workspace_mode`、`default_model`、`skills_dir`、`workspace_dir`、`artifacts_dir` 仍作为运行时派生/内部字段存在，前端产品模型以显式字段为主。

## Skill 域

`SkillConfig` 字段：`id`、`display_name`、`description`、`source`、`entry`、`tags`、`enabled`。

## API 上游渠道域

`ProviderConfig` 字段：`id`、`display_name`、`extension_id`、`kind`、`description`、`base_url`、`api_key_env`、`default_model`、`available_models`、`model_discovery`、`enabled`。

`extension_id` 指向贡献上游实现的扩展。`kind` 表示接口类型；当前内置 OpenAI 渠道统一使用 `openai`。`default_model` 是用户确认后的稳定配置；扩展可以通过 `model_discovery` 和 provider contribution 提供模型建议，用户仍可手动输入模型。

## Extension 域

扩展运行态以 `ExtensionRuntimeState` 为准，扩展包通过 manifest 贡献页面、面板、主题、语言、命令、Hook 和 Provider 实现。

## 数据库快照

- `assets/db.sql`：新库初始化入口，完整、可执行、自包含。
- `assets/migrations/`：数据库结构演进脚本目录；当前为空，后续结构变更时新增 migration。
