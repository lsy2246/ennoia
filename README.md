# Ennoia

`Ennoia` 是面向单操作者、多 Agent 的本地 AI Web 工作台。当前仓库处于开发基线阶段，不维护旧数据库或旧目录兼容。

## 产品结构

- 工作台：核心只提供宿主、配置、路径、日志和扩展代理；业务能力由扩展提供。
- Agents：维护协作者档案、上游渠道、模型、技能和启用状态。
- 技能：Agent 可引用的能力包，和扩展严格分离。
- API 上游渠道：Agent 绑定的具体模型访问实例。
- 扩展：系统插件包，可贡献页面、面板、主题、语言、命令、Hook 和上游实现。
- 会话：以内置 `session` 扩展形式提供 Conversation、Lane、Message 与 Handoff。
- 记忆：以内置 `memory` 扩展形式提供独立前后端、私有数据库与 Web 页面。
- 编排：以内置 `workflow` 扩展承载运行编排、stage、decision、gate 与 artifact 产出。
- 日志：聚合前端日志和扩展运行事件。
- 设置：通过表单直接编辑 `app/server` 文件配置、`config/profile.toml` 和 `config/preferences/*.toml`。

## 技术栈

- 后端：Rust、Tokio、Axum、Serde、TOML
- 存储：系统配置走 TOML 文件；扩展按需使用自己的私有存储。
- 前端：React、Vite、TanStack Router、Zustand
- 包管理：`bun`
- 发布目标：一个 npm 包 + `~/.ennoia` 运行目录

## 核心模块

- `crates/kernel`：共享协议、配置和扩展 manifest 模型
- `crates/extension-host`：扩展运行时、热刷新、诊断和后端进程托管
- `crates/server`：HTTP API、系统配置文件、日志、扩展代理与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web`：Ennoia Web 工作台
- `web/packages/api-client`：前端统一 API 访问层
- `builtins/extensions/session`：内置会话扩展，包含 `plugins/`、`ui/`、`data/`
- `builtins/extensions/memory`：内置记忆扩展，包含 `plugins/`、`ui/`、`data/`
- `builtins/extensions/workflow`：内置编排扩展，包含 `plugins/`、`hooks/`、`timers/`、`data/`

## 内置能力源码

- `builtins/extensions/*`：官方内置扩展源码
- `builtins/skills/*`：官方内置技能源码
- 初始化会把未卸载的内置包同步到 `~/.ennoia/extensions/*` 与 `~/.ennoia/skills/*`
- 用户安装/启用/卸载登记统一写入 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## 存储边界

- 核心系统配置只走 `~/.ennoia/config/*.toml`。
- 核心日志写入 `~/.ennoia/logs/`。
- 扩展私有数据写入 `~/.ennoia/data/extensions/{extension_id}/`。
- 核心不提供主业务 SQLite，不内建会话、记忆、编排、任务或产物索引表。

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
