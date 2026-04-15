# Ennoia 运行目录

`Ennoia` 的运行目录默认位于：

```text
~/.ennoia/
```

CLI 路径解析顺序：

1. 命令行参数传入的目录
2. 环境变量 `ENNOIA_HOME`
3. 默认目录 `~/.ennoia`

## 1. 目录树

```text
~/.ennoia/
├─ config/
│  ├─ ennoia.toml
│  ├─ server.toml
│  ├─ ui.toml
│  ├─ agents/
│  │  └─ *.toml
│  └─ extensions/
│     └─ *.toml
├─ state/
│  ├─ queue/
│  ├─ runs/
│  ├─ cache/
│  └─ sqlite/
│     └─ ennoia.db
├─ global/
│  ├─ extensions/
│  └─ skills/
├─ agents/
│  └─ <agent_id>/
│     ├─ skills/
│     ├─ workspace/
│     ├─ artifacts/
│     └─ cache/
├─ spaces/
│  └─ <space_id>/
│     ├─ workspace/
│     ├─ artifacts/
│     └─ cache/
└─ logs/
```

## 2. 配置层

### `config/`

这一层只放启动配置和目录扫描入口：

- `ennoia.toml`：全局配置
- `server.toml`：服务监听、安全和资源限制
- `ui.toml`：前端 UI 配置
- `agents/*.toml`：每个 Agent 一个配置文件
- `extensions/*.toml`：每个 system extension 一个配置文件

加载方式采用目录扫描，参考 Nginx 风格。

`extensions/*.toml` 当前包含：

- `enabled`：是否启用扩展
- `install_dir`：扩展安装目录
- `id`、`kind`：扩展标识信息

## 3. 状态层

### `state/`

这一层只放系统级状态：

- `queue/`：调度队列与轻量 IPC
- `runs/`：run 注册信息、状态索引、owner 映射
- `cache/`：全局缓存
- `sqlite/ennoia.db`：SQLite 模式数据库

业务产物始终随 owner 管理，不进入 `state/`。

## 4. 全局层

### `global/extensions/`

安装后的 system extension 目录。

每个扩展目录至少包含：

```text
global/extensions/<ext_id>/
└─ manifest.toml
```

`manifest.toml` 中的 `contributes` 当前支持：

- `pages[]`：`id`、`title`、`route`、`mount`、`icon`
- `panels[]`：`id`、`title`、`mount`、`slot`、`icon`
- `themes[]`：`id`、`label`、`entry`
- `commands[]`：`id`、`title`、`action`、`shortcut`
- `providers[]`：`id`、`kind`、`entry`
- `hooks[]`：`event`、`handler`

### `global/skills/`

所有 Agent 与 Space 都能复用的全局技能目录。

## 5. Agent 层

### `agents/<agent_id>/`

表示跟某个 Agent 私聊时，它自己的执行与产物空间：

- `skills/`：Agent 私有技能
- `workspace/`：该 Agent 的私聊工作区
- `artifacts/`：该 Agent 产生的产物
- `cache/`：该 Agent 私有缓存

Run 级产物建议归档到：

```text
agents/<agent_id>/artifacts/runs/<run_id>/
```

## 6. Space 层

### `spaces/<space_id>/`

表示群聊、项目房间和多人空间：

- `workspace/`：该 Space 的共享工作区
- `artifacts/`：该 Space 的产物
- `cache/`：该 Space 的缓存

这里不单独再建 `shared/`，因为 `workspace/` 本身就表示该空间的共享工作区。

Run 级产物建议归档到：

```text
spaces/<space_id>/artifacts/runs/<run_id>/
```

## 7. 日志层

### `logs/`

日志按职责细分：

- `logs/server/`
- `logs/scheduler/`
- `logs/agents/`
- `logs/spaces/`
- `logs/extensions/`

## 8. 归属规则

- 私聊产生的工作内容归属对应 Agent
- 群聊协作产生的工作内容归属对应 Space
- 全局共享能力进入 `global/`
- 协调和索引状态进入 `state/`
