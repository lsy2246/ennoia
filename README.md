# Ennoia

`Ennoia` 是一个面向单操作者、多 Agent 的本地 AI 工作台。

当前仓库已经切换到新的正式产品结构，一级导航固定为：

- `聊天`
- `计划任务`
- `Agent`
- `扩展`
- `日志`
- `设置`

## 当前产品结构

- `聊天`
  - 私聊与群聊统一为 `ChatThread`
  - 主视图是聊天盒子，不再拆成“会话 / 空间”两套入口
  - 子 Agent 调用是消息流中的一环，可进入非正式子聊天窗口查看
  - 工具调用、思考过程、执行过程、输出都围绕消息流展示
- `计划任务`
  - 基于统一调度记录承载
  - 支持创建、编辑、删除、启停、立即执行
- `Agent`
  - 作为长期可配置的协作者档案
- `扩展`
  - 支持挂载开发目录、启用、停用、重载、重启、查看诊断与日志
- `日志`
  - 只保留系统级、扩展级、任务级总览日志
  - 业务过程日志贴近聊天和任务详情页
- `设置`
  - 管理语言、主题、时区、运行时配置
  - 主题与语言支持即时预览，保存后持久化

## 技术栈

- 后端：Rust、Tokio、Axum、SQLx、Serde、TOML
- 数据库：SQLite
- 前端：React、Vite、TanStack Router、Zustand、Panda CSS
- 包管理：`bun`
- 发布目标：`一个 npm 包 + ~/.ennoia 配置目录`

## 核心模块

- `crates/kernel`：共享领域模型与系统配置协议
- `crates/memory`：上下文、记忆、回顾
- `crates/orchestrator`：聊天消息到执行计划的编排
- `crates/scheduler`：计划任务与后台推进
- `crates/extension-host`：扩展运行时、挂载、热刷新、诊断
- `crates/server`：HTTP API 与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web/apps/shell`：前端主壳与工作台
- `web/packages/api-client`：前端统一 API 访问层

## 启动方式

### 本地开发

```bash
cargo run -p ennoia-cli -- dev
```

`dev` 是一键开发入口，会自动：

- 初始化运行目录
- 启动 Shell Vite dev server
- 启动 API Server
- 监听 Rust 代码变化并自动重编译/重启 API
- 启动扩展前端 `frontend.dev_command`
- 由 Extension Runtime 托管扩展后端 `backend.dev_command`
- 汇总日志到 `~/.ennoia/logs/`

默认地址：

- Shell：`http://127.0.0.1:5173`
- API：`http://127.0.0.1:3710`

如果开发入口提示 `API port 3710 is already in use`，先关闭已有 `ennoia-api-*.exe` 进程后再重试。

### Docker

```bash
docker compose up -d
```

## 验证命令

- `cargo fmt --all`
- `cargo check --workspace`
- `cargo test --workspace`
- `bun run --cwd web/apps/shell typecheck`
- `bun run --cwd web/apps/shell build`

当前 `web/apps/shell` 还没有单独的 `lint` 脚本。

## 文档入口

- [AGENTS.md](AGENTS.md)
- [docs/development/conventions.md](docs/development/conventions.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/extension-development.md](docs/extension-development.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
- [docs/i18n-and-theming.md](docs/i18n-and-theming.md)
