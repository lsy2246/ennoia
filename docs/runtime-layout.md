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
│  ├─ ennoia.toml              # 应用级系统配置
│  ├─ server.toml              # HTTP、中间件、系统内置组件配置
│  ├─ ui.toml                  # Web 标题、语言、主题、Dockview 偏好
│  ├─ profile.toml             # 实例资料（显示名、locale、时区、默认空间）
│  ├─ interfaces.toml          # 细粒度系统动作到扩展 Worker 方法的显式绑定
│  ├─ preferences/
│  │  ├─ instance.toml         # 实例级 UI 偏好
│  │  └─ spaces/               # 空间级 UI 偏好
│  ├─ agents/                  # Agent 结构化配置文件
│  ├─ providers/               # API 上游渠道实例配置
│  ├─ skills.toml              # 技能注册表
│  └─ extensions.toml          # 扩展注册表
├─ agents/                     # Agent 私有运行资料根目录，具体 Agent 目录懒创建
├─ extensions/                 # 扩展安装内容根目录
├─ skills/                     # 技能安装内容根目录
├─ data/
│  ├─ system/
│  │  ├─ schedules.json        # 系统 scheduler 的计划记录
│  │  └─ sqlite/
│  │     └─ system-log.db      # 系统日志 SQLite
│  └─ extensions/              # 扩展私有运行数据，例如 memory / workflow 的 sqlite
└─ logs/
   ├─ server/
   ├─ agents/
   ├─ spaces/
   └─ extensions/
```

## 配置职责

- `config/ennoia.toml`：应用级公共配置。
- `config/server.toml`：HTTP、中间件、日志级别和 bootstrap 状态等系统配置。
- `config/interfaces.toml`：接口绑定配置。
  - `bindings.<interface_key>.extension_id`
  - `bindings.<interface_key>.method`
- `config/extensions.toml`：扩展注册表，记录来源、启用状态、路径和移除意图。
- `config/skills.toml`：技能注册表，记录来源、启用状态、路径和移除意图。

## 数据职责

- `data/system/sqlite/system-log.db`：系统日志库，只记录系统组件观测事件，不记录会话 history。
- `data/system/schedules.json`：scheduler 计划列表，记录 trigger、target、params、启用状态和最近执行结果；target 可以是扩展动作或本机命令。
- `data/extensions/{extension_id}/`：扩展私有运行数据根目录。
  - 扩展私有配置、数据库、缓存和业务运行态都应保留在自己的扩展目录内，不再上浮到 `config/` 根目录。
  - `memory` 扩展在自己的目录中维护完整记忆系统数据。
  - `workflow` 扩展在自己的目录中维护 run / task / artifact / handoff 等运行数据。

## 目录职责

- `agents/`：Agent 私有工作目录、技能目录和产物目录。
- `extensions/`：扩展包真实内容目录。
- `skills/`：技能包真实内容目录。
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

系统配置始终走 TOML；接口绑定走 `config/interfaces.toml`；系统日志始终走独立 SQLite；定时计划走 `data/system/schedules.json`；会话、记忆和运行等业务数据始终由扩展实现维护。
