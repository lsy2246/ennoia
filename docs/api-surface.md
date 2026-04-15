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

### Runs

- `GET /api/v1/runs`
- `POST /api/v1/runs/private`
- `POST /api/v1/runs/space`

## 3. WebSocket 方向

后续实时接口建议统一走：

- `/ws`

可承载：

- message stream
- run updates
- task updates
- log stream
- panel data feeds

## 4. 设计原则

- 首版接口围绕系统概览和主链路表达
- 资源命名与领域对象一致
- 前端主壳从概览、注册表和实时流组合界面状态
- `/api/v1/extensions` 维持扁平扩展列表
- `/api/v1/extensions/registry` 提供完整 registry 快照
- `/api/v1/extensions/pages` 与 `/api/v1/extensions/panels` 提供细粒度挂载入口
