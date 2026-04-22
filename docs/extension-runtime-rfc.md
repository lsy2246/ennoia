# Extension Runtime RFC

本文档记录当前 Extension Runtime 的已落地约定。历史设计中的 `global/extensions`、`packages/extensions`、旧附加来源注册表和 Skill/Extension 混合模型已经废弃。

## 当前目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 安装扩展包：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 扩展日志：`<ENNOIA_HOME>/logs/extensions/`

## 当前协议

Extension 使用 `extension.toml` 描述系统插件能力。Skill 使用 `skill.toml` 描述 Agent 能力包，两者互不兼容、互不混用。

Extension descriptor 包含：

- `source`
- `frontend`
- `backend`
- `build`
- `assets`
- `watch`
- `capabilities`
- `contributes`

贡献类型包含：页面、面板、主题、语言包、命令、Provider 实现和 Hook。

## 运行流程

1. CLI 初始化运行目录和默认配置。
2. CLI 同步内置扩展到 `<ENNOIA_HOME>/extensions/*`，并写入 `config/extensions.toml`。
3. 开发模式下 CLI 把仓库内 `builtins/extensions/*` 追加为开发来源。
4. Extension Host 扫描 `config/extensions.toml` 中启用且未移除的扩展来源。
5. Extension Host 启动后端 dev command 并采集事件。
6. Server 暴露 runtime snapshot、事件、诊断、日志和资源贡献接口。
7. Web 工作台通过 runtime snapshot 动态挂载扩展贡献。

扩展源码推荐目录为 `plugins/`、`hooks/`、`timers/`、`ui/`、`data/`。这些目录不是必备项，但表达 Hook 决定时机、Timer 决定时间触发、Plugin 决定执行动作、Extension 负责组织接入系统。

## API

- `GET /api/v1/extensions`
- `GET /api/v1/extensions/runtime`
- `GET /api/v1/extensions/events`
- `GET /api/v1/extensions/events/stream`
- `GET /api/v1/extensions/{extension_id}`
- `GET /api/v1/extensions/{extension_id}/diagnostics`
- `PUT /api/v1/extensions/{extension_id}/enabled`
- `POST /api/v1/extensions/{extension_id}/reload`
- `POST /api/v1/extensions/{extension_id}/restart`
- `POST /api/v1/extensions/attach`
- `DELETE /api/v1/extensions/attach/{extension_id}`
