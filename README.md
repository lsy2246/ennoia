# Ennoia

`Ennoia` 是面向单操作者、多 Agent 的本地 AI Web 工作台。当前仓库处于开发基线阶段，不维护旧数据库或旧目录兼容。

## 产品结构

- 工作台：核心只提供宿主、配置、路径、日志、权限、事件总线、动作管道与 Worker RPC；业务能力由扩展提供。
- Agents：维护协作者档案、模型接入、模型、技能和启用状态。
- Agent 权限：系统级权限策略、审批和事件记录统一由宿主裁决；扩展 manifest 不声明底层权限边界，宿主按 operation 与调用上下文裁决。
- 技能：Agent 可挂载的标准能力包；skill 默认对外暴露单一能力入口，内部脚本不直接等同于 action，具体调用说明写在各自的 `README.md` 中。
- API 模型接入：Agent 绑定的具体模型访问实例。
- 扩展：扩展包，manifest 只声明系统可见契约：`id`、`version`、`name`、`description`、`docs`、`compat`、`views`、`operations`、`events`、`settings`、`conversation`。扩展自己的 UI / service 入口、数据库、缓存和内部实现由目录约定与扩展代码自行负责，不进入系统级设计界面。宿主把扩展/技能目录整理成结构化 `context` 交给 model provider 渲染，不再把它们直接硬拼进自然语言 prompt，也不自动注入文档正文。
- 会话：前端通过通用 `/api/actions/{action}` 分发 `conversation.*`、`message.*`、`lane.*` 等动作，底层由内置 `conversation` 扩展实现。
- 记忆：以内置 `memory` 扩展形式提供记忆、上下文、审查和图谱能力；不再镜像保存原始会话消息。
- 编排：以内置 `workflow` 扩展承载 run、task、artifact，以及会话触发、审批恢复、结果回写等产品编排；核心只保留动作、事件、provider 和 runtime operation 这些中立桥接能力。
- 日志：聚合前端日志和扩展运行事件。
- 设置：通过表单直接编辑 `config/server.toml`、`config/ui.toml`、`config/profile.toml` 和 `config/preferences/*.toml`，其中 `profile.toml` 保存操作者显示名、语言、时区与自动识别的操作者系统；`server.toml` 统一承载上游超时、流式轮询节奏和后台循环节奏等运行时默认值；模型接入默认写入 `0`，表示不限制超时；`ui.toml` 只在显式配置时才对前端请求施加默认超时。

## 技术栈

- 后端：Rust、Tokio、Axum、Serde、TOML
- 存储：系统配置走 TOML 文件；定时计划走 `data/system/schedules.json`；扩展按需使用自己的私有存储。
- 前端：React、Vite、TanStack Router、Zustand
- 包管理：`bun`
- 发布目标：一个 npm 包 + `~/.ennoia` 运行目录

## 运行态与开发态

- `ennoia start` / `ennoia serve`：运行态，继续使用 `ENNOIA_HOME` 或默认 `~/.ennoia`
- `ennoia start` / `ennoia serve` 不接收路径参数；运行目录只能通过 `ENNOIA_HOME` 或默认 `~/.ennoia` 决定
- `ennoia dev` / `npm run dev`：开发态，固定使用仓库根目录下的 `.dev/`
- `ennoia stop`：停止当前开发态或运行态进程；`ennoia stop dev` 显式停止仓库 `.dev/`，`ennoia stop [home]` 停止指定运行目录
- 开发态不会默认读写 `~/.ennoia`，开发日志、扩展设置、热加载状态和扩展私有数据都留在仓库内的 `.dev/` 下
- `start/serve` 不读取 `dev_sources`，只消费运行目录中的已安装扩展和技能；`dev_sources` 仅在 `ennoia dev` 下生效

## 核心模块

- `crates/kernel`：共享协议、配置和扩展 manifest 模型
- `crates/extension-host`：扩展运行时、热刷新、诊断和 Worker RPC 分发
- `crates/server`：HTTP API、系统配置文件、日志、能力路由与运行时装配
- `crates/cli`：初始化、开发与启动入口
- `web`：Ennoia Web 工作台
- `web/packages/api-client`：前端统一 API 访问层
- `assets/extensions/conversation`：内置会话扩展，声明原生会话事实接口
- `assets/extensions/memory`：内置记忆扩展，声明记忆、上下文、审查与图谱接口
- `assets/extensions/workflow`：内置编排扩展，声明 run/task/artifact 接口

## 内置能力源码

- `assets/extensions/*`：官方内置扩展源码
- `assets/skills/*`：官方内置技能源码
- 运行态初始化会把未被 `blocked_builtin_sync` 屏蔽的内置包同步到 `~/.ennoia/extensions/*` 与 `~/.ennoia/skills/*`
- 开发态会把内置扩展和技能源码分别挂到仓库 `.dev/config/extensions.toml` 与 `.dev/config/skills.toml` 的 `dev_sources`
- 开发态只生成配置、日志、pid 和状态数据，不把内置扩展/技能包复制到 `.dev/extensions/*` 与 `.dev/skills/*`，并会清理旧版本留下的内置包副本目录
- `config/extensions.toml` 与 `config/skills.toml` 保存运行时覆盖状态；其中 `dev_sources` 仅用于开发态

## 存储边界

- 运行目录统一分成两类：`config/` 保存声明性配置，`data/` 保存全部运行数据。
- 运行态核心系统配置走 `~/.ennoia/config/*.toml`；开发态对应配置走仓库 `.dev/config/*.toml`。
- 系统动作规则来自扩展 manifest 的 `operations[]`；`operation.name` 是唯一调用名，同时作为 action key、Worker method 和事件投递目标。
- 系统定时计划写入 `~/.ennoia/data/system/schedules.json`，到期后由宿主运行命令或触发 Agent，并可把完整结果、摘要或最终结论投递到会话 / lane。
- 系统事件总线写入 `~/.ennoia/data/system/sqlite/events.db`，用于持久化会话等系统事件及其 Hook 投递状态。
- 系统日志数据写入 `~/.ennoia/data/system/sqlite/logs.db`，统一承载 logs、traces 和 span links。
- Agent 基础配置与权限策略统一写入 `~/.ennoia/agents/{agent_id}/agent.toml`，权限事件与审批写入 `~/.ennoia/data/system/sqlite/permissions.db`。
- 核心文本日志与前端日志写入对应 home 的 `data/system/logs/`；开发态默认是仓库 `.dev/data/system/logs/`。
- 运行态与开发态的 pid 文件统一写入对应 home 的 `data/system/pids/`。
- 扩展私有数据写入对应 home 的 `data/extensions/{extension_id}/`，扩展级宿主配置写入 `config/extensions/{extension_id}.toml`。
- 技能私有数据写入对应 home 的 `data/skills/{skill_id}/`，其中最近一次检测结果缓存为 `status.json`；技能级宿主配置写入 `config/skills/{skill_id}.toml`。
- 核心不提供主业务 SQLite，不内建语义记忆、编排、任务或产物索引表。

## 启动方式

安装依赖：

```bash
bun install
```

安装阶段会自动执行 `web` typecheck，并在本机具备 Rust toolchain 时执行 `cargo check --workspace`。

启动开发环境：

```bash
npm run dev
```

开发态固定使用当前仓库根目录下的 `.dev/` 作为 home。

停止开发环境：

```bash
npm run stop -- dev
```

`npm run stop` 会直接读取 pid 文件停进程，不触发 Rust CLI 编译。

初始化运行目录：

```bash
cargo run -p ennoia-cli -- init
```

启动默认运行目录：

```bash
npm run start
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
