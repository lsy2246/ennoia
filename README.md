# Ennoia

`Ennoia` 是一个面向单操作者、多 Agent 的本地 AI Web 工作台。

本次已按“完全重构、不考虑兼容”的方向收敛为新的统一产品结构：

- `工作台`
- `Agents`
- `技能`
- `API 上游渠道`
- `扩展`
- `日志`
- `记忆`
- `任务`
- `设置`

## 当前产品结构

### 工作台

- 私聊和群聊统一从一个入口创建
- 选择 `1` 个 Agent = `direct`
- 选择 `2+` 个 Agent = `group`
- 每个会话都有唯一 `conversation_id`
- 聊天输入框支持 `@agent_id`
- 不再保留“目标”输入框

### Agents

- 支持完整 CRUD
- 当前产品模型字段收敛为：
  - `id`
  - `display_name`
  - `description`
  - `system_prompt`
  - `provider_id`（产品语义为“API 上游渠道”）
  - `model_id`
  - `reasoning_effort`
  - `skills`
  - `enabled`

### 技能

- 技能是 Agent 可引用的能力包
- 技能与扩展完全分离
- 技能不承载插件挂载、运行时面板或扩展目录扫描

### API 上游渠道

- Agent 绑定的是具体渠道实例，而不是实现类型字符串
- 创建渠道时选择接口类型
- 默认可用 `OpenAI` 类型
- 扩展可以贡献新的接口类型实现

### 扩展

- 扩展是系统插件，不是技能
- 工作台展示真实状态、诊断、贡献能力、重载、日志
- 扩展以扩展包为分区标准

### 记忆

- 记忆通过数据库与可视化页面管理
- 支持按 owner、kind、namespace、stability、status 检索
- 支持 recall 与 review

### 任务

- 统一承载计划任务
- 支持两类：
  - `AI 任务`
  - `命令任务`
- 任务 payload 可承载：
  - 超时
  - 运行后删除
  - 完成后投递到某个会话

### 日志

- 前后端日志统一进入一个日志流
- 扩展事件和运行摘要也并入统一日志流
- 支持按关键字、等级、来源筛选

### 设置

- 运行时配置以表单编辑，不再直接暴露 JSON
- 支持多套工作台配色
- 保留语言、主题、时区等实例级偏好

## 技术栈

- 后端：Rust、Tokio、Axum、SQLx、Serde、TOML
- 数据库：SQLite
- 前端：React、Vite、TanStack Router、Zustand
- 包管理：`bun`
- 发布目标：`一个 npm 包 + Ennoia 运行目录`

## 核心模块

- `crates/kernel`：共享领域模型与系统配置协议
- `crates/memory`：上下文、记忆、回顾
- `crates/orchestrator`：消息到 run/task 的编排
- `crates/scheduler`：计划任务与后台推进
- `crates/extension-host`：扩展运行时、热刷新、诊断
- `crates/server`：HTTP API 与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web/apps/shell`：Ennoia Web 工作台
- `web/packages/api-client`：前端统一 API 访问层

## 启动方式

### 本地开发

```bash
cargo run -p ennoia-cli -- dev
```

默认地址：

- Web：`http://127.0.0.1:5173`
- API：`http://127.0.0.1:3710`

### 初始化运行目录

```bash
cargo run -p ennoia-cli -- init
```

初始化会自动创建并生成可用目录：

- `config/`
- `agents/`
- `extensions/`
- `packages/extensions/`
- `data/`
- `logs/`

初始化不会创建默认 Agent、默认扩展包、`workspace/`、`spaces/`、`policies/`、`global/`；`agents/<agent_id>/` 等私有运行目录只在创建或运行对应 Agent 时懒创建。

## 验证命令

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- `bun run --cwd web/apps/shell typecheck`
- `bun run --cwd web/apps/shell build`

当前 `web/apps/shell` 还没有单独的 `lint` 脚本。

## 文档入口

- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
