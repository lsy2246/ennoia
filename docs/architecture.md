# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI 工作台。系统核心只负责运行时骨架：配置、路径、系统日志、扩展生命周期、能力路由和扩展代理；具体业务能力通过内置实现或扩展实现接入。

## 总体分层

```text
Web
  -> API Client
    -> Server
      -> Kernel / Contract / Paths / Observability
      -> Extension Host
      -> System Log
    -> Behavior Router
      -> Workflow Extension Backend
    -> Memory Router
      -> Journal Builtin Memory
      -> Memory Extension Backend
```

## 核心边界

- `Kernel`：定义系统级配置、扩展 manifest、共享运行时模型和能力声明结构。
- `Contract`：定义跨边界 DTO；当前包含 `behavior` 与 `memory` 协议响应结构。
- `Paths`：统一解析运行目录，所有运行时文件位置都通过 `RuntimePaths` 推导。
- `Extension Host`：负责扩展扫描、attach / detach、reload / restart、诊断和后端进程托管。
- `Server`：负责 HTTP API、配置读写、能力选择、能力代理、系统日志和系统内置组件装配。

## 行为层

- 行为层解决“系统如何触发一次执行”。
- 系统只认识 `behavior` 能力协议，不依赖具体 `workflow` 实现。
- `workflow` 是一个内置扩展实现，声明 `contributes.behaviors`，通过统一入口对外暴露运行能力。
- 当前系统行为入口包括：
  - `GET /api/v1/behaviors`
  - `GET /api/v1/behaviors/active`
  - `GET /api/v1/behavior/status`
  - `ANY /api/v1/behavior/{*path}`
- `config/behavior.toml` 只负责选择当前激活的行为实现：`active_extension` 与 `active_behavior`。

## 记忆层

- 记忆层解决“系统从哪里读写会话、工作区和记忆数据”。
- 系统只负责记忆能力路由，不实现复杂记忆内部逻辑。
- `journal` 与 `memory` 都属于记忆层实现，只是能力强度不同：
  - `journal`：系统内置、文件机制、偏轻量的记忆实现。
  - `memory`：内置扩展、完整记忆系统实现。
- 多种记忆实现可以并存；是否启用、优先读哪个、优先工作区用哪个，由 `config/memory.toml` 决定。
- 当前系统记忆入口包括：
  - `GET /api/v1/memories`
  - `GET /api/v1/memories/active`
  - `GET /api/v1/memories/{memory_id}/status`
  - `ANY /api/v1/memory/{memory_id}/{*path}`
  - `ANY /api/v1/memory/active/{*path}`
- Conversation / message / handoff / history 这些具体数据组织属于记忆层内部责任，不属于系统日志。

## 系统日志

- 系统日志是系统组件自己的观测层，不属于记忆层。
- 系统日志记录系统级事件，例如：宿主启动、扩展 attach / reload / restart、行为路由失败、记忆路由失败、扩展代理失败等。
- 系统日志使用独立 SQLite，文件位于 `data/system/sqlite/system-log.db`。
- 系统日志通过以下接口暴露：
  - `GET /api/v1/system/logs`
  - `GET /api/v1/system/logs/{log_id}`
- 系统日志不承载记忆层 history，也不作为 `memory` / `journal` 的底层存储依赖。

## Hook 边界

- Hook 保留为系统事件分发机制，但不再承担行为执行主通道。
- 行为层通过 `behavior` 能力协议直接接入。
- 记忆层通过 `memory` 能力协议直接接入。
- 系统只在确实需要广播系统事件时才使用 Hook，不把行为和记忆重新耦合回 Hook。

## 扩展能力模型

- 扩展可声明多类贡献：`pages`、`panels`、`themes`、`locales`、`commands`、`providers`、`behaviors`、`memories`、`hooks`。
- UI 工作台读取扩展快照时，同时获得 `behaviors` 与 `memories` 能力清单。
- `workflow` 依赖系统定义的 behavior SPI；`memory` 扩展依赖系统定义的 memory SPI；系统不反向依赖具体扩展。

## 存储划分

- 系统级配置：`~/.ennoia/config/*.toml`
- 系统级日志：`~/.ennoia/data/system/sqlite/system-log.db`
- `journal` 内置记忆：`~/.ennoia/data/journal/`
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`
- 核心不维护主业务总库；行为运行数据和完整记忆数据都放在各自实现边界内。
