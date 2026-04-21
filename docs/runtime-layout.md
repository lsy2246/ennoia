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
│  ├─ ennoia.toml              # 应用、工作区、扫描目录、调度节拍
│  ├─ server.toml              # HTTP、日志等级、CORS、WebSocket
│  ├─ ui.toml                  # Web 标题、语言、主题、Dockview 偏好
│  ├─ agents/                  # Agent 结构化配置文件
│  ├─ providers/               # API 上游渠道实例配置
│  ├─ skills/                  # 可发现技能索引
│  └─ extensions/              # 扩展启用、来源与扫描入口
├─ agents/                     # Agent 私有运行资料根目录，具体 Agent 目录懒创建
├─ extensions/
│  ├─ attached/workspaces.toml # 开发目录扫描来源
│  ├─ runtimes/                # 扩展运行时状态
│  └─ cache/                   # 扩展运行时内部缓存
├─ packages/
│  └─ extensions/              # 以扩展包为分区的安装内容
├─ data/
│  ├─ sqlite/ennoia.db         # SQLite 数据库
│  ├─ queue/                   # 调度队列
│  ├─ runs/                    # 运行状态
│  └─ cache/                   # 系统缓存
└─ logs/
   ├─ server/
   ├─ scheduler/
   ├─ agents/
   ├─ spaces/
   └─ extensions/
```

## 目录职责

- `config/`：全局结构化配置入口。Web 设置页只编辑表单，不暴露原始 JSON。
- `agents/`：Agent 私有运行资料，包含私有技能和产物，不承载全局 Agent 清单语义。
- `extensions/`：扩展运行态、开发目录扫描入口和运行缓存。
- `packages/extensions/`：以扩展包为分区的安装内容；扩展包贡献页面、面板、主题、语言、命令或 API 上游接口实现。
- `data/`：系统内部数据目录，替代旧的 `state/` 命名。
- `logs/`：统一日志文件落点，Web 日志页聚合前端日志、后端日志、扩展事件和运行摘要。

## 懒创建目录

以下目录只在实际使用时创建，不在新用户初始化时出现：

- `workspace/`：用户在设置中选择的工作区根路径，只有发生工作区写入时创建。
- `agents/<agent_id>/`：只有创建或运行对应 Agent 时创建，包含私有技能和产物。
- `spaces/`：只有 Space 产物写入时创建。
- `policies/`：只有用户自定义策略文件时创建；默认策略来自内置值。
- `global/`：只有显式安装全局资产时创建。

## 初始化行为

`cargo run -p ennoia-cli -- init` 会自动创建运行目录、基础配置、技能索引、默认 API 上游渠道、数据库目录和日志目录。初始化不会创建默认 Agent、默认扩展包、`workspace/`、`agents/<agent_id>/`、`spaces/`、`policies/`、`global/`。
