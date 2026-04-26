# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI 工作台。系统核心只负责运行时骨架：配置、路径、Observability、扩展生命周期、能力路由和 Worker RPC；具体业务能力通过内置实现或扩展能力包接入。

## 总体分层

```text
Web
  -> API Client
    -> Server
      -> Kernel / Contract / Paths / Observability
      -> Extension Host / Wasm Worker
      -> Extension Host / Process Worker
      -> Observability Store
      -> Event Bus
      -> Interface Router
        -> Memory Worker / Workflow Worker / Other Extension Workers
      -> Scheduler
        -> Schedule Action Worker RPC
```

## 核心边界

- `Kernel`：定义系统级配置、扩展 manifest、共享运行时模型和能力声明结构。
- `Contract`：定义跨边界 DTO；当前保留 `behavior` 与 `memory` 兼容协议响应结构。
- `Paths`：统一解析运行目录，所有运行时文件位置都通过 `RuntimePaths` 推导。
- `Extension Host`：负责扩展扫描、attach / detach、reload / restart、诊断、Worker 解析和 Worker RPC 分发；Worker 可以是 Wasm，也可以是进程型 stdio RPC。
- `Server`：负责 HTTP API、配置读写、接口绑定、定时调度、Worker RPC 路由、Observability、事件总线和系统内置组件装配。

## 细粒度接口层

- 系统用稳定 `/api/...` 表达产品动作，例如会话列表、创建会话、写消息、创建运行、读取任务。
- 每个产品动作映射为一个接口键，例如 `conversation.list`、`message.append_user`、`run.create`、`task.list_by_run`。
- 扩展通过 manifest 的 `capabilities[].metadata.interface` 声明接口实现，实际执行统一进入扩展 Worker RPC。
- `config/interfaces.toml` 只保存必要的显式绑定；没有显式绑定且只有一个实现时自动绑定，有多个实现时返回冲突。
- 当前系统接口管理入口包括：
  - `GET /api/extensions/interfaces`
  - `GET /api/interfaces`
  - `GET /api/interfaces/bindings`
  - `PUT /api/interfaces/bindings`

## 会话与记忆边界

- 核心不再内置 `journal` 文件记录层。
- `/api/conversations`、`/api/conversations/{id}/messages` 等稳定入口通过接口层路由，不直接绑定某个 memory 大能力。
- 内置 `conversation` 扩展当前声明会话、线路和消息接口；内置 `memory` 扩展只负责记忆、上下文、审查和图谱侧车。
- `memory` 的系统入口固定为 `/api/memory/*`，底层通过 `memory.*` 接口键解析到扩展 Worker。
- `conversation` 不直接调用 `memory`；它只把稳定事件交给宿主事件总线。
- `memory` 不直接读取 `conversation.db`；它只消费宿主持久化的系统事件。
- Conversation、Message、Memory Graph、Review 等业务数据组织属于扩展私有责任，不属于 Observability。

## 运行与定时边界

- `workflow` 是一个内置扩展实现，声明 run/task/artifact 接口，并承接定时器里的 Agent 执行。
- `/api/runs`、`/api/runs/{id}/tasks`、`/api/conversations/{id}/runs` 等稳定入口通过接口层路由到扩展。
- 系统 scheduler 只负责保存计划、计算到期、串行触发、失败重试和记录最近运行历史。
- 定时器支持两类执行方式：
  - `command`：直接在本机 shell 中运行命令，用于脚本和本地自动化。
  - `agent`：触发指定 Agent 的编排运行，底层通过 `run.create` 进入工作流扩展；可独立运行，也可指定某个会话作为运行参考上下文，且与结果投递分开配置。
- 定时器支持可选 `delivery.conversation_id`、`delivery.lane_id` 和 `delivery.content_mode`，可以把完整结果、摘要或最终结论投递到某个会话的指定 lane。
- `command` 执行器支持 `command`、`cwd`、`timeout_ms`，并记录 stdout / stderr 摘要；业务风险由本机操作者自行控制。
- 当前定时入口包括：
  - `GET /api/schedule-actions`
  - `GET /api/schedules`
  - `POST /api/schedules`
  - `GET /api/schedules/{schedule_id}`
  - `PUT /api/schedules/{schedule_id}`
  - `DELETE /api/schedules/{schedule_id}`
  - `POST /api/schedules/{schedule_id}/run`
  - `POST /api/schedules/{schedule_id}/pause`
  - `POST /api/schedules/{schedule_id}/resume`

## Observability

- 宿主内建统一 Observability 子系统，不属于记忆层，也不混入业务主数据。
- Observability 当前统一落到 `data/system/sqlite/observability.db`，内部按表区分 `logs`、`spans` 和 `span_links`。
- `logs` 记录系统级事件，例如：宿主启动、扩展 attach / reload / restart、行为路由失败、Worker RPC 失败等。
- `spans` 记录调用链节点；`span_links` 记录异步关联，避免把所有异步链路都硬塞成父子关系。
- Trace 模型固定使用 `trace_id`、`span_id`、`parent_span_id`、`request_id`、`sampled` 和 `source`。
- 当前先追踪跨边界 span，不追踪每条 SQL：
  - HTTP 入口
  - Interface Router -> Worker RPC
  - Behavior Router -> Worker RPC
  - `/api/extensions/{extension_id}/rpc/{method}`
  - Event Bus publish
  - Event Bus hook delivery
- Worker RPC `context` 会收到 `trace` 字段，扩展可以把它继续透传给自己的内部子流程。
- 当前系统 observability 查询接口包括：
  - `GET /api/observability/overview`
  - `GET /api/observability/logs`
  - `GET /api/observability/logs/{log_id}`
  - `GET /api/observability/traces`
  - `GET /api/observability/traces/{trace_id}`

## Hook 边界

- Hook 保留为扩展订阅系统事件的方式，但事件先进入宿主持久化事件总线，不做同步强耦合调用。
- 接口层完成会话创建、消息追加等动作后，把 `conversation.created`、`conversation.message.created` 等事件写入 `events.db`。
- 事件总线异步把事件投递给已注册 Hook；扩展临时离线不会阻塞会话写入。
- 系统不把会话、记忆或编排重新耦合回 Hook。

## 扩展能力模型

- 扩展 manifest 只保留当前协议，不再声明独立协议版本号。
- 扩展是能力包，可选声明 `ui` 和 `worker`，主声明模型统一为：`resource_types`、`capabilities`、`surfaces`、`locales`、`themes`、`commands`、`subscriptions`。
- `pages`、`panels`、`providers`、`behaviors`、`memories`、`hooks`、`interfaces`、`schedule_actions` 都是运行时派生视图，不再是 manifest 顶层主声明。
- UI 工作台读取扩展快照时，同时获得通用声明和派生视图。
- `workflow` 和 `memory` 都只是内置扩展实现；系统依赖接口键和动作 ID，不反向依赖具体扩展。
- 扩展不自行开放端口；Provider、Behavior、Memory、Hook、Interface 和 Schedule Action 的执行统一走宿主 Worker RPC，Worker 通过 Wasm ABI 或进程 stdio 协议接入。
- 扩展 UI、语言、主题和业务配置归扩展包所有；Web 主壳只按 runtime snapshot 发现并挂载，不在系统前端包中静态注册某个扩展页面或文案。
- 扩展 UI 通过独立 ESM bundle 动态加载；主壳只导入 `/api/extensions/{extension_id}/ui/module` 暴露的模块包装器，再按 mount id 调用扩展自己的 `mount/unmount`。
- 扩展主题通过 `ennoia.theme` 与主壳对接；主壳只消费稳定语义 token 和 dockview token，不把内部 class 结构暴露给扩展。

## 存储划分

- 系统级配置：`~/.ennoia/config/*.toml`
- 接口绑定：`~/.ennoia/config/interfaces.toml`
- 系统级观测：`~/.ennoia/data/system/sqlite/observability.db`
- 系统级事件总线：`~/.ennoia/data/system/sqlite/events.db`
- 系统定时计划：`~/.ennoia/data/system/schedules.json`
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`
- 扩展私有配置：`~/.ennoia/data/extensions/{extension_id}/` 下由扩展自行定义
- 核心不维护主业务总库；会话、运行数据和完整记忆数据都放在各自扩展边界内，例如 `conversation`、`memory`、`workflow` 各自维护自己的数据目录。
