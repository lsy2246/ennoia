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
- `GET /api/extensions/behaviors`
- `GET /api/extensions/memories`
- `GET /api/extensions/hooks`
- `GET /api/extensions/interfaces`
- `GET /api/extensions/schedule-actions`
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

## Interface Binding

- `GET /api/interfaces`
- `GET /api/interfaces/bindings`
- `PUT /api/interfaces/bindings`

接口绑定用于把系统稳定动作键绑定到扩展 Worker RPC 方法。没有显式绑定且只有一个实现时自动使用该实现；多个实现时要求用户或前端写入绑定。

## Conversation

- `GET /api/conversations`
- `POST /api/conversations`
- `GET /api/conversations/{conversation_id}`
- `DELETE /api/conversations/{conversation_id}`
- `GET /api/conversations/{conversation_id}/messages`
- `POST /api/conversations/{conversation_id}/messages`
- `GET /api/conversations/{conversation_id}/lanes`

Conversation API 是稳定产品入口，实际由以下接口键解析到扩展 Worker：

- `conversation.list`
- `conversation.create`
- `conversation.get`
- `conversation.delete`
- `lane.list_by_conversation`
- `message.list`
- `message.append_user`
- `message.append_agent`

## Memory Capability

- `GET /api/memories`
- `GET /api/memories/active`
- `GET /api/memories/{memory_id}/status`
- `ANY /api/memory/{memory_id}/{*path}`
- `ANY /api/memory/active/{*path}`

Memory 能力通过扩展 Worker RPC 分发。Memory 扩展拥有自己的私有存储；核心不再提供内置 Journal 存储。
`/api/memory/active/*` 仅在当前只有一个启用的 Memory 实现时自动选择；存在多个实现时调用方应使用显式 `{memory_id}` 或稳定接口绑定。

### Conversation 约定

- `agent_ids.len() == 1` 时创建 `direct`
- `agent_ids.len() >= 2` 时创建 `group`
- 消息可附带 `addressed_agents`

## Run / Task / Artifact

- `POST /api/runs`
- `GET /api/runs/{run_id}`
- `GET /api/conversations/{conversation_id}/runs`
- `GET /api/runs/{run_id}/tasks`
- `GET /api/runs/{run_id}/artifacts`

运行相关 API 是稳定产品入口，实际由以下接口键解析到扩展 Worker：

- `run.create`
- `run.get`
- `run.list_by_conversation`
- `task.list_by_run`
- `artifact.list_by_run`

## Schedule

- `GET /api/schedule-actions`
- `GET /api/schedules`
- `POST /api/schedules`
- `GET /api/schedules/{schedule_id}`
- `PUT /api/schedules/{schedule_id}`
- `DELETE /api/schedules/{schedule_id}`
- `POST /api/schedules/{schedule_id}/run`
- `POST /api/schedules/{schedule_id}/pause`
- `POST /api/schedules/{schedule_id}/resume`

Scheduler 只保存计划并触发目标。当前触发器支持 `once`、`interval` 和带外部 `next_run_at` 的 `cron`。

目标支持两种：

- `extension`：调用扩展声明的 `schedule_actions`，适合把定时任务交给 AI / Workflow 执行。
- `command`：直接在本机 shell 中运行命令，适合脚本和本地自动化。

`command` target 示例：

```json
{
  "target": {
    "kind": "command",
    "command": {
      "command": "bun run --cwd web build",
      "cwd": "C:/Users/Administrator/Desktop/code/ennoia",
      "timeout_ms": 120000
    }
  }
}
```

## Workflow / Behavior Capability

- `GET /api/behaviors`
- `GET /api/behaviors/active`
- `GET /api/behavior/status`
- `ANY /api/behavior/{*path}`

Behavior 能力入口保留用于兼容和扩展自有 API。系统级运行入口优先使用更细粒度的 run/task/artifact 接口绑定。
`/api/behavior/*` 不再读取系统级 behavior 配置；存在多个 Behavior 实现时调用方应使用稳定接口绑定或显式扩展 RPC。

## 日志

- `GET /api/logs`
- `POST /api/logs/frontend`

### 日志筛选参数

`GET /api/logs` 支持：

- `limit`
- `q`
- `level`
- `source`
