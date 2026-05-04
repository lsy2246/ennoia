# Ennoia 运行目录

## 路径解析

运行目录按以下顺序解析：

- 命令行参数
- 环境变量 `ENNOIA_HOME`
- 默认目录

默认目录按平台显示为：

- Windows：`C:/Users/<User>/.ennoia`
- macOS / Linux：`~/.ennoia`

## 当前落地目录

```text
<ENNOIA_HOME>/
├─ config/
│  ├─ server.toml              # HTTP、中间件、系统内置组件配置
│  ├─ ui.toml                  # Web 标题、语言、主题、默认操作者名与本地化默认值
│  ├─ profile.toml             # 实例资料（显示名、locale、时区、默认空间）
│  ├─ preferences/
│  │  ├─ instance.toml         # 实例级 UI 偏好
│  │  └─ spaces/               # 空间级 UI 偏好
│  ├─ model-endpoints/               # 模型接入实例配置
│  ├─ skills.toml              # 技能注册表
│  └─ extensions.toml          # 扩展注册表
├─ agents/
│  └─ <agent_id>/
│     ├─ agent.toml            # Agent 基础配置 + 权限配置 + 执行环境配置
│     ├─ work/                 # Agent 工作目录
│     ├─ artifacts/            # Agent 产物目录
│     └─ skills/               # Agent 私有技能目录
├─ extensions/                 # 扩展安装内容根目录
├─ skills/                     # 技能安装内容根目录
├─ data/
│  ├─ system/
│  │  ├─ schedules.json        # 系统 scheduler 的计划记录
│  │  └─ sqlite/
│  │     ├─ logs.db            # 系统日志 SQLite（logs / spans / span_links）
│  │     ├─ events.db          # 系统事件总线 SQLite
│  ├─ cache/
│  │  └─ sandboxes/            # native 执行环境可复用的沙盒缓存目录
│  └─ extensions/              # 扩展私有运行数据，例如 memory / workflow 的 sqlite
└─ logs/
   ├─ server/
   ├─ agents/
   ├─ spaces/
   └─ extensions/
```

## 配置职责

- `config/server.toml`：HTTP、中间件、内置工具超时、上游默认超时、流式轮询间隔、后台循环间隔、扩展运行时默认值、调度默认值、开发态 supervisor 参数和 bootstrap 状态等系统配置。
- `config/ui.toml`：Web 标题、语言主题、默认操作者名、默认时区、本地化默认值、可选的前端 API 默认超时和通知默认行为。
- `config/extensions.toml`：扩展注册表，记录来源、启用状态、路径和移除意图。
- `config/skills.toml`：技能注册表，记录来源、启用状态、路径和移除意图。

## 数据职责

- `data/system/sqlite/logs.db`：系统日志库，统一保存 logs、traces 和 span links；不记录会话 history。
- `data/system/sqlite/events.db`：系统事件总线，记录会话创建、消息追加等稳定系统事件，以及它们到各扩展 Hook 的投递状态。
- `data/system/schedules.json`：scheduler 计划列表，记录 trigger、executor、delivery、retry、启用状态、最近执行结果和最近运行历史；executor 可以是命令或 Agent。
- `data/extensions/{extension_id}/`：扩展私有运行数据根目录。
  - `settings.toml`：可选；由宿主为声明了 `settings[]` 的扩展保存扩展级配置。
  - 扩展私有配置、数据库、缓存和业务运行态都应保留在自己的扩展目录内，不再上浮到 `config/` 根目录。
  - `conversation` 扩展在自己的目录中维护会话、线路和消息数据。
  - `memory` 扩展在自己的目录中维护完整记忆系统数据。
  - `workflow` 扩展在自己的目录中维护 run / task / artifact / handoff 等运行数据。

## 目录职责

- `agents/`：Agent 的统一目录根；每个 Agent 的基础配置、权限配置、执行环境配置、工作目录、技能目录和产物目录都收敛在自己的子目录里。
- `extensions/`：扩展真实内容目录。
- `skills/`：技能真实内容目录。
- `logs/`：文本日志与开发日志输出目录，不等同于系统日志数据库。

## 懒创建目录

以下目录只在实际使用时创建：

- `agents/<agent_id>/`
- `spaces/`
- `policies/`
- `global/`
- `data/system/schedules.json`
- `data/extensions/<extension_id>/`

## 初始化行为

`cargo run -p ennoia-cli -- init` 会自动创建运行目录、基础配置、扩展与技能注册表、日志目录，并同步未卸载的内置扩展与技能。初始化不会预先写入会话数据、记忆数据、定时计划或运行数据。

系统配置始终走 TOML；系统日志与系统事件总线都走独立 SQLite；定时计划走 `data/system/schedules.json`；会话、记忆和运行等业务数据始终由扩展实现维护。
