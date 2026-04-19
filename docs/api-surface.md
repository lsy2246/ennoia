# Ennoia API 边界

## 系统与引导

- `GET /health`
- `GET /api/v1/overview`
- `GET /api/v1/bootstrap/status`
- `POST /api/v1/bootstrap/setup`

## Runtime

- `GET /api/v1/ui/runtime`
- `GET /api/v1/ui/messages`
- `GET /api/v1/runtime/profile`
- `PUT /api/v1/runtime/profile`
- `GET /api/v1/runtime/preferences`
- `PUT /api/v1/runtime/preferences`
- `GET /api/v1/runtime/config`
- `GET /api/v1/runtime/config/snapshot`
- `GET /api/v1/runtime/config/{key}`
- `PUT /api/v1/runtime/config/{key}`
- `GET /api/v1/runtime/config/{key}/history`

`GET /api/v1/ui/messages` 支持：

- `locale`
- `namespaces`

## 目录与扩展

- `GET /api/v1/extensions`
- `GET /api/v1/extensions/registry`
- `GET /api/v1/extensions/pages`
- `GET /api/v1/extensions/panels`
- `GET /api/v1/agents`
- `GET /api/v1/spaces`
- `GET /api/v1/spaces/{space_id}/ui-preferences`
- `PUT /api/v1/spaces/{space_id}/ui-preferences`

## 会话

- `GET /api/v1/conversations`
- `POST /api/v1/conversations`
- `GET /api/v1/conversations/{conversation_id}`
- `GET /api/v1/conversations/{conversation_id}/messages`
- `POST /api/v1/conversations/{conversation_id}/messages`
- `GET /api/v1/conversations/{conversation_id}/runs`
- `GET /api/v1/conversations/{conversation_id}/lanes`
- `GET /api/v1/lanes/{lane_id}/handoffs`
- `POST /api/v1/lanes/{lane_id}/handoffs`

会话消息写入返回统一 envelope：

- `conversation`
- `lane`
- `message`
- `run`
- `tasks`
- `artifacts`

## 执行面

- `GET /api/v1/runs`
- `GET /api/v1/runs/{run_id}/tasks`
- `GET /api/v1/runs/{run_id}/artifacts`
- `GET /api/v1/runs/{run_id}/stages`
- `GET /api/v1/runs/{run_id}/decisions`
- `GET /api/v1/runs/{run_id}/gates`
- `GET /api/v1/tasks`
- `GET /api/v1/artifacts`

## 记忆与作业

- `GET /api/v1/memories`
- `POST /api/v1/memories`
- `POST /api/v1/memories/recall`
- `POST /api/v1/memories/review`
- `GET /api/v1/jobs`
- `POST /api/v1/jobs`
