# Ennoia API 边界

## 1. 目标

Server 首版提供一组稳定、可扩展的系统接口。

## 2. 首版接口

### 系统

- `GET /health`
- `GET /api/v1/overview`

### Extensions

- `GET /api/v1/extensions`
- `GET /api/v1/extensions/registry`
- `GET /api/v1/extensions/pages`
- `GET /api/v1/extensions/panels`

### Agents

- `GET /api/v1/agents`

### Spaces

- `GET /api/v1/spaces`

### Conversations

- `GET /api/v1/threads`
- `GET /api/v1/threads/{thread_id}/messages`
- `GET /api/v1/threads/{thread_id}/runs`
- `POST /api/v1/threads/private/messages`
- `POST /api/v1/threads/space/messages`

### Runs

- `GET /api/v1/runs`
- `GET /api/v1/runs/{run_id}/tasks`
- `GET /api/v1/runs/{run_id}/artifacts`
- `POST /api/v1/runs/private`
- `POST /api/v1/runs/space`

### Tasks

- `GET /api/v1/tasks`

### Artifacts

- `GET /api/v1/artifacts`

### Memory

- `GET /api/v1/memories`

### Jobs

- `GET /api/v1/jobs`
- `POST /api/v1/jobs`

## 3. 会话创建响应

以下会话写入接口返回统一 envelope：

- `thread`
- `message`
- `run`
- `tasks`
- `artifacts`

这让 shell 可以在一次提交后立即刷新线程、任务与产物侧栏，而不需要拼装多个兼容响应。

## 4. WebSocket 方向

后续实时接口建议统一走：

- `/ws`

可承载：

- message stream
- run updates
- task updates
- log stream
- panel data feeds

## 5. 设计原则

- 首版接口围绕系统概览和主链路表达
- 资源命名与领域对象一致
- 前端主壳从概览、注册表和实时流组合界面状态
- `/api/v1/extensions` 维持扁平扩展列表
- `/api/v1/extensions/registry` 提供完整 registry 快照
- `/api/v1/extensions/pages` 与 `/api/v1/extensions/panels` 提供细粒度挂载入口
- `threads/messages/runs/tasks/artifacts/memories` 形成统一的会话事实面
- `POST /api/v1/runs/private` 与 `POST /api/v1/runs/space` 继续提供兼容入口
- `POST /api/v1/threads/private/messages` 与 `POST /api/v1/threads/space/messages` 作为正式 workspace 写入入口
