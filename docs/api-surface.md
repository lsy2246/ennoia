# Ennoia API 边界

## 系统与引导

- `GET /health`
- `GET /api/v1/overview`
- `GET /api/v1/bootstrap/status`
- `POST /api/v1/bootstrap/setup`

## 系统运行配置

- `GET /api/v1/ui/runtime`
- `GET /api/v1/ui/messages`
- `GET /api/v1/runtime/profile`
- `PUT /api/v1/runtime/profile`
- `GET /api/v1/runtime/preferences`
- `PUT /api/v1/runtime/preferences`
- `GET /api/v1/runtime/app-config`
- `PUT /api/v1/runtime/app-config`
- `GET /api/v1/runtime/server-config`
- `PUT /api/v1/runtime/server-config`

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
- `ANY /api/ext/{extension_id}/{*path}`

## Journal / Conversation

- `GET /api/v1/conversations`
- `POST /api/v1/conversations`
- `GET /api/v1/conversations/{conversation_id}`
- `DELETE /api/v1/conversations/{conversation_id}`
- `GET /api/v1/conversations/{conversation_id}/messages`
- `POST /api/v1/conversations/{conversation_id}/messages`
- `GET /api/v1/conversations/{conversation_id}/lanes`

Journal 默认关闭；`journal.enabled = false` 时，Conversation API 返回 `journal_disabled`，系统不读写 `data/journal/`。

## Memory Extension

- `GET /api/ext/memory/workspace`
- `GET /api/ext/memory/conversations`
- `POST /api/ext/memory/conversations`
- `GET /api/ext/memory/conversations/{conversation_id}`
- `DELETE /api/ext/memory/conversations/{conversation_id}`
- `GET /api/ext/memory/conversations/{conversation_id}/messages`
- `POST /api/ext/memory/conversations/{conversation_id}/messages`
- `GET /api/ext/memory/conversations/{conversation_id}/lanes`
- `GET /api/ext/memory/memories`
- `POST /api/ext/memory/memories/remember`
- `POST /api/ext/memory/memories/recall`
- `POST /api/ext/memory/memories/review`

Memory 扩展拥有自己的 Conversation API 和私有存储；它与 Journal 是并存机制，不通过 Journal 同步。

### Conversation 约定

- `agent_ids.len() == 1` 时创建 `direct`
- `agent_ids.len() >= 2` 时创建 `group`
- 消息可附带 `addressed_agents`

## Workflow Extension

- `GET /api/ext/workflow/health`
- `POST /api/ext/workflow/hooks/conversation-message-created`

主系统不暴露 workflow 内部编排 API。运行编排、任务、产物索引、stage、decision 和 gate 都属于 workflow 扩展私有能力。

## 日志

- `GET /api/v1/logs`
- `POST /api/v1/logs/frontend`

### 日志筛选参数

`GET /api/v1/logs` 支持：

- `limit`
- `q`
- `level`
- `source`
