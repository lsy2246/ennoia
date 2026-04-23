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
- `owner`
- `trigger`
- `target`
- `params`
- `enabled`
- `next_run_at`
- `last_run_at`
- `last_status`
- `last_error`
- `created_at`
- `updated_at`

约定：

- `target.extension_id` 和 `target.action_id` 指向扩展声明的 `schedule_actions`。
- `target.kind = "extension"` 时，scheduler 调用扩展 Wasm Worker。
- `target.kind = "command"` 时，scheduler 直接运行本机 shell 命令，字段为 `command.command`、`command.cwd`、`command.timeout_ms`。
- `params` 原样传给扩展 Worker，业务含义由扩展定义。
- Scheduler 只负责计划与触发，不解释业务语义。

## Conversation 接口域

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
- 产品文案可以称为“会话”，系统 API 使用 `conversation`。
- 具体持久化格式由绑定到 `conversation.*`、`lane.*`、`message.*` 的扩展决定。

## Message 接口域

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
- `generation_options`
- `skills`
- `enabled`

`kind`、`default_model`、`skills_dir`、`working_dir`、`artifacts_dir` 作为运行时派生/内部字段存在，前端产品模型以显式字段为主。Agent 工作目录按 `agents/<agent_id>/work` 自动派生。

## Skill 域

`SkillConfig` 字段：`id`、`display_name`、`description`、`source`、`entry`、`tags`、`enabled`。

## API 上游渠道域

`ProviderConfig` 字段：`id`、`display_name`、`kind`、`description`、`base_url`、`api_key_env`、`default_model`、`available_models`、`model_discovery`、`enabled`。

`kind` 表示接口类型，也是系统解析实现扩展的唯一键；当前内置 OpenAI 渠道统一使用 `openai`。`default_model` 是用户确认后的稳定配置；扩展可以通过 `model_discovery` 和 provider contribution 提供模型建议，用户仍可手动输入模型。

## Extension 域

扩展运行态以 `ExtensionRuntimeState` 为准，扩展包通过 manifest 贡献页面、面板、主题、语言、命令、Hook、Provider、Interface 和 Schedule Action 实现。

## 存储快照

- 核心系统配置：`~/.ennoia/config/*.toml`。
- 接口绑定：`~/.ennoia/config/interfaces.toml`。
- 定时计划：`~/.ennoia/data/system/schedules.json`。
- 核心前端日志：`~/.ennoia/logs/frontend.jsonl`。
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`。
- 核心不维护主业务数据库快照。
