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

## Conversation

- `GET /api/v1/conversations`
- `POST /api/v1/conversations`
- `GET /api/v1/conversations/{conversation_id}`
- `DELETE /api/v1/conversations/{conversation_id}`
- `GET /api/v1/conversations/{conversation_id}/messages`
- `POST /api/v1/conversations/{conversation_id}/messages`
- `GET /api/v1/conversations/{conversation_id}/runs`
- `GET /api/v1/conversations/{conversation_id}/lanes`
- `GET /api/v1/lanes/{lane_id}/handoffs`
- `POST /api/v1/lanes/{lane_id}/handoffs`

### Conversation 约定

- `agent_ids.len() == 1` 时创建 `direct`
- `agent_ids.len() >= 2` 时创建 `group`
- 消息可附带 `addressed_agents`

## Agent / Skill / API 上游渠道

- `GET /api/v1/agents`
- `POST /api/v1/agents`
- `GET /api/v1/agents/{agent_id}`
- `PUT /api/v1/agents/{agent_id}`
- `DELETE /api/v1/agents/{agent_id}`

- `GET /api/v1/skills`
- `POST /api/v1/skills`
- `GET /api/v1/skills/{skill_id}`
- `PUT /api/v1/skills/{skill_id}`
- `DELETE /api/v1/skills/{skill_id}`

- `GET /api/v1/providers`
- `POST /api/v1/providers`
- `GET /api/v1/providers/{provider_id}`
- `GET /api/v1/providers/{provider_id}/models`
- `PUT /api/v1/providers/{provider_id}`
- `DELETE /api/v1/providers/{provider_id}`

## Extension

- `GET /api/v1/extensions`
- `GET /api/v1/extensions/runtime`
- `GET /api/v1/extensions/events`
- `GET /api/v1/extensions/events/stream`
- `GET /api/v1/extensions/registry`
- `GET /api/v1/extensions/pages`
- `GET /api/v1/extensions/panels`
- `GET /api/v1/extensions/commands`
- `GET /api/v1/extensions/providers`
- `GET /api/v1/extensions/hooks`
- `GET /api/v1/extensions/{extension_id}`
- `GET /api/v1/extensions/{extension_id}/diagnostics`
- `GET /api/v1/extensions/{extension_id}/frontend/module`
- `GET /api/v1/extensions/{extension_id}/themes/{theme_id}/stylesheet`
- `GET /api/v1/extensions/{extension_id}/logs`
- `PUT /api/v1/extensions/{extension_id}/enabled`
- `POST /api/v1/extensions/{extension_id}/reload`
- `POST /api/v1/extensions/{extension_id}/restart`
- `POST /api/v1/extensions/attach`
- `DELETE /api/v1/extensions/attach/{extension_id}`

## Memory

- `GET /api/v1/memories`
- `POST /api/v1/memories`
- `POST /api/v1/memories/recall`
- `POST /api/v1/memories/review`

## 执行与调度

- `GET /api/v1/runs`
- `GET /api/v1/runs/{run_id}/tasks`
- `GET /api/v1/runs/{run_id}/artifacts`
- `GET /api/v1/runs/{run_id}/stages`
- `GET /api/v1/runs/{run_id}/decisions`
- `GET /api/v1/runs/{run_id}/gates`
- `GET /api/v1/tasks`
- `GET /api/v1/artifacts`
- `GET /api/v1/jobs`
- `POST /api/v1/jobs`
- `GET /api/v1/jobs/{job_id}`
- `PUT /api/v1/jobs/{job_id}`
- `DELETE /api/v1/jobs/{job_id}`
- `POST /api/v1/jobs/{job_id}/run`
- `POST /api/v1/jobs/{job_id}/enable`
- `POST /api/v1/jobs/{job_id}/disable`

## 日志

- `GET /api/v1/logs`
- `POST /api/v1/logs/frontend`

### 日志筛选参数

`GET /api/v1/logs` 支持：

- `limit`
- `q`
- `level`
- `source`
