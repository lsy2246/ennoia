# Ennoia 架构总览

## 目标

`Ennoia` 是单操作者、多 Agent 的本地 AI Web 工作台。当前代码以 `Conversation` 作为协作主实体，产品文案可以称为“会话”，但后端、数据库、API 与 SDK 统一使用 `conversation` 命名。

## 分层

```text
Web
  -> API Client
    -> Server
      -> Kernel
      -> Memory
      -> Runtime
      -> Scheduler
      -> Extension Host
      -> SQLite
```

## 领域边界

- `Kernel` 定义跨模块共享模型：`ConversationSpec`、`LaneSpec`、`MessageSpec`、`RunSpec`、`AgentConfig`、`SkillConfig`、`ProviderConfig`。
- `Server` 负责 API 路由、运行时装配、数据库初始化与迁移执行入口。
- `Memory` 负责 memory 协议、episode/context 模型、receipt 模型与 sqlite 存储实现。
- `Runtime` 负责 stage machine、gate pipeline、runtime store 协议与审计实现。
- `Scheduler` 负责 job 协议、worker、sqlite store 与调度推进。
- `Extension Host` 负责扩展扫描、attach/detach、运行状态、诊断和贡献注册。
- `Web` 负责工作台 UI、Dockview 多实例视图、表单化设置和扩展贡献挂载。

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
- 扩展与技能的安装登记统一放在 `config/extensions.toml` 与 `config/skills.toml`，真实包内容放在 `extensions/*` 与 `skills/*`。

## 数据库

- `assets/db.sql` 承载当前完整数据库基线，用于新库初始化。
- `assets/migrations/` 承载后续数据库结构变更脚本。
- 新库初始化执行 `db.sql`，已有库在存在新增 migration 时执行迁移。
- 运行时 CRUD 默认使用 SeaQuery 构造 SQL，migration 和少数执行器边界允许原始 SQL。
