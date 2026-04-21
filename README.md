# Ennoia

`Ennoia` 是面向单操作者、多 Agent 的本地 AI Web 工作台。当前仓库处于开发基线阶段，不维护旧数据库或旧目录兼容。

## 产品结构

- 工作台：统一创建 `direct` / `group` Conversation。
- Agents：维护协作者档案、上游渠道、模型、技能和启用状态。
- 技能：Agent 可引用的能力包，和扩展严格分离。
- API 上游渠道：Agent 绑定的具体模型访问实例。
- 扩展：系统插件包，可贡献页面、面板、主题、语言、命令和上游实现。
- 记忆：通过 SQLite 与 Web 页面管理 recall / review。
- 任务：统一承载 AI 任务与命令任务。
- 日志：聚合前端、后端、扩展事件和运行摘要。
- 设置：通过表单管理运行时配置和实例偏好。

## 技术栈

- 后端：Rust、Tokio、Axum、SQLx、SeaQuery、Serde、TOML
- 数据库：SQLite
- 前端：React、Vite、TanStack Router、Zustand
- 包管理：`bun`
- 发布目标：一个 npm 包 + `~/.ennoia` 运行目录

## 核心模块

- `crates/kernel`：共享领域模型与配置协议
- `crates/memory`：记忆、上下文与回顾
- `crates/orchestrator`：消息到 run/task 的编排
- `crates/scheduler`：计划任务与后台推进
- `crates/extension-host`：扩展运行时、热刷新、诊断
- `crates/server`：HTTP API、SQLite 仓储与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web`：Ennoia Web 工作台
- `web/packages/api-client`：前端统一 API 访问层

## 内置能力源码

- `builtins/extensions/*`：官方内置扩展源码
- `builtins/skills/*`：官方内置技能源码
- 初始化会把未卸载的内置包同步到 `~/.ennoia/extensions/*` 与 `~/.ennoia/skills/*`
- 用户安装/启用/卸载登记统一写入 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## 数据库基线

- `assets/db.sql` 是新库初始化入口，完整、可执行、自包含。
- `assets/migrations/` 当前为空；后续数据库结构变更时再新增 migration。
- 新库启动执行 `db.sql`，已有库在后续结构变更时执行新增 migration。

## 启动方式

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

## 验证命令

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- `bun run --cwd web typecheck`
- `bun run --cwd web build`

当前 `web` 还没有单独的 `lint` 脚本。

## 文档入口

- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
- [docs/extension-development.md](docs/extension-development.md)
