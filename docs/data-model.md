# Ennoia 数据模型

## 核心模型

- `RuntimeProfile`
- `AgentConfig`
- `SkillConfig`
- `ModelEndpointConfig`
- `ExtensionRuntimeState`
- `ActionRule`
- `ScheduleRecord`
- `SystemLog`

核心模型表达系统配置、扩展运行态、动作规则、scheduler 计划和宿主协议。Conversation、Message、Memory、Run、Task、Artifact 等业务数据由对应扩展在私有边界内管理。

补充约定：

- 系统核心额外提供 `ExtensionStateEntry` 与 `ExtensionRecordEntry` 两个宿主级通用原语，供扩展保存轻量运行态和会话可视记录。
- 这两个模型不是 workflow、memory、conversation 的替代主数据，只用于跨刷新状态同步、前端挂载和运行事实投影。

## RuntimeProfile 域

`RuntimeProfile` 字段：

- `id`
- `display_name`
- `locale`
- `time_zone`
- `operating_system`
- `default_space_id`
- `created_at`
- `updated_at`

约定：

- `operating_system` 表示当前操作者设备系统，前端优先自动识别后写入；允许为空，兼容旧实例未落盘该字段的 `profile.toml`。

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
- workflow 运行数据以结构化 `plan` 为真相源；`task` 是从 `plan.steps` 派生的执行与展示投影视图，不再作为系统硬编码拆解步骤的来源。

## Action Rule 域

`ActionRule` 字段：

- `action`
- `operation`
- `method`
- `phase`
- `priority`
- `enabled`
- `result_mode`
- `when`
- `schema`

约定：

- `action` 是系统动作键，例如 `conversation.list`、`message.append`、`run.create`。
- `operation` 是扩展 manifest 中的 operation 名称。
- `method` 指向扩展 Worker 的 RPC method；当前与 `operation` 保持一致。
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
- 具体持久化格式由绑定到 `conversation.*`、`branch.*`、`lane.*`、`message.*` 的扩展决定。

`ConversationBranchSpec` 字段：

- `id`
- `conversation_id`
- `name`
- `kind`
- `status`
- `parent_branch_id`
- `source_message_id`
- `inherit_mode`
- `created_at`
- `updated_at`

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
- `parent_message_id`
- `reply_to_message_id`
- `rewrite_from_message_id`
- `created_at`

约定：

- `parent_message_id` 用于把工具输出、系统过程或其他附属消息稳定挂到某条主消息下，不替代分支语义。

## Run 接口域

`RunSpec` 字段：

- `id`
- `owner`
- `conversation_id`
- `lane_id`
- `source_message_id`
- `trigger`
- `stage`
- `goal`
- `created_at`
- `updated_at`

## Agent 域

`AgentConfig` 字段：

- `id`
- `display_name`
- `description`
- `system_prompt`
- `model_endpoint_id`
- `model_id`
- `generation_options`
- `skills`
- `enabled`
- `file_access`

`AgentConfig`、`AgentPermissionProfile` 与 `AgentFileAccessProfile` 统一持久化在 `agents/<agent_id>/agent.toml`。`kind`、`default_model`、`skills_dir`、`working_dir`、`artifacts_dir` 作为运行时派生/内部字段存在，前端产品模型以显式字段为主。`working_dir` / `artifacts_dir` 是模型侧可见的虚拟路径，默认展示为 `/workspace` 与 `/artifacts`，不等同于宿主机绝对路径。

## Agent 权限域

`AgentPermissionProfile` 字段：

- `mode`
- `entries`

## Agent 文件访问域

`AgentFileAccessProfile` 字段：

- `default_root`
- `roots`

`AgentFileAccessRoot` 字段：

- `id`
- `path`
- `mode`

约定：

- 默认文件访问根为 `/workspace`、`/artifacts`、`/tmp`，默认根是 `/workspace`。
- `command.exec` 的 `cwd` 只接受配置过的虚拟根及其子路径；相对路径按 `default_root` 解析。
- 文件访问配置只负责把模型侧虚拟路径解析到 Agent 自己的运行目录，不负责权限裁决。
- 当前文件访问不是进程隔离，也不保证宿主进程无法访问其他宿主文件。

运行时不再让用户直接维护底层规则。系统会把 `AgentPermissionProfile` 编译成内部 `AgentPermissionPolicy`：

- `mode = "whitelist"`：`command.exec` 默认 `ask`，命中 `entries` 后直接 `allow`
- `mode = "blacklist"`：`command.exec` 默认 `allow`，命中 `entries` 后改为 `ask`
- `entries[]` 只描述命令调用匹配规则，每条条目包含：
  - `match`：`exact | prefix | regex`
  - `value`：用户自定义的命令匹配字符串
- `entries[]` 匹配的是规范化后的完整命令调用串，例如 `git status`、`git diff --cached`、`node C:/tools/search-runner.mjs`
- 文件访问与权限模型解耦；它只决定路径入口如何解析，不参与白名单/黑名单判定

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

- Policy 是系统级主模型；扩展 manifest 不声明底层权限边界，宿主按 operation、调用参数和 actor 上下文构造权限请求。
- `effect` 固定使用 `allow`、`deny`、`ask`。
- 审批通过后只会产生临时 grant，不再写回 policy。
- 事件记录只表达“谁、在什么作用域、请求了什么、系统如何裁决”，不复写业务结果。

## Extension Runtime 域

`ExtensionStateEntry` 字段：

- `extension_id`
- `namespace`
- `scope_type`
- `scope_id`
- `key`
- `value`
- `version`
- `updated_at`
- `expires_at`

`ExtensionRecordEntry` 字段：

- `id`
- `extension_id`
- `namespace`
- `scope_type`
- `scope_id`
- `kind`
- `status`
- `title`
- `summary`
- `payload`
- `related_message_id`
- `parent_id`
- `created_at`
- `updated_at`
- `closed_at`

约定：

- `ExtensionStateEntry` 适合保存“当前草案”“会话活跃 route”“上次同步游标”这类小型宿主状态。
- `ExtensionRecordEntry` 适合保存“执行过程块”“规划块”“审批块”这类需要被前端时间线渲染的扩展记录。
- 宿主只按 `extension_id + namespace + scope` 做通用存储与查询，不解释 `payload` 的业务结构。
- 扩展自己的 run、draft、plan、memory graph、conversation message 等主数据仍然保留在扩展私有数据库中。

## Skill 域

Skill 磁盘格式固定为 `SKILL.md + config.toml`。`SKILL.md` 的 YAML frontmatter 提供 `name` 与 `description`；`config.toml` 提供 Ennoia 增强配置。运行时把两者合成为 `SkillManifest`。

`SkillManifest` 字段：`id`、`version`、`description`、`mount.mode`、`actions[]`、`settings[]`、`diagnostics`、`prepare`。

`actions[]` 字段：`id`、`description`、`entry`。

约定：`actions[]` 表示 skill 的对外可调用入口，不等于 skill 内部的所有脚本或子流程；默认一个 skill 只暴露一个 action。

`SkillConfig` 是运行时返回对象，在合成后的 manifest 基础上额外附带：`enabled`、`builtin_sync_blocked`、`readiness`。

## 模型接入域

`ModelEndpointConfig` 字段：`id`、`display_name`、`kind`、`description`、`base_url`、`api_key`、兼容字段 `api_key_env`、`default_model`、`available_models`、`model_discovery.manual_allowed`、`enabled`。

`kind` 表示模型提供方实现类型，也是系统解析实现扩展的唯一键；当前内置 OpenAI 统一使用 `openai`。`api_key` 保存当前接入实例的密钥值；`api_key_env` 只作为旧配置兼容字段保留，旧配置填写时可从服务进程环境中读取密钥，若两者同时存在则优先使用 `api_key`。新建 OpenAI 接入不预填也不展示环境变量字段。`default_model` 是用户确认后的稳定配置；`available_models` 直接保存模型对象列表，每项只定义三项：`id`、`max_context_tokens`、`max_input_tokens`。其中后两项分别表示模型总上下文上限和最大输入上限；未知时允许为空。系统提供一个统一的“获取模型列表”入口，但只负责按 `kind` 把当前模型接入配置转发给对应扩展；具体如何请求上游、如何解析响应，都由扩展自己的 `list_models` 实现决定。`model_discovery.manual_allowed` 只表达该模型接入是否允许手动维护模型列表与默认模型。

## Extension 域

扩展运行态以 `ExtensionRuntimeState` 为准。扩展 manifest 只声明系统可见契约：`id`、`version`、`name`、`description`、`docs`、`compat`、`views`、`operations`、`events`、`settings`、`conversation`。需要进入会话上下文时，通过 `conversation.visible`、`conversation.resources` 和 `conversation.operations` 声明会话装配规则。会话里只复用这一份 `description`，`docs` 仍然只是按需查阅的文档入口。

## 存储快照

- `config/` 保存声明性配置；`data/` 保存全部运行数据。
- 核心系统配置：`~/.ennoia/config/*.toml`。
- Agent 基础配置、权限配置与文件访问配置：`~/.ennoia/agents/{agent_id}/agent.toml`。
- 定时计划：`~/.ennoia/data/system/schedules.json`。
- Agent 权限事件与审批：`~/.ennoia/data/system/sqlite/permissions.db`。
- 扩展通用运行态 state/record：`~/.ennoia/data/system/sqlite/extensions.db`。
- 核心前端日志：`~/.ennoia/data/system/logs/frontend.jsonl`。
- 核心文本日志目录：`~/.ennoia/data/system/logs/`。
- 运行态与开发态 pid 文件：`~/.ennoia/data/system/pids/*.pid`。
- 扩展级宿主配置：`~/.ennoia/config/extensions/{extension_id}.toml`。
- 技能级宿主配置：`~/.ennoia/config/skills/{skill_id}.toml`。
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`。
- 技能私有数据：`~/.ennoia/data/skills/{skill_id}/`。
- 核心不维护主业务数据库快照。
- `memory` 不再维护原始会话消息镜像或 session shadow state。
