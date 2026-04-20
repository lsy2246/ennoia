# Ennoia 架构总览

## 目标

`Ennoia` 的当前目标是一套单操作者、多 Agent 的本地工作台。

系统关键约束已经固定：

- 只有一个本地操作者
- 通过欢迎引导建立实例级 `workspace_profile`
- 会话分为 `direct` 与 `group`
- `Space` 负责承载项目上下文与长期协作容器

## 分层

```text
Shell
  -> Server
    -> Kernel
    -> Memory
    -> Orchestrator
    -> Scheduler
    -> Extension Host
```

### Kernel

负责定义共享领域模型：

- `WorkspaceProfile`
- `ConversationSpec`
- `LaneSpec`
- `HandoffSpec`
- `RunSpec / TaskSpec / ArtifactSpec`

### Memory

负责记忆和上下文组装。

当前实现保留原始记录，并围绕 owner、conversation 和 run 组装上下文视图。

### Orchestrator

负责把一条会话消息转换为：

- 一个 `run`
- 若干个 `task`
- 一次阶段流转
- 一组 gate verdict

### Server

负责提供：

- bootstrap 引导
- runtime profile / preferences
- conversation / lane / handoff API
- runs / tasks / artifacts / memories / jobs API
- extension runtime snapshot / attach / reload / diagnostics API

### Shell

负责提供：

- 首次欢迎引导
- direct/group 会话工作台
- 工作流、任务调度、记忆、扩展、Agent、产物、日志视图
- 会话创建/删除与基础治理操作
- 本地缓存驱动的多语言与多主题切换

## 扩展运行时

当前扩展子系统已经抽象为独立的 `Extension Runtime`：

```text
Shell
  -> Server
    -> Extension Runtime
      -> Workspace Registry
      -> Watch Service
      -> Frontend Module Resolver
      -> Backend Runner Manager
      -> Extension Graph Store
    -> Kernel
    -> Memory
    -> Orchestrator
    -> Scheduler
```

当前目标是：

- 让 `workspace extension` 成为一等公民
- 让开发态与发布态共享同一份扩展协议
- 让扩展 registry 支持 generation 级原子切换
- 让 Shell 以运行时快照感知扩展图谱变化

当前仓库已经落地：

- `ennoia.extension.toml` / `manifest.toml` 双入口解析
- `ExtensionRuntimeSnapshot` 统一快照
- `attach / detach / reload / restart / diagnostics` API 与 CLI
- `extensions/events/stream` SSE 事件流与 Shell 自动刷新
- 轮询式运行时刷新与 generation 递增

## 主链路

### 首次启动

1. Shell 读取 `/api/v1/bootstrap/status`
2. 若未初始化，进入欢迎页
3. 提交 `workspace_profile + instance_ui_preferences`
4. Server 落盘并把 bootstrap 标记为已完成

### 一对一会话

1. 创建 `conversation(topology=direct)`
2. 自动创建默认 `lane`
3. 操作者发送消息
4. Orchestrator 创建单 Agent run 与 response task
5. 结果写入 artifact、memory 和 runtime audit

### 多 Agent 会话

1. 创建 `conversation(topology=group)`
2. 一个会话下可有多条 `lane`
3. 每条消息按 lane 参与者生成协作 task 集
4. 跨线信息通过 `handoff` 传递

## 偏好与缓存

- 浏览器缓存负责首屏无闪烁启动
- 服务端 `instance_ui_preferences` 负责实例级同步
- `space_ui_preferences` 负责项目级覆盖
