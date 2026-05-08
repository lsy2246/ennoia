# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI 工作台。系统核心只负责运行时骨架：配置、路径、日志、扩展生命周期、能力路由和 Worker RPC；具体业务能力通过内置实现或扩展接入。

## 总体分层

```text
Web
  -> API Client
    -> Server
      -> Kernel / Contract / Paths / Logs
      -> Extension Host / Wasm Worker
      -> Extension Host / Process Worker
        -> Host Capability Dispatcher
      -> Logs Store
      -> Event Bus
      -> Agent Permission Store
      -> Action Pipeline Runtime
      -> Action Router
        -> Memory Worker / Workflow Worker / Other Extension Workers
      -> Scheduler
        -> Schedule Action Worker RPC
```

## 核心边界

- `Kernel`：定义系统级配置、扩展 manifest、共享运行时模型和能力声明结构。
- `config/server.toml`：系统级唯一宿主配置入口，统一承载 API 绑定地址、前端开发地址、宿主中间件配置、内置工具默认值、provider 默认值、流式轮询节奏、后台循环节奏、调度默认值和开发态 supervisor 参数；CLI、Server、Worker 桥接和 Web dev 都只消费这份配置，不再各自维护运行时常量。
- `Contract`：定义跨边界 DTO；当前保留 `behavior` 与 `memory` 兼容协议响应结构。
- `Paths`：统一解析运行目录，所有运行时文件位置都通过 `RuntimePaths` 推导。
- `Extension Host`：负责扩展扫描、attach / detach、reload / restart、诊断、Worker 解析和 Worker RPC 分发；Worker 可以是 Wasm，也可以是进程型 stdio RPC。对于进程型 Worker，宿主还负责当前 RPC 会话内的 `plugin -> host capability` 反向调用分发。
- `Server`：负责 HTTP API、定时调度、Worker RPC 路由、日志、事件总线和系统内置组件装配。
- `Pipeline Runtime`：负责稳定 action 的规则收集、阶段执行和结果收敛。它不拥有 conversation、memory、workflow 的主数据。
- `Extension Runtime Store`：提供扩展可复用的宿主级轻量状态与记录原语，只负责通用 state/record 持久化，不承担 workflow draft、memory graph、conversation message 等业务语义。

## Agent 权限裁决

- Agent 权限属于系统核心，不做成扩展，也不交给 Wasm / Process Worker 自行裁决。
- 扩展只通过 `capabilities[].metadata.permission` 声明动作、目标类型、风险等级和作用域；最终允许、拒绝、审批都由宿主 `AgentPermissionStore` 决定。
- 当前宿主会在两类入口做权限判断：
  - Action Router：当内部 Agent 以 `permission_actor` 身份调用 `conversation.*`、`memory.*`、`run.*` 等稳定动作时。
  - Provider 调用：Agent 真正发起 `provider.generate` 上游请求前。
- 裁决结果固定为 `allow`、`deny`、`ask`。`ask` 会生成待审批记录；审批通过后只会生成临时 grant，当前支持单次放行、本次回复同类操作放行和当前会话放行。
- 当前产品层权限模型不再区分“网络 / 配置 / 扩展管理”等高风险分类，只保留三件事：命令默认模式、命令规则、路径规则。命令规则决定 `command.exec` 是默认允许还是默认询问；路径规则决定 `fs.read` / `fs.write` 对哪些路径可以直接访问。
- 系统默认只管 Agent 身份，不拦截操作者直接发起的 HTTP 调用。

## Agent 执行环境

- Agent 执行环境与权限系统是两套独立机制。
- 权限系统回答“允不允许”；执行环境回答“在哪里执行”。
- 当前执行环境只保留一个布尔开关：
  - `sandbox_enabled = false`：直接在宿主机运行时执行。
  - `sandbox_enabled = true`：使用原生沙盒语义，模型与内置工具优先只看到 `/workspace`、`/artifacts`、`/tmp` 三个虚拟根。
- 内置 `fs.read`、`fs.write`、`command.exec`、`net.fetch` 都应先经过权限裁决，再进入执行环境层，不直接把宿主机绝对路径暴露给模型。

## 细粒度接口层

- 系统不再为 conversation、memory、workflow 暴露固定产品 REST；产品动作统一通过 `/api/actions/{action}` 进入运行时。
- 每个产品动作映射为一个接口键，例如 `conversation.list`、`message.append`、`run.create`、`task.list`。
- 扩展通过 manifest 的 `capabilities[].metadata.action` 声明动作规则，规则携带 `key`、`phase`、`priority`、`result_mode` 和可选 `when`。
- 同一个动作键可以同时挂多条规则；宿主按阶段和优先级执行，并按 `result_mode` 收敛返回值。
- 当前系统接口管理入口包括：
  - `GET /api/extensions/actions`
  - `GET /api/actions`

## 会话、记忆与组合层边界

- 核心不再内置 `journal` 文件记录层。
- 会话、记忆、运行等产品视图都通过通用 action runtime 和扩展 RPC 组合，不再保留核心包装 REST。
- 内置 `conversation` 扩展当前声明会话、分支、检查点、线路和消息接口；内置 `memory` 扩展只负责记忆、上下文、审查和图谱侧车。
- 动作管道是系统级中立执行层，只负责执行规则、收敛结果和发出事实事件；它不再承接 workflow、memory、provider 或 Agent tool 的产品编排。
- 会话展示层首屏和后续刷新统一由前端通过 action runtime 组装 detail、run 和 approval 快照；核心不再维护会话专属 SSE 聚合面。
- `memory` 只暴露 `memory.*` 动作键，不再保留 `/api/memory/*` 核心包装入口。
- `conversation` 不直接调用 `memory` 或 `workflow`；它只维护会话事实并发出事实事件。
- `memory` 不直接读取 `conversation.db`，也不再镜像保存整段会话消息或 shadow session state。
- `workflow` 不假设自己一定挂在 conversation 上；会话事实是否进入 workflow、何时回写 conversation / memory，由 workflow 扩展自己订阅 `conversation.message.created`、`permission.approval.resolved` 等事件后决定。
- 宿主允许扩展通过通用 `extension.state` / `extension.record` 原语保存跨刷新轻量状态和会话可视记录；这些条目只表达扩展自己的运行事实，不提升为系统业务模型。
- Conversation、Message、Memory Graph、Review 等业务数据组织属于扩展私有责任，不属于日志主数据。

## 运行与定时边界

- `workflow` 是一个内置扩展实现，声明 run/task/artifact 接口，并承接定时器里的 Agent 执行。
- `workflow` 自己负责生成结构化执行计划；`plan` 是执行真相源，`task` 只是从 `plan.steps` 派生出来的展示与执行投影视图，系统核心不再硬编码猜测任务清单。
- `workflow` 相关读取与执行统一通过 `run.*`、`task.*`、`artifact.*` 动作键或扩展 RPC 暴露，不再保留 `/api/runs/*` 核心包装入口。
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

## 日志

- 宿主内建统一日志子系统，不属于记忆层，也不混入业务主数据。
- 日志数据当前统一落到 `data/system/sqlite/logs.db`，内部按表区分 `logs`、`spans` 和 `span_links`。
- `logs` 记录系统级事件，例如：宿主启动、扩展 attach / reload / restart、行为路由失败、Worker RPC 失败等。
- `spans` 记录调用链节点；`span_links` 记录异步关联，避免把所有异步链路都硬塞成父子关系。
- Trace 模型固定使用 `trace_id`、`span_id`、`parent_span_id`、`request_id`、`sampled` 和 `source`。
- 当前先追踪跨边界 span，不追踪每条 SQL：
  - HTTP 入口
  - Action Router -> Worker RPC
  - Behavior Router -> Worker RPC
  - `/api/extensions/{extension_id}/rpc/{method}`
  - Event Bus publish
  - Event Bus hook delivery
- Worker RPC `context` 会收到 `trace` 字段，扩展可以把它继续透传给自己的内部子流程。
- 当前系统日志中心接口包括：
  - `GET /api/logs/overview`
  - `GET /api/logs/entries`
  - `GET /api/logs/entries/stream`
  - `GET /api/logs/entries/{log_id}`
  - `GET /api/logs/traces`
  - `GET /api/logs/traces/{trace_id}`

## Hook 边界

- Hook 保留为扩展订阅系统事件的方式，但事件先进入宿主持久化事件总线，不做同步强耦合调用。
- 动作管道在完成会话创建、消息追加、审批解析等动作后，把 `conversation.created`、`conversation.message.created`、`permission.approval.resolved` 等事件写入 `events.db`。
- 事件总线异步把事件投递给已注册 Hook；扩展临时离线不会阻塞会话写入。
- 系统不要求 memory / workflow 必须通过 Hook 互相耦合；跨域组合统一走事件链和宿主中立 runtime bridge（action、provider、runtime operation、permission）。

## 扩展能力模型

- 扩展 manifest 只保留当前协议，不再声明独立协议版本号。
- 扩展负责系统能力，可选声明 `ui` 和 `worker`，主声明模型统一为：`resource_types`、`capabilities`、`surfaces`、`entrypoints`、`settings`、`locales`、`themes`、`commands`、`subscriptions`。
- `pages`、`panels`、`providers`、`behaviors`、`memories`、`actions`、`hooks`、`schedule_actions` 都是运行时派生视图，不再是 manifest 顶层主声明。
- UI 工作台读取扩展快照时，同时获得通用声明和派生视图。
- `entrypoints` 只表达“用户从哪里进入这个扩展”；入口名称、说明和优先级由扩展自身定义，主壳只负责展示与打开。
- `settings` 表达扩展级配置字段；主壳按声明渲染表单，实际值保存在扩展私有数据目录，不上浮为系统级配置模型。
- `workflow` 和 `memory` 都只是内置扩展实现；系统依赖接口键和动作 ID，不反向依赖具体扩展。
- 扩展不自行开放端口；Provider、Behavior、Memory、Hook、Action 和 Schedule Action 的执行统一走宿主 Worker RPC，Worker 通过 Wasm ABI 或进程 stdio 协议接入。
- 进程型 Worker 不再通过 localhost HTTP 回环访问宿主能力；统一通过 process stdio 控制消息发起平台级 host capability 调用，宿主复用既有 action、provider、runtime operation、extension state / record 与权限链路完成分发。
- Provider 模型发现也不再挂在 `/api/model-endpoints/*/models` 这类产品路由下，而是走宿主提供的通用 provider runtime 代理。
- 扩展 UI、语言、主题和业务配置归扩展自身所有；Web 主壳只按 runtime snapshot 发现并挂载，不在系统前端包中静态注册某个扩展页面或文案。
- 扩展 UI 通过独立 ESM bundle 动态加载；主壳只导入 `/api/extensions/{extension_id}/ui/module` 暴露的模块包装器，再按 mount id 调用扩展自己的 `mount/unmount`。
- 会话时间线同样只提供通用 record mount 槽位；主壳不再硬编码 workflow 专属卡片，任何扩展都可以把自己的 record 以会话附件或独立块渲染出来。
- 扩展主题通过 `ennoia.theme` 与主壳对接；主壳只消费稳定语义 token 和 dockview token，不把内部 class 结构暴露给扩展。
- 扩展默认不进入会话目录；只有显式声明 `conversation.inject` 时，宿主才会把该扩展作为会话可见目录项暴露给模型。进入会话时只注入扩展自身的 `description`、受限资源/能力目录与 `docs` 入口，不自动注入 `docs` 正文。
- 如需参与 Agent 权限裁决，扩展应在 capability metadata 中额外声明 `permission`，例如 `action`、`target_kind`、`scope_kind`；没有声明 `permission` 的 capability 不会自动进入 Agent 权限系统。
- Agent 调用上游模型时，宿主统一构造结构化 `context`，至少包含 `runtime`、`conversation`、`extensions`、`skills` 四块，再由 provider 适配层渲染成模型可见消息；`metadata` 只保留给链路追踪和调试，不承担模型上下文职责。
- 当前模型侧应优先使用 `runtime.workspace_root`、`runtime.artifacts_root` 与 `runtime.temp_root` 这些虚拟根；它们表示 Agent 自己的内部执行视图，不等同于用户项目工作区，也不应默认向用户主动播报宿主机绝对路径。

## Skill 模型

- Skill 不负责实现系统能力；它只描述工具与用法。
- Skill 只保留最小目录元信息与 `docs` 入口；CLI、参数和完整操作流程都放在文档中。
- Skill 可以声明 `keywords` 供宿主做发现和路由，但不会因为这些字段自动把正文塞进每轮会话。
- Skill 如果被 Agent 启用，会和扩展目录一样进入结构化 `context.skills`，只暴露目录元信息与文档入口，不自动展开正文。
- 扩展可以带自己的能力说明文档，但扩展说明不等于 skill；前者回答“系统里这块能力是什么”，后者回答“Agent 怎么使用它”。

## 存储划分

- 系统级配置：`~/.ennoia/config/*.toml`
- 系统级日志：`~/.ennoia/data/system/sqlite/logs.db`
- 系统级事件总线：`~/.ennoia/data/system/sqlite/events.db`
- Agent 权限事件与审批：`~/.ennoia/data/system/sqlite/permissions.db`
- 扩展通用运行态 state/record：`~/.ennoia/data/system/sqlite/extensions.db`
- Agent 基础配置、权限配置与执行环境配置：`~/.ennoia/agents/{agent_id}/agent.toml`
- 系统定时计划：`~/.ennoia/data/system/schedules.json`
- 扩展私有数据：`~/.ennoia/data/extensions/{extension_id}/`
- 扩展私有配置：`~/.ennoia/data/extensions/{extension_id}/` 下由扩展自行定义
- 核心不维护主业务总库；会话、运行数据和完整记忆数据都放在各自扩展边界内，例如 `conversation`、`memory`、`workflow` 各自维护自己的数据目录。
