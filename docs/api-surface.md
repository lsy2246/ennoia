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
- `GET /api/runtime/server-config`
- `PUT /api/runtime/server-config`

## Agent / Skill / 模型接入

- `GET /api/agents`
- `POST /api/agents`
- `GET /api/agents/{agent_id}`
- `PUT /api/agents/{agent_id}`
- `DELETE /api/agents/{agent_id}`

约定：

- `GET /api/agents`、`POST /api/agents`、`GET /api/agents/{agent_id}`、`PUT /api/agents/{agent_id}` 返回或接收的 Agent 主模型里同时包含 `permission_profile` 与 `execution_environment` 字段；当前 `permission_profile` 固定使用 `mode + entries[]`，其中 `entries[]` 为 `{ match, value }` 命令匹配条目，`execution_environment` 固定使用 `sandbox_enabled`。

- `GET /api/skills`
- `POST /api/skills`
- `GET /api/skills/{skill_id}`
- `PUT /api/skills/{skill_id}`
- `DELETE /api/skills/{skill_id}`

- `GET /api/model-endpoints`
- `POST /api/model-endpoints`
- `GET /api/model-endpoints/{model_endpoint_id}`
- `PUT /api/model-endpoints/{model_endpoint_id}`
- `DELETE /api/model-endpoints/{model_endpoint_id}`

## Extension

- `GET /api/extensions`
- `GET /api/extensions/runtime`
- `GET /api/extensions/events`
- `GET /api/extensions/events/stream`
- `GET /api/extensions/registry`
- `GET /api/extensions/pages`
- `GET /api/extensions/panels`
- `GET /api/extensions/{extension_id}/ui/module`
- `GET /api/extensions/{extension_id}/ui/assets/{*asset_path}`
- `GET /api/extensions/commands`
- `GET /api/extensions/providers`
- `GET /api/extensions/behaviors`
- `GET /api/extensions/memories`
- `GET /api/extensions/hooks`
- `GET /api/extensions/actions`
- `GET /api/extensions/schedule-actions`
- `GET /api/extensions/{extension_id}`
- `GET /api/extensions/{extension_id}/settings`
- `PUT /api/extensions/{extension_id}/settings`
- `GET /api/extensions/{extension_id}/diagnostics`
- `GET /api/extensions/{extension_id}/ui/module`
- `GET /api/extensions/{extension_id}/themes/{theme_id}/stylesheet`
- `GET /api/extensions/{extension_id}/logs`
- `POST /api/extensions/{extension_id}/rpc/{*method}`
- `POST /api/extensions/providers/{provider_kind}/{method}`
- `PUT /api/extensions/{extension_id}/enabled`
- `POST /api/extensions/{extension_id}/reload`
- `POST /api/extensions/{extension_id}/restart`
- `POST /api/extensions/attach`
- `DELETE /api/extensions/attach/{extension_id}`

`GET /api/extensions/{extension_id}/settings` 返回扩展当前生效的扩展级配置；宿主会把 manifest 默认值和扩展私有目录中的已保存值合并后返回。

`PUT /api/extensions/{extension_id}/settings` 接收扩展声明过的配置字段值，宿主会按 manifest 做键、类型、必填项和 `select` 值校验，然后写入 `~/.ennoia/config/extensions/{extension_id}.toml`。

## Action Runtime

- `GET /api/actions`
- `POST /api/actions/{action}`

动作运行时返回系统动作键下挂载的规则列表。每条规则包含扩展、能力、阶段、优先级、结果收敛方式和条件。

`POST /api/actions/{action}` 是产品动作统一入口。请求体默认直接传动作参数；宿主内部 Worker 也可以传 `{ params, context }` 包装体，把 `permission_actor` 等运行时上下文一起送进动作管道。前端不再直接调用 `/api/conversations/*`、`/api/memory/*`、`/api/runs/*` 这类核心包装 REST，而是提交动作键与参数，例如：

- `conversation.list`
- `conversation.create`
- `conversation.get`
- `conversation.delete`
- `message.append`
- `lane.list`
- `branch.list`
- `branch.create`
- `branch.switch`
- `branch.update`
- `branch.delete`
- `memory.workspace.get`
- `memory.entry.list`
- `memory.episode.list`
- `memory.ingest`
- `memory.query`
- `memory.review`
- `memory.build_context`
- `run.create`
- `run.get`
- `run.list`
- `task.list`
- `artifact.list`

## Runtime Bridge

- `GET /api/operations`
- `POST /api/runtime/operations/{operation}`
- `POST /api/extensions/providers/{provider_kind}/{method}`

宿主只保留中立 runtime bridge，不再把 workflow / memory / conversation 的产品编排写死在核心里。

- `runtime/operations/{operation}`：当前 Agent-facing runtime operation 只保留 `command.exec`。读取文件、写入文件和网络访问不再作为独立内置 operation 提供；需要这些能力时统一通过命令完成，并在入口处统一走 permission + sandbox。
- `GET /api/operations`：按 `conversation_id`、`run_id`、`message_id` 查询宿主持久化的 operation 快照，供会话页直接渲染运行态、阻塞态和恢复态。
- `extensions/providers/{provider_kind}/{method}`：供扩展按 provider kind 调用上游 runtime；当 `context.permission_actor` 存在且 `method == generate` 时，宿主会在真正发起上游请求前统一执行 `provider.generate` 权限裁决。

### Conversation 约定

- `agent_ids.len() == 1` 时创建 `direct`
- `agent_ids.len() >= 2` 时创建 `group`
- 消息可附带 `addressed_agents`

### 会话流

核心不再提供 `/api/conversations/{conversation_id}/stream` 聚合流。会话页首屏、分支切换、运行状态和审批状态统一由前端调用通用动作接口与权限接口自行组装快照。

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

`GET /api/schedule-actions` 仍保留给扩展声明定时模板；定时器主模型不再依赖它。

Scheduler 只保存计划并触发执行器。当前触发器支持 `once`、`interval` 和带外部 `next_run_at` 的 `cron`。

执行模型包括：

- `command`：直接在本机 shell 中运行命令，适合脚本和本地自动化。
- `agent`：触发一个指定 Agent 的编排运行；可选通过 `agent.context.conversation_id` 指定运行参考会话，不指定时独立运行。
- `delivery.conversation_id`：可选；把结果作为系统消息投递到某个会话。
- `delivery.lane_id`：可选；在目标会话里进一步投递到指定 lane。
- `delivery.content_mode`：可选；控制投递完整结果、摘要或最终结论。
- `retry`：控制失败重试次数和重试间隔。
- `history`：保留最近运行记录，包括状态、错误和投递结果。

`command` 定时器示例：

```json
{
  "name": "前端构建",
  "trigger": {
    "kind": "interval",
    "every_seconds": 3600
  },
  "executor": {
    "kind": "agent",
    "agent": {
      "agent_id": "operator",
      "prompt": "整理今天的待办并产出晨会提醒",
      "model_id": "gpt-5.5",
      "max_turns": 6,
      "context": {
        "conversation_id": "conv-daily"
      }
    }
  },
  "delivery": {
    "conversation_id": "conv-123",
    "lane_id": "lane-default",
    "content_mode": "summary"
  },
  "retry": {
    "max_attempts": 2,
    "backoff_seconds": 30
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
- `GET /api/logs/overview`
- `GET /api/logs/entries`
- `GET /api/logs/entries/stream`
- `GET /api/logs/entries/{log_id}`
- `GET /api/logs/traces`
- `GET /api/logs/traces/{trace_id}`

### 日志筛选参数

`GET /api/logs` 支持：

- `limit`
- `q`
- `level`
- `source`

`GET /api/logs/entries` 支持：

- `event`
- `level`
- `component`
- `source_kind`
- `source_id`
- `request_id`
- `trace_id`
- `cursor`
- `limit`

`GET /api/logs/traces` 支持：

- `request_id`
- `component`
- `kind`
- `source_kind`
- `source_id`
- `limit`

### 日志流

`GET /api/logs/entries/stream` 返回 SSE：

- 事件 `logs.delta`：增量日志、增量链路和最新 overview
- 事件 `logs.error`：流读取失败时的错误说明

## Agent 权限

- `GET /api/permissions/policies`
- `GET /api/permissions/events`
- `GET /api/permissions/approvals`
- `POST /api/permissions/approvals/{approval_id}/resolve`

权限 API 只服务系统级 Agent 裁决，不暴露给扩展自行放权。约定如下：

- `GET /api/permissions/policies`：返回当前 Agent 列表对应的策略摘要，便于前端展示模式和编译后 allow / ask / deny 规则数量。
- `GET /api/permissions/events`：返回最近权限事件，支持 `agent_id`、`action`、`decision`、`limit`。
- `GET /api/permissions/approvals`：返回审批记录，支持 `agent_id`、`status`、`limit`。
- `POST /api/permissions/approvals/{approval_id}/resolve`：处理待审批请求，`resolution` 取值固定为 `allow_once`、`allow_reply_action`、`allow_conversation_all`、`deny`。
