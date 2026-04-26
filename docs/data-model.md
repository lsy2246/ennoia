# Ennoia 数据模型

## 核心模型

- `RuntimeProfile`
- `AgentConfig`
- `SkillConfig`
- `ProviderConfig`
- `ExtensionRuntimeState`
- `InterfaceBindingsConfig`
- `ScheduleRecord`
- `SystemLog`

核心模型表达系统配置、扩展运行态、接口绑定、scheduler 计划和宿主协议。Conversation、Message、Memory、Run、Task、Artifact 等业务数据由对应扩展在私有边界内管理。

## Interface Binding 域

`InterfaceBindingsConfig` 字段：

- `bindings`

`InterfaceBindingConfig` 字段：

- `extension_id`
- `method`

约定：

- key 是系统动作键，例如 `conversation.list`、`message.append_user`、`run.create`。
- value 指向扩展 ID 和该扩展 Worker 的 RPC method。
- 没有显式绑定且只有一个实现时自动绑定；多个实现时由用户或 UI 写入配置。

## Schedule 域

`ScheduleRecord` 字段：

- `id`
- `name`
- `description`
- `owner`
- `trigger`
- `executor`
- `delivery`
- `retry`
- `enabled`
- `next_run_at`
- `last_run_at`
- `last_status`
- `last_error`
- `last_output`
- `history`
- `created_at`
- `updated_at`

约定：

- `executor.kind = "command"` 时，scheduler 直接运行本机 shell 命令，字段为 `command.command`、`command.cwd`、`command.timeout_ms`。
- `executor.kind = "agent"` 时，scheduler 触发指定 Agent 的编排运行，字段为 `agent.agent_id`、`agent.prompt`、`agent.model_id`、`agent.max_turns`，可选 `agent.context.conversation_id` 作为运行参考上下文；未指定时独立运行。
- `delivery.conversation_id` 可选；存在时，scheduler 会把结果作为系统消息投递到对应会话。
- `delivery.lane_id` 可选；存在时，scheduler 会把结果投递到会话内指定 lane。
- `delivery.content_mode` 可选；支持 `full`、`summary`、`conclusion`。
- `retry` 控制失败重试次数和间隔。
- `history` 保存最近运行记录，包含状态、错误、输出与投递结果。
- Scheduler 只负责计划、触发、重试与记录，不解释业务语义。

## Conversation 接口域

`ConversationSpec` 字段：

- `id`
- `topology`
- `owner`
- `space_id`
- `title`
- `participants`
- `active_branch_id`
- `default_lane_id`
- `created_at`
- `updated_at`

约定：

- `agent_ids.len() == 1` 创建 `direct`。
- `agent_ids.len() >= 2` 创建 `group`。
- 产品文案可以称为“会话”，系统 API 使用 `conversation`。
- 具体持久化格式由绑定到 `conversation.*`、`branch.*`、`checkpoint.*`、`lane.*`、`message.*` 的扩展决定。

`ConversationBranchSpec` 字段：

- `id`
- `conversation_id`
- `name`
- `kind`
- `status`
- `parent_branch_id`
- `source_message_id`
- `source_checkpoint_id`
- `inherit_mode`
- `created_at`
- `updated_at`

`ConversationCheckpointSpec` 字段：

- `id`
- `conversation_id`
- `branch_id`
- `message_id`
- `kind`
- `label`
- `created_at`

## Message 接口域

`MessageSpec` 字段：

- `id`
- `conversation_id`
- `branch_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`
- `reply_to_message_id`
- `rewrite_from_message_id`
- `created_at`

## Agent 域

`AgentConfig` 字段：

- `id`
- `display_name`
- `description`
- `system_prompt`
- `provider_id`
- `model_id`
- `generation_options`
- `skills`
- `enabled`

`kind`、`default_model`、`skills_dir`、`working_dir`、`artifacts_dir` 作为运行时派生/内部字段存在，前端产品模型以显式字段为主。Agent 工作目录按 `agents/<agent_id>/work` 自动派生。

## Skill 域

`SkillConfig` 字段：`id`、`display_name`、`description`、`source`、`entry`、`docs`、`keywords`、`enabled`。

## API 上游渠道域

`ProviderConfig` 字段：`id`、`display_name`、`kind`、`description`、`base_url`、`api_key_env`、`default_model`、`available_models`、`model_discovery`、`enabled`。

`kind` 表示接口类型，也是系统解析实现扩展的唯一键；当前内置 OpenAI 渠道统一使用 `openai`。`default_model` 是用户确认后的稳定配置；扩展可以通过 `model_discovery` 和 provider contribution 提供模型建议，用户仍可手动输入模型。

## Extension 域

扩展运行态以 `ExtensionRuntimeState` 为准。扩展 manifest 只保留一份 `description` 与一份 `docs`；如果需要进入会话目录，再通过 `conversation.inject`、`conversation.resource_types` 和 `conversation.capabilities` 声明会话装配规则。会话里只复用这一份 `description`，`docs` 仍然只是按需查阅的文档入口。

## 存储快照

- 核心系统配置：`~/.ennoia/config/*.toml`。
- 接口绑定：`~/.ennoia/config/interfaces.toml`。
- 定时计划：`~/.ennoia/data/system/schedules.json`。
- 核心前端日志：`~/.ennoia/logs/frontend.jsonl`。
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`。
- 核心不维护主业务数据库快照。
