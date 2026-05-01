# Ennoia 数据模型

## 核心模型

- `RuntimeProfile`
- `AgentConfig`
- `SkillConfig`
- `ProviderConfig`
- `ExtensionRuntimeState`
- `ActionRule`
- `ScheduleRecord`
- `SystemLog`

核心模型表达系统配置、扩展运行态、动作规则、scheduler 计划和宿主协议。Conversation、Message、Memory、Run、Task、Artifact 等业务数据由对应扩展在私有边界内管理。

## ServerConfig 域

`ServerConfig` 当前包含：

- `host`
- `port`
- `rate_limit`
- `cors`
- `timeout`
- `logging`
- `body_limit`
- `bootstrap`

约定：

- 动作管道是系统内部实现边界，不暴露为运行时配置。
- conversation、memory、workflow 各自仍是自己的原生数据边界；系统只在动作管道与事件链里把事实拼接成业务流程。

## Action Rule 域

`ActionRule` 字段：

- `action`
- `capability_id`
- `method`
- `phase`
- `priority`
- `enabled`
- `result_mode`
- `when`
- `schema`

约定：

- `action` 是系统动作键，例如 `conversation.list`、`message.append`、`run.create`。
- `capability_id` 是扩展自己的能力标识。
- `method` 指向扩展 Worker 的 RPC entry。
- `phase` 支持 `before`、`execute`、`after_success`、`after_error`。
- 宿主把同一动作键下的规则收集为一组，按阶段和优先级执行。

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

`AgentConfig` 与 `AgentPermissionPolicy` 统一持久化在 `agents/<agent_id>/agent.toml`。`kind`、`default_model`、`skills_dir`、`working_dir`、`artifacts_dir` 作为运行时派生/内部字段存在，前端产品模型以显式字段为主。`working_dir` / `artifacts_dir` 表示 Agent 自己的运行目录，不等同于用户项目工作区；默认分别按 `agents/<agent_id>/work` 与 `agents/<agent_id>/artifacts` 自动派生。

## Agent 权限域

`AgentPermissionPolicy` 字段：

- `mode`
- `rules`

`AgentPermissionRule` 字段：

- `id`
- `effect`
- `actions`
- `extension_scope`
- `conversation_scope`
- `run_scope`
- `path_include`
- `path_exclude`
- `host_scope`

`PermissionApprovalRecord` 字段：

- `approval_id`
- `status`
- `agent_id`
- `action`
- `target`
- `scope`
- `trigger`
- `matched_rule_id`
- `reason`
- `created_at`
- `resolved_at`
- `resolution`

`PermissionEventRecord` 字段：

- `event_id`
- `agent_id`
- `action`
- `decision`
- `target`
- `scope`
- `extension_id`
- `matched_rule_id`
- `approval_id`
- `trace_id`
- `created_at`

约定：

- Policy 是系统级主模型，扩展只声明 `capabilities[].metadata.permission`，不保存最终授权结果。
- `effect` 固定使用 `allow`、`deny`、`ask`。
- 审批通过后可以产生临时 grant，也可以直接写回 policy。
- 事件记录只表达“谁、在什么作用域、请求了什么、系统如何裁决”，不复写业务结果。

## Skill 域

`SkillConfig` 字段：`id`、`display_name`、`description`、`source`、`entry`、`docs`、`keywords`、`enabled`。

## API 上游渠道域

`ProviderConfig` 字段：`id`、`display_name`、`kind`、`description`、`base_url`、`api_key_env`、`default_model`、`available_models`、`model_discovery.manual_allowed`、`enabled`。

`kind` 表示接口类型，也是系统解析实现扩展的唯一键；当前内置 OpenAI 渠道统一使用 `openai`。`default_model` 是用户确认后的稳定配置；`available_models` 直接保存模型对象列表，每项只定义三项：`id`、`max_context_tokens`、`max_input_tokens`。其中后两项分别表示模型总上下文上限和最大输入上限；未知时允许为空。系统提供一个统一的“获取上游模型”入口，但只负责按 `kind` 把当前渠道配置转发给对应扩展；具体如何请求上游、如何解析响应，都由扩展自己的 `list_models` 实现决定。`model_discovery.manual_allowed` 只表达该渠道是否允许手动维护模型列表与默认模型。

## Extension 域

扩展运行态以 `ExtensionRuntimeState` 为准。扩展 manifest 只保留一份 `description` 与一份 `docs`；如果需要进入会话目录，再通过 `conversation.inject`、`conversation.resource_types` 和 `conversation.capabilities` 声明会话装配规则。会话里只复用这一份 `description`，`docs` 仍然只是按需查阅的文档入口。

## 存储快照

- 核心系统配置：`~/.ennoia/config/*.toml`。
- Agent 基础配置与权限策略：`~/.ennoia/agents/{agent_id}/agent.toml`。
- 定时计划：`~/.ennoia/data/system/schedules.json`。
- Agent 权限事件与审批：`~/.ennoia/data/system/sqlite/permissions.db`。
- 核心前端日志：`~/.ennoia/logs/frontend.jsonl`。
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`。
- 核心不维护主业务数据库快照。
- `memory` 不再维护原始会话消息镜像或 session shadow state。
