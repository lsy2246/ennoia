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
- extension registry 与 UI runtime snapshot

### Shell

负责提供：

- 首次欢迎引导
- direct/group 会话工作台
- 工作流与产物视图
- 本地缓存驱动的多语言与多主题切换

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
