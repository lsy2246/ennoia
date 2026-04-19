# Ennoia 架构总览

## 1. 顶层目标

`Ennoia` 是一个 `Space-first` 的 AI 工作台平台。

- `Space`：私聊、群聊、线程和多人协作的统一上下文容器
- `Agent`：能参与对话、执行任务、拥有私有技能和私有工作区的参与者
- `Run`：一次完整的编排执行实例
- `Task`：Run 内的可追踪工作单元
- `Extension`：系统扩展总称
- `Skill`：面向 Agent 与 Task 的能力包

## 2. 系统分层

```text
Web Shell
  -> Server
    -> Kernel
    -> Memory
    -> Orchestrator
    -> Scheduler
    -> Extension Host
```

### kernel

负责定义系统是什么：

- 核心对象模型
- 配置协议
- 扩展 manifest 契约
- 公共枚举、标识、运行时上下文

### memory

负责定义系统如何记住：

- truth memory
- working memory
- context artifact
- active view
- projection
- review workbench
- graph sidecar

### orchestrator

负责定义系统如何工作：

- 消息转 run
- run 转 task
- 计划与门禁
- 多 Agent 编排
- owner 归属和产物定位

### scheduler

负责定义系统如何后台推进：

- cron
- delay
- retry
- maintenance
- wake

### extension-host

负责定义系统如何扩展：

- 扫描扩展目录
- 校验 manifest
- 注册后端 Hook
- 注册前端贡献
- 管理 theme、provider、page、panel 和 command

### server

负责定义系统如何对外提供服务：

- HTTP API
- WebSocket
- 鉴权和会话订阅
- 静态资源
- 主壳注入和扩展注册表输出

### shell

负责定义系统如何呈现：

- 顶部栏
- 侧栏
- 子页面
- 面板
- 命令系统
- 拖拽工作台
- 使用 `Panda CSS` 维护 shell 的 token、布局样式和可复用视觉约束

## 3. 主链路

### 私聊链路

1. 用户向某个 Agent 发消息
2. Server upsert 私聊 thread，并写入 user message
3. Memory 按 owner/thread 召回记忆并组装上下文视图
4. Orchestrator 基于 message 创建 run 与 response task
5. Server 写入 run、task、memory 和 artifact summary
6. Agent 执行 skill / extension / provider，并继续追加消息与产物

### 群聊链路

1. 用户在某个 Space 发消息
2. Server 判断提及策略和参与者，并 upsert Space thread
3. Memory 按 owner/thread 召回共享上下文
4. Orchestrator 生成群聊 run 与 collaboration task 集
5. 各 Agent 在共享 Space 上下文中协作
6. 结果写回消息流、task 状态、memory 和 artifacts

### 后台链路

1. 用户或系统注册定时任务
2. Scheduler 到点触发 job
3. Orchestrator 创建后台 run
4. 执行结果按 owner 归档
5. Shell 的日志页、任务页、通知中心实时可见

## 3.1 Phase 2 当前事实

当前后端已经形成以下正式能力：

- `threads/messages/runs/tasks/artifacts/memories` 进入统一 SQLite 持久化
- `memory` 支持按 `owner/thread/run` 作用域召回
- `orchestrator` 以消息为输入创建正式 run/task 生命周期
- `server` 同时提供正式 conversation API 和兼容旧壳子的 run API

## 3.2 Phase 3 当前事实

当前前端已经形成以下正式能力：

- `web/apps/shell` 提供统一 workspace，串联私聊、群聊、任务侧栏、memory 侧栏和 extension surface
- `web/packages/ui-sdk` 提供 extension page / panel 的共享类型与 slot 归一化助手
- `web/packages/builtins` 提供主壳内建 page / panel 描述，主壳通过 mount 协议消费描述而不是直接内嵌扩展实现
- `web/packages/api-client` 与 `web/packages/contract` 提供统一 API 调用与错误契约
- workspace smoke 与 e2e 已覆盖私聊、群聊、memory、artifacts 与 extension registry 主链路

## 4. 前端形态

前端由一个统一 `Shell` 承载：

- `Shell` 是唯一主壳
- 扩展注册的是 `Shell` 下的子页面和面板
- 子页面挂载在内容区
- 面板挂载在 Dock 区域，支持拖拽、停靠、分栏和恢复

## 5. 运行时事实源

- 数据库：以 `SQLite` 作为首版默认事实源，文件位于 `~/.ennoia/state/sqlite/ennoia.db`
- 文件系统：工作区、扩展安装位、产物、缓存

系统核心命名统一使用 `kernel / memory / orchestrator / scheduler / extension-host / server / shell` 体系。
