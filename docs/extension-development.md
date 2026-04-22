# Ennoia 扩展开发指南

## 定位

Extension 是系统插件包，Skill 是 Agent 可引用的能力包。两者目录、生命周期和 manifest 语义分离。

## 源码放置

- 官方内置扩展源码放在 `builtins/extensions/<extension_id>/`
- 官方内置技能源码放在 `builtins/skills/<skill_id>/`
- 运行目录里的真实包内容分别落在 `~/.ennoia/extensions/<id>/` 与 `~/.ennoia/skills/<id>/`
- 是否启用、是否卸载、来源路径统一登记在 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## Manifest

系统扩展只使用 `extension.toml`。推荐字段：

- `source`
- `frontend`
- `backend`
- `build`
- `assets`
- `watch`
- `capabilities`
- `contributes`

`contributes` 可包含：

- `pages[]`
- `panels[]`
- `themes[]`
- `locales[]`
- `commands[]`
- `providers[]`
- `hooks[]`

Hook 贡献声明扩展要接收的系统时机：

```toml
[capabilities]
hooks = true

[contributes]
hooks = [
  { event = "conversation.message.created", handler = "/hooks/conversation-message-created" }
]
```

系统会把 `HookEventEnvelope` POST 到 `handler`。扩展返回 `HookDispatchResponse`：

- `handled=true`：扩展已处理该事件。
- `result`：可选结构化结果，供调用方继续返回或落库。
- `message`：可选诊断说明。

系统只保证事件协议和触发时机；memory、workflow、任务规划、上下文组装等业务语义由扩展实现。

后端扩展建议补充：

- `backend.base_url`：主系统代理扩展后端时使用的目标地址
- `backend.command`：扩展运行时托管的长期后端命令
- `backend.dev_command`：开发模式覆盖命令

Provider 贡献用于声明上游接口实现。实现扩展声明 `kind`、`interfaces`、`model_discovery`、`recommended_model` 和 `manual_model`，渠道实例在 `config/providers/*.toml` 中保存用户确认后的 `default_model`。若扩展希望初始化时提供默认渠道实例，可在扩展包内放置 `provider-presets/*.toml`，由 CLI 通用扫描并写入 `config/providers/`。

## 推荐目录

```text
<extension_id>/
├─ extension.toml
├─ plugins/          # 可选：触发后执行什么
├─ hooks/            # 可选：系统时机触发声明/适配
├─ timers/           # 可选：时间触发声明/适配
├─ ui/               # 可选：页面、面板、主题、语言
├─ data/             # 可选：schema、私有模型、资源
└─ provider-presets/ # 可选：初始化上游渠道实例
```

Skill 目录独立：

```text
<skill_id>/
├─ skill.toml
├─ entry.js
└─ schemas/
```

## 运行链路

1. `cargo run -p ennoia-cli -- dev` 初始化运行目录。
2. CLI 把内置扩展同步到 `<ENNOIA_HOME>/extensions/<extension_id>/`，并更新 `config/extensions.toml`。
3. CLI 扫描内置扩展中的 `provider-presets/*.toml`，把默认渠道实例写入 `config/providers/`。
4. CLI 把仓库内 `builtins/extensions/*` 追加为开发来源，供开发模式覆盖安装目录。
5. CLI 启动 Web dev server 和扩展前端 dev command。
6. Extension Host 托管扩展后端命令，并向命令注入 `ENNOIA_HOME` / `ENNOIA_EXTENSION_ROOT`。
7. Server 暴露 runtime snapshot、事件流、诊断、日志、资源贡献接口，以及 `/api/ext/{extension_id}/{*path}` 代理入口。
8. Core 只在自身生命周期时机派发 Hook；扩展内部也可按同一规范组织自己的 Hook/Plugin。
9. Web 工作台根据 runtime snapshot 挂载页面、面板、主题、语言和命令；如果某个 mount 在本地 registry 中存在实现，则直接渲染真实组件。

## 安装与扫描目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 技能注册表：`<ENNOIA_HOME>/config/skills.toml`
- 扩展包目录：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 技能包目录：`<ENNOIA_HOME>/skills/<skill_id>/`
- 扩展私有数据目录：`<ENNOIA_HOME>/data/extensions/<extension_id>/`

扩展自己的数据库、缓存和私有运行态文件都应放在扩展私有数据目录。核心不提供主业务 SQLite，也不提供 Rust SDK；扩展通过 HTTP API、Hook envelope、配置文件和 `ENNOIA_HOME` / `ENNOIA_EXTENSION_ROOT` 接入系统。
