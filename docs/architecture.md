# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI 工作台。系统核心只负责运行时骨架：配置、路径、系统日志、扩展生命周期、能力路由和 Worker RPC；具体业务能力通过内置实现或扩展能力包接入。

## 总体分层

```text
Web
  -> API Client
    -> Server
      -> Kernel / Contract / Paths / Observability
      -> Extension Host / Wasm Worker
      -> System Log
      -> Interface Router
        -> Memory Worker / Workflow Worker / Other Extension Workers
      -> Scheduler
        -> Schedule Action Worker RPC
```

## 核心边界

- `Kernel`：定义系统级配置、扩展 manifest、共享运行时模型和能力声明结构。
- `Contract`：定义跨边界 DTO；当前保留 `behavior` 与 `memory` 兼容协议响应结构。
- `Paths`：统一解析运行目录，所有运行时文件位置都通过 `RuntimePaths` 推导。
- `Extension Host`：负责扩展扫描、attach / detach、reload / restart、诊断、Worker 解析和 Worker RPC 分发。
- `Server`：负责 HTTP API、配置读写、接口绑定、定时调度、Worker RPC 路由、系统日志和系统内置组件装配。

## 细粒度接口层

- 系统用稳定 `/api/...` 表达产品动作，例如会话列表、创建会话、写消息、创建运行、读取任务。
- 每个产品动作映射为一个接口键，例如 `conversation.list`、`message.append_user`、`run.create`、`task.list_by_run`。
- 扩展通过 manifest 的 `contributes.interfaces` 声明实现，实际执行统一进入扩展 Wasm Worker RPC。
- `config/interfaces.toml` 只保存必要的显式绑定；没有显式绑定且只有一个实现时自动绑定，有多个实现时返回冲突。
- 当前系统接口管理入口包括：
  - `GET /api/extensions/interfaces`
  - `GET /api/interfaces`
  - `GET /api/interfaces/bindings`
  - `PUT /api/interfaces/bindings`

## 会话与记忆边界

- 核心不再内置 `journal` 文件记录层。
- `/api/conversations`、`/api/conversations/{id}/messages` 等稳定入口通过接口层路由，不直接绑定某个 memory 大能力。
- 内置 `memory` 扩展当前声明会话、线路、消息和记忆接口；其他扩展可以替换其中任意一个细粒度接口。
- `GET /api/memories`、`ANY /api/memory/...` 作为兼容能力入口保留，用于访问扩展自己的 memory API。
- Conversation、Message、Memory Graph、Review 等业务数据组织属于扩展私有责任，不属于系统日志。

## 运行与定时边界

- `workflow` 是一个内置扩展实现，声明 run/task/artifact 接口和 `workflow.run` 定时动作。
- `/api/runs`、`/api/runs/{id}/tasks`、`/api/conversations/{id}/runs` 等稳定入口通过接口层路由到扩展。
- 系统 scheduler 只负责保存计划、计算到期、串行触发和记录最近一次执行状态。
- 定时目标支持两类：
  - `extension`：调用扩展的 `contributes.schedule_actions`，用于“交给 AI / Workflow 执行”。
  - `command`：直接在本机 shell 中运行命令，用于脚本和本地自动化。
- `command` 目标支持 `command`、`cwd`、`timeout_ms`，并记录 stdout / stderr 摘要；业务风险由本机操作者自行控制。
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

## 系统日志

- 系统日志是系统组件自己的观测层，不属于记忆层。
- 系统日志记录系统级事件，例如：宿主启动、扩展 attach / reload / restart、行为路由失败、记忆路由失败、Worker RPC 失败等。
- 系统日志使用独立 SQLite，文件位于 `data/system/sqlite/system-log.db`。
- 系统日志通过以下接口暴露：
  - `GET /api/system/logs`
  - `GET /api/system/logs/{log_id}`
- 系统日志不承载会话 history，也不作为扩展业务数据的底层存储依赖。

## Hook 边界

- Hook 保留为系统事件分发机制，但不承担业务执行主通道。
- 接口层完成会话创建、消息追加等动作后，可以广播 `conversation.created`、`conversation.message.created` 等事件。
- 系统只在确实需要广播系统事件时才使用 Hook，不把会话、记忆或编排重新耦合回 Hook。

## 扩展能力模型

- 扩展是能力包，可选声明 `ui` 和 `worker`，并可声明多类贡献：`pages`、`panels`、`themes`、`locales`、`commands`、`providers`、`behaviors`、`memories`、`hooks`、`interfaces`、`schedule_actions`。
- UI 工作台读取扩展快照时，同时获得接口实现和定时动作清单。
- `workflow` 和 `memory` 都只是内置扩展实现；系统依赖接口键和动作 ID，不反向依赖具体扩展。
- 扩展不自行开放端口；Provider、Behavior、Memory、Hook、Interface 和 Schedule Action 的执行统一走宿主 Worker RPC。

## 存储划分

- 系统级配置：`~/.ennoia/config/*.toml`
- 接口绑定：`~/.ennoia/config/interfaces.toml`
- 系统级日志：`~/.ennoia/data/system/sqlite/system-log.db`
- 系统定时计划：`~/.ennoia/data/system/schedules.json`
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`
- 核心不维护主业务总库；会话、运行数据和完整记忆数据都放在各自扩展边界内。
