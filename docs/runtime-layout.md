# Ennoia 运行目录

## 路径解析

运行态目录按以下顺序解析：

- 命令行参数
- 环境变量 `ENNOIA_HOME`
- 默认目录

默认目录按平台显示为：

- Windows：`C:/Users/<User>/.ennoia`
- macOS / Linux：`~/.ennoia`

开发态目录单独处理：

- `ennoia dev` 固定使用当前仓库根目录下的 `.dev/`
- 开发态不读取命令行 `home` 参数
- 开发态不默认读取 `ENNOIA_HOME`
- 开发态才会读取 `config/extensions.toml` 与 `config/skills.toml` 中的 `dev_sources`
- `ennoia start` / `ennoia serve` 会忽略 `dev_sources`，只使用运行目录中的安装扩展和技能

## 开发态目录

```text
<repo>/.dev/
├─ config/
├─ extensions/           # 开发态可存在；内置扩展旧副本会被清理，源码真源是 assets/extensions/*
├─ skills/               # 开发态可存在；内置技能旧副本会被清理，源码真源是 assets/skills/*
└─ data/
   ├─ system/
   │  ├─ logs/
   │  └─ pids/
   ├─ extensions/
   └─ skills/
```

- 开发日志、pid、扩展设置、扩展私有数据、技能私有数据、热加载过程中的运行状态都写入 `.dev/`
- `.dev/` 属于仓库内开发目录，应由 `.gitignore` 屏蔽
- `config/extensions.toml` 与 `config/skills.toml` 中的 `dev_sources` 只用于开发态

## 运行态落地目录

```text
<ENNOIA_HOME>/
├─ config/
│  ├─ server.toml              # HTTP、中间件、系统内置组件配置
│  ├─ ui.toml                  # Web 标题、语言、主题、默认操作者名与本地化默认值
│  ├─ profile.toml             # 实例资料（显示名、locale、时区、操作者系统、默认空间）
│  ├─ preferences/
│  │  ├─ instance.toml         # 实例级 UI 偏好
│  │  └─ spaces/               # 空间级 UI 偏好
│  ├─ model-endpoints/               # 模型接入实例配置
│  ├─ skills.toml              # 技能运行时覆盖状态
│  ├─ extensions.toml          # 扩展运行时覆盖状态
│  ├─ skills/                  # 技能级宿主配置
│  └─ extensions/              # 扩展级宿主配置
├─ agents/
│  └─ <agent_id>/
│     ├─ agent.toml            # Agent 基础配置 + 权限配置 + 文件访问配置
│     ├─ work/                 # Agent 工作目录
│     ├─ artifacts/            # Agent 产物目录
│     └─ skills/               # Agent 私有技能目录
├─ extensions/                 # 扩展安装内容根目录
├─ skills/                     # 技能安装内容根目录
├─ data/
│  ├─ system/
│  │  ├─ schedules.json        # 系统 scheduler 的计划记录
│  │  ├─ logs/
│  │  │  ├─ frontend.jsonl     # 前端日志落盘
│  │  │  ├─ server/            # API / Web / 构建等文本日志
│  │  │  ├─ agents/
│  │  │  ├─ spaces/
│  │  │  └─ extensions/
│  │  ├─ pids/
│  │  │  ├─ server.pid         # 运行态 server 进程 pid
│  │  │  └─ dev.pid            # 开发态 supervisor 进程 pid
│  │  └─ sqlite/
│  │     ├─ logs.db            # 系统日志 SQLite（logs / spans / span_links）
│  │     ├─ events.db          # 系统事件总线 SQLite
│  ├─ cache/
│  │  └─ file-access/          # Agent 文件访问临时目录
│  ├─ extensions/              # 扩展私有运行数据，例如 memory / workflow 的 sqlite
│  └─ skills/                  # 技能私有运行数据
```

## 配置职责

- `config/server.toml`：HTTP、中间件、内置工具超时、上游默认超时、流式轮询间隔、后台循环间隔、扩展运行时默认值、调度默认值、开发态 supervisor 参数和 bootstrap 状态等系统配置。
- `config/ui.toml`：Web 标题、语言主题、默认操作者名、默认时区、本地化默认值、可选的前端 API 默认超时和通知默认行为。
- `config/extensions.toml`：扩展运行时覆盖状态，记录 `enabled` 与 `blocked_builtin_sync`；开发态目录中额外保存 `dev_sources`，指向 `assets/extensions/*` 等源码目录。
- `config/skills.toml`：技能运行时覆盖状态，记录 `enabled` 与 `blocked_builtin_sync`；开发态目录中额外保存 `dev_sources`，指向 `assets/skills/*` 等源码目录。
- `config/extensions/{extension_id}.toml`：扩展级宿主配置，例如声明过的 `settings[]`。
- `config/skills/{skill_id}.toml`：技能级宿主配置，保存技能声明过的设置字段值。

## 数据职责

- `config/` 只保存声明性配置；运行过程中的输出、状态与缓存都进入 `data/`。
- `data/system/sqlite/logs.db`：系统日志库，统一保存 logs、traces 和 span links；不记录会话 history。
- `data/system/sqlite/events.db`：系统事件总线，记录会话创建、消息追加等稳定系统事件，以及它们到各扩展 Hook 的投递状态。
- `data/system/schedules.json`：scheduler 计划列表，记录 trigger、executor、delivery、retry、启用状态、最近执行结果和最近运行历史；executor 可以是命令或 Agent。
- `data/system/logs/`：文本日志与前端日志输出目录，不等同于系统日志数据库。
- `data/system/pids/server.pid`：当前运行态 server 的 pid 文件，供 `ennoia stop [home]` 使用。
- `data/system/pids/dev.pid`：当前开发态 supervisor 的 pid 文件，供 `ennoia stop dev` 使用。
- `data/extensions/{extension_id}/`：扩展私有运行数据根目录。
  - `conversation` 扩展在自己的目录中维护会话、线路和消息数据。
  - `memory` 扩展在自己的目录中维护完整记忆系统数据。
  - `workflow` 扩展在自己的目录中维护 run / task / artifact / handoff 等运行数据。
- `data/skills/{skill_id}/`：技能私有运行数据根目录。
  - `status.json`：最近一次技能检测结果缓存，供技能页展示“是否就绪 / 缺什么 / 为什么不可用”。

## 目录职责

- `agents/`：Agent 的统一目录根；每个 Agent 的基础配置、权限配置、文件访问配置、工作目录、技能目录和产物目录都收敛在自己的子目录里。
- `extensions/`：运行态扩展真实内容目录；开发态内置扩展直接从 `dev_sources` 指向的源码目录加载。
- `skills/`：运行态技能真实内容目录；开发态内置技能直接从 `dev_sources` 指向的源码目录加载。
- `data/system/logs/`：文本日志与开发日志输出目录，不等同于系统日志数据库。

## 懒创建目录

以下目录只在实际使用时创建：

- `agents/<agent_id>/`
- `spaces/`
- `policies/`
- `global/`
- `data/system/schedules.json`
- `data/extensions/<extension_id>/`

## 初始化行为

`cargo run -p ennoia-cli -- init` 会自动创建运行目录、基础配置、扩展与技能运行时覆盖文件、`data/system/logs/`、`data/system/pids/` 等基础目录，并同步未被 `blocked_builtin_sync` 屏蔽的内置扩展与技能。初始化不会预先写入会话数据、记忆数据、定时计划或运行数据。

`cargo run -p ennoia-cli -- dev` 会在当前仓库根目录自动创建 `.dev/`，并在该目录下初始化开发态所需的配置、日志、pid 和数据目录。开发态会把 `assets/extensions/*` 与 `assets/skills/*` 注册到对应的 `dev_sources`，不物化内置扩展/技能包，并会清理旧版本留下的内置包副本目录；热加载状态、扩展设置和技能设置都写入 `.dev/`。

系统配置始终走 TOML；系统日志与系统事件总线都走独立 SQLite；定时计划走 `data/system/schedules.json`；会话、记忆和运行等业务数据始终由扩展实现维护。
