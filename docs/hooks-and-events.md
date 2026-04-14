# Ennoia Hooks 与事件字典

## 1. 目标

Hooks 是 `system extension` 的行为接入点，事件总线负责把系统关键生命周期暴露给扩展层。

## 2. 事件分类

### App

- `app.starting`
- `app.ready`
- `app.shutdown`

### Extension

- `extension.discovered`
- `extension.registered`
- `extension.enabled`

### Conversation

- `thread.message.received`
- `thread.message.persisted`
- `thread.message.composed`

### Run

- `run.created`
- `run.started`
- `run.blocked`
- `run.completed`

### Task

- `task.created`
- `task.started`
- `task.failed`
- `task.completed`
- `task.artifact_created`

### Memory

- `memory.before_recall`
- `memory.after_recall`
- `memory.before_remember`
- `memory.after_remember`

### Scheduler

- `job.scheduled`
- `job.triggered`
- `job.retried`
- `job.dead_lettered`

### UI

- `ui.page_registered`
- `ui.panel_registered`
- `ui.page_mounted`
- `ui.panel_mounted`

## 3. Hook 载荷建议

所有 Hook 事件都应包含：

- `event_id`
- `event_name`
- `occurred_at`
- `owner`
- `run_id`
- `thread_id`
- `payload`

## 4. 扩展行为

System extension 可以：

- 观察事件
- 增补元数据
- 触发外部 side effect
- 注册新的页面、面板和命令
