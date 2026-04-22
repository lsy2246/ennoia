# Ennoia 扩展开发指南

## 定位

Extension 是系统插件包，Skill 是 Agent 可引用的能力包。两者目录、生命周期和 manifest 语义分离。

## 源码放置

- 官方内置扩展源码放在 `builtins/extensions/<extension_id>/`
- 官方内置技能源码放在 `builtins/skills/<skill_id>/`
- 运行目录里的真实包内容分别落在 `~/.ennoia/extensions/<id>/` 与 `~/.ennoia/skills/<id>/`
- 是否启用、是否卸载、来源路径统一登记在 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## Manifest

系统扩展使用 `ennoia.extension.toml`，兼容开发期的 `manifest.toml` 文件名。推荐字段：

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

Provider 贡献用于声明上游接口实现。实现扩展声明 `kind`、`interfaces`、`model_discovery`、`recommended_model` 和 `manual_model`，渠道实例在 `config/providers/*.toml` 中保存用户确认后的 `default_model`。若扩展希望初始化时提供默认渠道实例，可在扩展包内放置 `provider-presets/*.toml`，由 CLI 通用扫描并写入 `config/providers/`。

## 推荐目录

```text
<extension_id>/
├─ ennoia.extension.toml
├─ provider-presets/
├─ src/
│  ├─ frontend/
│  ├─ backend/
│  ├─ locales/
│  └─ themes/
└─ dist/
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
6. Extension Host 托管扩展后端 dev command。
7. Server 暴露 runtime snapshot、事件流、诊断、日志和资源贡献接口。
8. Web 工作台根据 runtime snapshot 挂载页面、面板、主题、语言和命令。

## 安装与扫描目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 技能注册表：`<ENNOIA_HOME>/config/skills.toml`
- 扩展包目录：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 技能包目录：`<ENNOIA_HOME>/skills/<skill_id>/`
