# Ennoia

`Ennoia` 是面向单操作者、多 Agent 的本地 AI Web 工作台。当前仓库处于开发基线阶段，不维护旧数据库或旧目录兼容。

## 产品结构

- 工作台：核心只提供宿主、配置、路径、日志和 Worker RPC；业务能力由扩展提供。
- Agents：维护协作者档案、上游渠道、模型、技能和启用状态。
- Agent 权限：系统级权限策略、审批和事件记录统一由宿主裁决，扩展只声明能力风险，不直接放权。
- 技能：Agent 可引用的工具与用法定义，只保留最小目录元信息与文档入口，和扩展严格分离。
- API 上游渠道：Agent 绑定的具体模型访问实例。
- 扩展：系统插件包，manifest 统一声明 `resource_types`、`capabilities`、`surfaces`、`locales`、`themes`、`commands`、`subscriptions`；如需进入会话目录，再额外声明 `conversation` 规则。宿主把扩展/技能目录整理成结构化 `context` 交给 provider 渲染，不再把它们直接硬拼进自然语言 prompt，也不自动注入文档正文。
- 会话：系统保留稳定 `/api/conversations` 入口，实际读写由 `conversation.*`、`message.*`、`lane.*` 等接口绑定到内置 `conversation` 扩展。
- 记忆：以内置 `memory` 扩展形式提供记忆、上下文、审查和图谱能力；会话事件先进入宿主事件总线，再异步投递给 `memory`。
- 编排：以内置 `workflow` 扩展承载 run、task、artifact；定时器里的 Agent 执行通过编排接口落地。
- 日志：聚合前端日志和扩展运行事件。
- 设置：通过表单直接编辑 `config/server.toml`、`config/profile.toml` 和 `config/preferences/*.toml`。

## 技术栈

- 后端：Rust、Tokio、Axum、Serde、TOML
- 存储：系统配置走 TOML 文件；接口绑定走 `config/interfaces.toml`；定时计划走 `data/system/schedules.json`；扩展按需使用自己的私有存储。
- 前端：React、Vite、TanStack Router、Zustand
- 包管理：`bun`
- 发布目标：一个 npm 包 + `~/.ennoia` 运行目录

## 核心模块

- `crates/kernel`：共享协议、配置和扩展 manifest 模型
- `crates/extension-host`：扩展运行时、热刷新、诊断和 Worker RPC 分发
- `crates/server`：HTTP API、系统配置文件、日志、能力路由与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web`：Ennoia Web 工作台
- `web/packages/api-client`：前端统一 API 访问层
- `builtins/extensions/conversation`：内置会话扩展，声明会话、线路与消息接口
- `builtins/extensions/memory`：内置记忆扩展，声明记忆、上下文、审查与图谱接口
- `builtins/extensions/workflow`：内置编排扩展，声明 run/task/artifact 接口

## 内置能力源码

- `builtins/extensions/*`：官方内置扩展源码
- `builtins/skills/*`：官方内置技能源码
- 初始化会把未卸载的内置包同步到 `~/.ennoia/extensions/*` 与 `~/.ennoia/skills/*`
- 用户安装/启用/卸载登记统一写入 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## 存储边界

- 核心系统配置只走 `~/.ennoia/config/*.toml`。
- 系统接口绑定写入 `~/.ennoia/config/interfaces.toml`；未显式绑定且只有一个实现时自动选中。
- 系统定时计划写入 `~/.ennoia/data/system/schedules.json`，到期后由宿主运行命令或触发 Agent，并可把完整结果、摘要或最终结论投递到会话 / lane。
- 系统事件总线写入 `~/.ennoia/data/system/sqlite/events.db`，用于持久化会话等系统事件及其 Hook 投递状态。
- 系统观测数据写入 `~/.ennoia/data/system/sqlite/observability.db`，统一承载 logs、traces 和 span links。
- Agent 基础配置与权限策略统一写入 `~/.ennoia/agents/{agent_id}/agent.toml`，权限事件与审批写入 `~/.ennoia/data/system/sqlite/permissions.db`。
- 核心日志写入 `~/.ennoia/logs/`。
- 扩展私有数据写入 `~/.ennoia/data/extensions/{extension_id}/`。
- 核心不提供主业务 SQLite，不内建语义记忆、编排、任务或产物索引表。

## 启动方式

安装依赖：

```bash
bun install
```

安装阶段会自动执行 `web` typecheck，并在本机具备 Rust toolchain 时执行 `cargo check --workspace`。

启动开发环境：

```bash
cargo run -p ennoia-cli -- dev
```

初始化运行目录：

```bash
cargo run -p ennoia-cli -- init
```

默认开发地址来自配置和 CLI 默认值：

- Web：`http://127.0.0.1:5173`
- API：`http://127.0.0.1:3710`

Docker Compose 运行时：

- `api` 容器内固定使用 `ENNOIA_HOME=/data/ennoia`
- 宿主机挂载目录优先读取宿主环境变量 `ENNOIA_HOME`
- 若宿主机未设置 `ENNOIA_HOME`，则回退到当前用户主目录下的 `~/.ennoia/`（Windows 对应 `%USERPROFILE%/.ennoia`）
- 因此 Docker 模式下不会再落到 Docker 命名卷；用户可以直接在宿主机查看和编辑运行目录

## 验证命令

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- `bun run build:workers`
- `bun run --cwd web lint`
- `bun run --cwd web typecheck`
- `bun run --cwd web build`

## 文档入口

- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
- [docs/extension-development.md](docs/extension-development.md)
