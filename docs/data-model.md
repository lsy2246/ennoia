# Ennoia 数据模型

## 核心主语义

新的正式模型只保留以下产品主语义：

- `ChatThread`
- `Schedule`
- `AgentProfile`
- `ExtensionRuntimeState`
- `SystemLog`
- `WorkspaceProfile`

旧的顶级产品概念 `空间 / 工作流 / 产物 / 记忆 / 观测台` 不再作为前端一级信息架构存在。

## 聊天域

### ChatThread

- `id`
- `topology`
- `owner.kind`
- `owner.id`
- `title`
- `participants`
- `default_lane_id`
- `created_at`
- `updated_at`

说明：

- 私聊与群聊统一归属 `ChatThread`
- 前端详情页以聊天盒子方式展示

### ChatLane

- `id`
- `conversation_id`
- `name`
- `lane_type`
- `status`
- `goal`
- `participants`
- `created_at`
- `updated_at`

说明：

- 当前仍复用后端 `lane` 作为执行分流载体
- 前端不再把它暴露为顶级产品概念

### ChatMessage

- `id`
- `conversation_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`
- `created_at`

### DelegationThread

- `id`
- `parent_message_id`
- `title`
- `summary`
- `status`
- `participants`
- `created_at`

说明：

- 子 Agent 调用属于消息流中的正式一环
- 不进入主聊天列表
- 可以从主聊天跳转到独立的非正式子聊天窗口

### DelegationMessage

- `id`
- `sender`
- `role`
- `body`
- `created_at`

### ExecutionRun

- `id`
- `owner.kind`
- `owner.id`
- `conversation_id`
- `lane_id`
- `trigger`
- `stage`
- `goal`
- `created_at`
- `updated_at`

### ExecutionStep

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

### RunOutput

- `id`
- `owner.kind`
- `owner.id`
- `run_id`
- `conversation_id`
- `lane_id`
- `kind`
- `relative_path`
- `created_at`

说明：

- 输出不再作为顶级导航
- 作为聊天或运行上下文中的附属结果展示

## 计划任务域

### Schedule

- `id`
- `owner_kind`
- `owner_id`
- `job_kind`
- `schedule_kind`
- `schedule_value`
- `status`
- `next_run_at`
- `created_at`

### ScheduleDetail

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

说明：

- 计划任务必须支持完整 CRUD
- 同时支持启停与立即执行

## Agent 域

### AgentProfile

- `id`
- `display_name`
- `kind`
- `workspace_mode`
- `default_model`
- `skills_dir`
- `workspace_dir`
- `artifacts_dir`

说明：

- Agent 是长期存在的协作者档案
- 聊天可引用多个 Agent 参与

## 扩展域

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

- 扩展支持启用 / 停用 / 重载 / 重启 / 挂载开发目录
- 停用状态需要可见，不能因为运行时移除而丢失管理入口

## 日志域

### SystemLog

- `id`
- `kind`
- `level`
- `title`
- `summary`
- `run_id`
- `task_id`
- `at`

说明：

- 顶级日志页只做总览
- 聊天和任务内部细节日志应贴近详情页呈现

## 偏好与实例

### WorkspaceProfile

- `id`
- `display_name`
- `locale`
- `time_zone`
- `default_space_id`
- `created_at`
- `updated_at`

### UiPreference

- `locale`
- `theme_id`
- `time_zone`
- `date_style`
- `density`
- `motion`
- `version`
- `updated_at`

说明：

- 语言和主题在前端支持即时预览
- 保存后写入实例偏好并持久化
- 时区只影响前端时间展示，不影响调度与后端存储

## SQLite 当前主表

- `workspace_profile`
- `instance_ui_preferences`
- `space_ui_preferences`
- `conversations`
- `conversation_participants`
- `lanes`
- `lane_members`
- `messages`
- `handoffs`
- `runs`
- `tasks`
- `artifacts`
- `jobs`
- `extensions`
- `memories`
