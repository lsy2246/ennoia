# Ennoia 运行目录

默认运行目录：

```text
~/.ennoia/
```

路径解析顺序：

1. 命令行参数
2. 环境变量 `ENNOIA_HOME`
3. 默认目录 `~/.ennoia`

## 目录树

```text
~/.ennoia/
├─ config/
│  ├─ ennoia.toml
│  ├─ server.toml
│  ├─ ui.toml
│  ├─ agents/
│  └─ extensions/
├─ extensions/
│  ├─ attached/
│  │  └─ workspaces.toml
│  ├─ runtimes/
│  └─ cache/
├─ packages/
│  └─ extensions/
│     └─ <extension_id>/
│        └─ ennoia.extension.toml
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
│     └─ artifacts/
├─ spaces/
│  └─ <space_id>/
│     ├─ workspace/
│     └─ artifacts/
└─ logs/
```

## 归属规则

- direct conversation 的运行产物归属对应 agent
- group conversation 的运行产物优先归属对应 space
- 全局扩展与共享技能进入 `global/`
- 扩展源码 attach 清单进入 `extensions/attached/`
- 扩展运行时派生信息进入 `extensions/runtimes/`
- 发布态扩展描述与构建产物进入 `packages/extensions/`
- SQLite、调度索引和缓存进入 `state/`
- Server、request 与 runtime audit 相关日志进入 `logs/`

Run 级产物落盘路径：

- `agents/<agent_id>/artifacts/runs/<run_id>/`
- `spaces/<space_id>/artifacts/runs/<run_id>/`
