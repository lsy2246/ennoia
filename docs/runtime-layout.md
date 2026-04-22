# Ennoia 运行目录

## 路径解析

运行目录通过以下顺序解析：

- 命令行参数
- 环境变量 `ENNOIA_HOME`
- 默认目录

默认目录按平台显示为：

- Windows：`C:/Users/<User>/.ennoia`
- macOS / Linux：`~/.ennoia`

Web 和生成配置应展示解析后的平台路径。`~/.ennoia` 只作为模板占位和配置输入兼容格式使用，不作为 Windows UI 的默认展示值。

## 当前落地目录

```text
<ENNOIA_HOME>/
├─ config/
│  ├─ ennoia.toml              # 应用级系统配置
│  ├─ server.toml              # HTTP、中间件、日志、bootstrap 状态
│  ├─ ui.toml                  # Web 标题、语言、主题、Dockview 偏好
│  ├─ profile.toml             # 实例资料（显示名、locale、时区、默认空间）
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
│  └─ extensions/              # 扩展私有运行数据，例如 session/session.db
└─ logs/
   ├─ server/
   ├─ agents/
   ├─ spaces/
   └─ extensions/
```

## 目录职责

- `config/`：全局结构化配置入口。系统级配置和实例偏好都只走文件；Web 设置页直接写 `ennoia.toml`、`server.toml`、`profile.toml` 与 `preferences/*.toml`；`providers/` 下的默认渠道实例可由扩展包内 `provider-presets/*.toml` 初始化。
- `agents/`：Agent 私有运行资料，包含私有技能和产物，不承载全局 Agent 清单语义。
- `extensions/`：扩展包真实内容目录。内置扩展、手动安装扩展和开发同步扩展都落在这里。
- `skills/`：技能包真实内容目录。内置技能与手动创建技能都落在这里。
- `config/extensions.toml`：扩展注册表，记录来源、启用状态、路径和用户卸载意图。
- `config/skills.toml`：技能注册表，记录来源、启用状态、路径和用户卸载意图。
- `data/`：扩展私有运行数据根目录。核心不创建主业务数据库；`session`、`memory`、`workflow` 等扩展各自管理私有存储。
- `logs/`：统一日志文件落点，Web 日志页聚合前端日志和扩展运行事件。

## 懒创建目录

以下目录只在实际使用时创建，不在新用户初始化时出现：

- `agents/<agent_id>/`：只有创建或运行对应 Agent 时创建，包含 `work/`、私有技能和产物。
- `spaces/`：只有 Space 产物写入时创建。
- `policies/`：只有用户自定义策略文件时创建；默认策略来自内置值。
- `global/`：只有显式安装全局资产时创建。

## 初始化行为

`cargo run -p ennoia-cli -- init` 会自动创建运行目录、基础配置、扩展与技能注册表、日志目录，并同步未卸载的内置扩展与技能。内置扩展若提供 `provider-presets/*.toml`，CLI 会把这些默认渠道实例同步到 `config/providers/`。初始化不会创建默认 Agent、`agents/<agent_id>/`、`spaces/`、`policies/`、`global/`。

系统配置只走 TOML 文件；扩展数据结构由各扩展自己的 `data/` 与私有迁移机制管理。
