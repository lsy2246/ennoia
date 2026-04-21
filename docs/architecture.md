# Ennoia 架构总览

## 目标

`Ennoia` 是一套单操作者、多 Agent 的本地 AI Web 工作台。

当前产品模型约束为：

- `Session` 是一等协作实体
- 会话拓扑只有两种：`direct` / `group`
- `1 Agent = direct`
- `2+ Agents = group`
- `Agent` 是可长期配置的协作者档案
- `API 上游渠道` 是 Agent 访问模型能力的具体渠道实例
- `Skill` 与 `Extension` 严格分离
- `Memory` 通过数据库和可视化界面管理
- Web 工作台采用单导航 + Dockview 多实例视图

## 分层

```text
Web
  -> Server
    -> Kernel
    -> Memory
    -> Orchestrator
    -> Scheduler
    -> Extension Host
```

## Web

Web 是 VS Code 风格的多实例工作台：

- 单一导航体系
- Dockview 资源视图
- Session / Agent / API 上游渠道 / Memory / Extension 等资源可多开
- 根容器 `WorkbenchHost` 不可删除
- 资源视图可在标签页、右侧、下方同时打开

Web 当前一等导航：

- `工作台`
- `Agents`
- `技能`
- `API 上游渠道`
- `扩展`
- `日志`
- `记忆`
- `任务`
- `设置`

## Kernel

Kernel 定义共享领域模型，当前关键模型包括：

- `WorkspaceProfile`
- `ConversationSpec`
- `LaneSpec`
- `MessageSpec`
- `RunSpec / TaskSpec / ArtifactSpec`
- `AgentConfig`
- `SkillConfig`
- `ProviderConfig`
- `MemoryRecord`

## Server

Server 负责提供：

- bootstrap 与实例偏好
- Session API
- Agent / Skill / API 上游渠道 API
- Extension Runtime 状态与控制
- Logs / Memory / Jobs / Runtime Config API

## Session 模型

### Session

- `direct`：只对应一个 Agent
- `group`：对应两个及以上 Agent

### 消息流

一条消息至少包含：

- `conversation_id`
- `lane_id`
- `sender`
- `role`
- `body`
- `mentions`

多 Agent 路由通过两层决定：

- 输入框里的 `@agent_id`
- 请求里的 `addressed_agents`

## Agent / Skill / API 上游渠道 / Extension 边界

### Agent

Agent 只表达协作者身份，不承载 Session 产物目录语义。

关键字段：

- `id`
- `display_name`
- `description`
- `system_prompt`
- `provider_id`
- `model_id`
- `reasoning_effort`
- `skills`
- `enabled`

### Skill

Skill 是能力包目录：

- 可以是全局技能
- 也可以是 Agent 私有技能
- “可发现”与“被某个 Agent 启用”是两层语义

### API 上游渠道

API 上游渠道是具体渠道实例：

- Agent 绑定实例，不绑定实现类型字符串
- 创建渠道时选择接口类型
- 扩展可以贡献新的接口类型实现

### Extension

Extension 是系统插件包：

- 可以贡献导航入口
- 可以贡献资源视图
- 可以贡献面板、主题、语言、命令
- 可以贡献 API 上游渠道接口实现

## 日志与记忆

### 日志

统一日志流聚合：

- 前端日志
- 后端日志
- 扩展事件
- 运行摘要

### 记忆

Memory 系统通过数据库与可视化界面管理：

- list
- recall
- review

不预先设计记忆文件目录结构。
