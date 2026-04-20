# Ennoia

`Ennoia` 是一个面向单操作者、多 Agent 的本地 AI 工作台。

当前源码仓库已经完成第一轮统一重构，系统主语义固定为：

- 单操作者实例，通过欢迎引导建立 `workspace_profile`
- 原生支持 `一对一会话 + 多 Agent 会话`
- `Space` 作为项目与工作容器承载长期上下文
- UI 偏好采用“浏览器本地缓存优先 + 实例级偏好同步”

## 当前能力

- Rust workspace 已拆分为 `kernel / memory / orchestrator / scheduler / extension-host / server / cli`
- 后端已经切换到 `workspace_profile / conversations / lanes / handoffs / runs / tasks / artifacts` 模型
- 扩展系统已经切到 `ExtensionRuntime`，统一暴露 workspace/package 描述、运行时快照、诊断与 reload/attach 链路
- 前端 `web/apps/shell` 现在提供首次引导、会话、空间、工作流、定时任务、记忆、扩展、Agent、产物、日志、设置等正式控制台视图
- 多语言、多主题和本地偏好缓存已接入正式运行链路，主题切换和控制台导航统一走运行时配置
- 会话已经具备创建、查看、删除的完整治理链路
- 默认运行目录与 SQLite 布局固定在 `~/.ennoia/`

## 技术栈

- 后端：Rust、Tokio、Axum、SQLx、Serde、TOML
- 数据库：SQLite
- 前端：React、Vite、TanStack Router、Zustand、Panda CSS
- 包管理：`bun`
- 发布目标：`一个 npm 包 + ~/.ennoia 配置目录`

## 核心模块

- `crates/kernel`：共享领域模型与系统配置协议
- `crates/memory`：记忆、上下文组装与回顾
- `crates/orchestrator`：会话消息到 run/task 的编排
- `crates/scheduler`：定时作业与后台推进
- `crates/extension-host`：扩展、页面、面板、主题、语言包注册
- `crates/server`：HTTP API 与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web/apps/shell`：单一前端主壳与正式控制台
- `web/packages/api-client`：前端统一 API 访问层

## 启动方式

### Docker

```bash
docker compose up -d
```

默认访问地址：

- Shell：`http://127.0.0.1:5173`
- API：`http://127.0.0.1:3710`

首次打开 Shell 时，如果实例尚未初始化，会自动进入欢迎引导页。

### 本地开发

```bash
cargo run -p ennoia-cli -- dev
```

`dev` 是一键开发编排入口，会自动：

- 初始化运行目录
- 扫描并 attach `./extensions/*/ennoia.extension.toml`
- 启动 Shell Vite dev server
- 启动 API Server
- 启动扩展前端 `frontend.dev_command`
- 由 Extension Runtime 托管扩展后端 `backend.dev_command`
- 汇总日志到 `~/.ennoia/logs/`
- `Ctrl+C` 时统一停止子进程

扩展开发常用命令：

```bash
cargo run -p ennoia-cli -- ext attach <path>
cargo run -p ennoia-cli -- ext inspect <id>
cargo run -p ennoia-cli -- ext reload <id>
cargo run -p ennoia-cli -- ext logs
cargo run -p ennoia-cli -- ext graph
```

当你从仓库根目录执行 `ennoia dev` 时，CLI 会自动扫描 `./extensions/*/ennoia.extension.toml` 并写入实例级 attach 清单。

环境示例文件：

- 仓库根目录：`.env.example`

## 验证命令

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- `bun run --cwd web/apps/shell typecheck`
- `bun run --cwd web/apps/shell build`

## 文档入口

- [AGENTS.md](AGENTS.md)
- [docs/development/conventions.md](docs/development/conventions.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/extension-development.md](docs/extension-development.md)
- [docs/extension-runtime-rfc.md](docs/extension-runtime-rfc.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
- [docs/i18n-and-theming.md](docs/i18n-and-theming.md)
