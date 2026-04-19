# Ennoia

`Ennoia` 是一个面向长期演进的 AI Agent 工作台平台。

它的产品形态同时具备三种特征：

- 对协作方式像微信：支持私聊、群聊、线程和多参与者协作
- 对界面像 VSCode：支持主壳、子页面、可拖拽面板、命令入口和布局持久化
- 对系统像 AI OS：具备任务编排、记忆控制平面、调度器、扩展宿主和可观测能力

## 当前状态

当前仓库已经进入第一版完整骨架阶段：

- 核心命名和目录已经确定
- Rust workspace 已经拆分出核心模块
- 文档体系覆盖架构、路线图、运行目录、数据模型、配置模型和扩展模型
- 后端已经收敛出 `assets / paths / observability / contract` 四个基础层，资源、路径、日志和错误不再散落在业务 crate 中
- 前端主壳已经形成正式 workspace，可统一查看私聊、群聊、run、task、memory 与 extension surface
- 前端改为 `web/apps + web/packages` 结构，`shell` 通过 `@ennoia/*` package 消费共享能力
- 打包目录和配置模板已经与 SQLite 默认运行时布局保持一致

## 技术栈

- 后端：Rust、Tokio、Axum、SQLx、Serde、TOML
- 数据库：SQLite 优先，围绕本地运行目录落地首版数据存储
- 前端：React、Vite、TanStack Router、TanStack Query、Zustand、Dockview、Monaco、Panda CSS
- 前端包管理：`bun`
- 发布形态：单个 `npm` 包 `ennoia`

## 核心模块

- `kernel`：领域模型、配置协议、扩展清单、共享契约
- `memory`：记忆实体、上下文视图、连续性、复盘与索引
- `orchestrator`：线程到 run、run 到 task、任务门禁和执行链路
- `scheduler`：定时器、延迟任务、周期作业、重试和维护作业
- `assets`：内建模板、默认策略与 SQLite migration 的唯一编译期资源入口
- `paths`：运行目录、SQLite、日志、owner 产物目录的唯一解析入口
- `observability`：日志初始化、request id 与观测上下文字段的统一入口
- `contract`：后端 API 错误响应与稳定错误码契约
- `extension-host`：system extension、skill、Hook、页面与面板贡献注册
- `server`：HTTP API、WebSocket、扩展注册表、前端主壳注入
- `cli`：初始化、开发、启动、本地输出和运行目录生成
- `assets/`：仓库级内建资源目录，包含模板与 migration
- `web/apps/shell`：主壳、子页面、面板和扩展 UI 容器
- `web/packages/ui-sdk`：前端 extension page / panel 的共享类型与挂载助手
- `web/packages/builtins`：主壳内建 extension 描述与默认 surface 元数据
- `web/packages/api-client`：前端统一 API 访问层
- `web/packages/observability`：前端统一 logger
- `web/packages/contract`：前端错误契约与共享类型

## 目录概览

```text
ennoia/
├─ assets/
├─ crates/
├─ web/
├─ sdk/
├─ packaging/
├─ tests/
├─ docs/
└─ AGENTS.md
```

运行目录位于：

```text
~/.ennoia/
```

运行时路径解析顺序：

- 命令行参数显式传入的目录
- 环境变量 `ENNOIA_HOME`
- 默认目录 `~/.ennoia`

详细说明见 [docs/runtime-layout.md](docs/runtime-layout.md)。

## 设计决议

- 系统核心命名统一使用 `kernel / memory / orchestrator / scheduler / extension-host / server / shell`
- 对外统一总称使用 `Extension`
- 内部区分 `system extension` 与 `skill`
- 前端扩展以主壳下的子页面与面板挂载
- 扩展采用编译安装、重启生效
- 私聊产物归属 Agent，群聊产物归属 Space

## 测试与验证

当前仓库的基础验证链为：

- 环境初始化：`bun run setup` 或 `bun run bootstrap`
- Rust：`cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`
- 前端：`bun run typecheck`、`bun run build`
- 集成验证：`bun run test:integration`
- 主链路验证：`bun run test:e2e`

测试目录说明：

- `tests/integration/`：后端集成测试
- `tests/e2e/`：前端与主链路端到端测试入口
- `tests/fixtures/`：测试夹具
- `.tmp-tests/`：测试运行产物目录

## 安装与体验

当前开发阶段已经补齐两条“像用户一样体验”的入口：

### 1. 当前平台 npm 安装

适合验证本机的一键打包和安装体验。

```bash
bun run package:npm
npm install -g ./dist/npm/ennoia-0.1.0.tgz
ennoia init
ennoia start
```

说明：

- `bun run package:npm` 会构建当前平台的 `ennoia` CLI 并输出到 `dist/npm/`
- 生成的 tarball 只适用于打包时所在的平台
- 默认运行目录为 `~/.ennoia/`
- 服务默认监听 `127.0.0.1:3710`

### 2. Docker 一键启动

适合快速拉起后端 API 与前端主壳。

```bash
bun run docker:up
```

启动后可通过以下地址访问：

- 前端主壳：`http://127.0.0.1:5173`
- 后端 API：`http://127.0.0.1:3710`

首次访问前端主壳时，如果当前运行目录还未完成初始化，页面会自动进入首启引导，要求创建第一个管理员账号并选择认证方式。

停止命令：

```bash
bun run docker:down
```

## 文档入口

- [docs/roadmap.md](docs/roadmap.md)
- [docs/architecture.md](docs/architecture.md)
- [docs/runtime-layout.md](docs/runtime-layout.md)
- [docs/data-model.md](docs/data-model.md)
- [docs/config-model.md](docs/config-model.md)
- [docs/hooks-and-events.md](docs/hooks-and-events.md)
- [docs/api-surface.md](docs/api-surface.md)
- [docs/extension-development.md](docs/extension-development.md)
