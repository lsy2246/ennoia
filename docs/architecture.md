# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI Web 工作台。核心定位为极简宿主，只负责配置、路径、日志、扩展生命周期、Hook 派发和扩展代理；业务能力由扩展接入。

## 分层

```text
Web
  -> API Client
    -> Server
      -> Kernel
      -> Extension Host
      -> Config / Paths / Logs
    -> Extension Proxy
      -> Memory Extension Backend
      -> Workflow Extension Backend
```

## 领域边界

- `Kernel` 定义共享协议、配置结构、UI 文本结构和扩展 manifest 模型。
- `Server` 负责 API 路由、TOML 配置文件、journal 文件记录、前端日志、扩展代理与扩展运行时装配。
- `Extension Host` 负责扩展扫描、attach/detach、运行状态、诊断、贡献注册和扩展后端进程托管。
- `Journal` 是系统内置的文件记录层，负责一套 Conversation、Lane、Message 与事件原文，默认关闭，可通过 `server.toml` 开启。
- `Memory` 是内置扩展，源码物理位于 `builtins/extensions/memory/`，负责另一套完整记忆机制，包括 Conversation、Lane、Message、Truth Memory、Context Artifact、Review 与 Graph。
- `Workflow` 是内置扩展，源码物理位于 `builtins/extensions/workflow/`，负责运行编排、任务、stage、decision、gate 与 artifact；定时器规范也归扩展自己实现。
- `Web` 负责工作台 UI、Dockview 多实例视图、文件配置表单和扩展贡献挂载。

## Conversation 模型

- `direct`：一个 Agent。
- `group`：两个及以上 Agent。
- 每个 Conversation 有默认 Lane，可按 Lane 展示消息、handoff、run 和 artifact。
- 消息通过 `mentions` 与请求中的 `addressed_agents` 共同决定目标 Agent。

## Agent / Skill / Provider / Extension

- `Agent` 是长期协作者档案，只表达身份、上游、模型、技能和启用状态。
- `Skill` 是 Agent 可引用的能力包，不承担插件挂载。
- `Provider` 是 API 上游渠道实例，`kind` 表示接口类型，并由系统按 `kind` 自动解析唯一实现扩展；当前 OpenAI 生成 / 对话能力统一收敛为单一 `openai` 接口，`default_model` 表示用户确认后的默认模型。
- `Extension` 是系统插件包，可贡献页面、面板、主题、语言、命令、Hook 和 Provider 实现。
- `Hook` 是扩展 manifest 的一部分，系统只定义事件名、事件 envelope 与触发时机，扩展声明 `event + handler` 并通过 HTTP 回包表达是否处理。
- 扩展页面是可选贡献；声明 `pages[].nav.default_pinned = true` 的页面默认进入导航栏，其余页面可从扩展详情页打开。
- `Memory` 是内置扩展：前端通过 `memory.page` 扩展页挂载，后端通过 `/api/ext/memory/*` 提供独立会话、上下文与记忆能力。
- `Workflow` 是内置扩展：编排实现和运行态产出都留在扩展私有边界。
- 扩展与技能的安装登记统一放在 `config/extensions.toml` 与 `config/skills.toml`，真实包内容放在 `extensions/*` 与 `skills/*`。

## Hook 事件边界

- 核心机制在系统生命周期时机构造 `HookEventEnvelope`，包含 `event`、`occurred_at`、`owner`、`resource` 和 `payload`。
- Extension Host 按 `contributes.hooks[]` 查找订阅扩展，并把 envelope POST 到扩展后端 handler。
- 扩展返回 `HookDispatchResponse`，其中 `handled=true` 表示该扩展接管了本次时机。
- Hook 决定“什么时候触发”，Plugin 决定“触发后做什么”，Extension 负责组织并接入系统。

## 存储

- 系统级配置只走 `~/.ennoia/config/*.toml`。
- Journal 原始记录写入 `~/.ennoia/data/journal/`，包含 conversations、messages 和 events 文件；默认关闭，关闭后系统不读写该目录。
- Memory 扩展私有记录写入 `~/.ennoia/data/extensions/memory/`，与 Journal 并存，启用任一方都不会自动关闭另一方。
- 核心日志写入 `~/.ennoia/logs/`。
- 核心不提供主业务数据库；会话事实由 Journal 文件记录，语义记忆、运行编排、任务、产物索引或调度队列由扩展管理。
- 扩展私有存储位于 `~/.ennoia/data/extensions/{extension_id}/`，例如 `memory.db` 和 `workflow.db`。
