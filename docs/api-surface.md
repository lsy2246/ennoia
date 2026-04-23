# Ennoia API 边界

## 系统与引导

- `GET /health`
- `GET /api/overview`
- `GET /api/bootstrap/status`
- `POST /api/bootstrap/setup`

## 系统运行配置

- `GET /api/ui/runtime`
- `GET /api/ui/messages`
- `GET /api/runtime/profile`
- `PUT /api/runtime/profile`
- `GET /api/runtime/preferences`
- `PUT /api/runtime/preferences`
- `GET /api/runtime/app-config`
- `PUT /api/runtime/app-config`
- `GET /api/runtime/server-config`
- `PUT /api/runtime/server-config`

## Agent / Skill / API 上游渠道

- `GET /api/agents`
- `POST /api/agents`
- `GET /api/agents/{agent_id}`
- `PUT /api/agents/{agent_id}`
- `DELETE /api/agents/{agent_id}`

- `GET /api/skills`
- `POST /api/skills`
- `GET /api/skills/{skill_id}`
- `PUT /api/skills/{skill_id}`
- `DELETE /api/skills/{skill_id}`

- `GET /api/providers`
- `POST /api/providers`
- `GET /api/providers/{provider_id}`
- `GET /api/providers/{provider_id}/models`
- `PUT /api/providers/{provider_id}`
- `DELETE /api/providers/{provider_id}`

## Extension

- `GET /api/extensions`
- `GET /api/extensions/runtime`
- `GET /api/extensions/events`
- `GET /api/extensions/events/stream`
- `GET /api/extensions/registry`
- `GET /api/extensions/pages`
- `GET /api/extensions/panels`
- `GET /api/extensions/commands`
- `GET /api/extensions/providers`
- `GET /api/extensions/hooks`
- `GET /api/extensions/{extension_id}`
- `GET /api/extensions/{extension_id}/diagnostics`
- `GET /api/extensions/{extension_id}/ui/module`
- `GET /api/extensions/{extension_id}/themes/{theme_id}/stylesheet`
- `GET /api/extensions/{extension_id}/logs`
- `POST /api/extensions/{extension_id}/rpc/{method}`
- `PUT /api/extensions/{extension_id}/enabled`
- `POST /api/extensions/{extension_id}/reload`
- `POST /api/extensions/{extension_id}/restart`
- `POST /api/extensions/attach`
- `DELETE /api/extensions/attach/{extension_id}`

## Journal / Conversation

- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/{conversation_id}`
- `DELETE /api/conversations/{conversation_id}`
- `GET /api/conversations/{conversation_id}/messages`
- `POST /api/conversations/{conversation_id}/messages`
- `GET /api/conversations/{conversation_id}/lanes`

Journal 默认关闭；`journal.enabled = false` 时，Conversation API 返回 `journal_disabled`，系统不读写 `data/journal/`。

## Memory Capability

- `GET /api/memories`
- `GET /api/memories/active`
- `GET /api/memories/{memory_id}/status`
- `ANY /api/memory/{memory_id}/{*path}`
- `ANY /api/memory/active/{*path}`

Memory 能力通过扩展 Worker RPC 或内置 Journal 分发。Memory 扩展拥有自己的私有存储；它与 Journal 是并存机制，不通过 Journal 同步。

### Conversation 约定

- `agent_ids.len() == 1` 时创建 `direct`
- `agent_ids.len() >= 2` 时创建 `group`
- 消息可附带 `addressed_agents`

## Workflow / Behavior Capability

- `GET /api/behaviors`
- `GET /api/behaviors/active`
- `GET /api/behavior/status`
- `ANY /api/behavior/{*path}`

主系统不暴露 workflow 内部编排 API。运行编排、任务、产物索引、stage、decision 和 gate 都属于 workflow 扩展私有能力，统一由 Worker RPC 承载。

## 日志

- `GET /api/logs`
- `POST /api/logs/frontend`

### 日志筛选参数

`GET /api/logs` 支持：

- `limit`
- `q`
- `level`
- `source`
