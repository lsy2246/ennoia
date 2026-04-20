# Ennoia 扩展开发指南

## 1. 文档定位

本文档说明 Ennoia 当前扩展模型、已落地的开发约定，以及扩展运行时的统一描述协议。

如需查看更完整的设计背景，可参考 [extension-runtime-rfc.md](extension-runtime-rfc.md)。

## 2. 当前扩展分类

Ennoia 对外统一使用 `Extension` 作为扩展总称，当前内部区分：

- `system extension`
- `skill`

## 3. 当前 system extension manifest

当前 `system extension` 统一通过 `ennoia.extension.toml` 或兼容的 `manifest.toml` 暴露能力。

当前 manifest 贡献字段：

- Child Page
- Panel
- Command
- Theme
- Locale Bundle
- Provider
- Hook

字段说明：

- `pages[]`：`id`、`title`、`route`、`mount`、`icon`
- `panels[]`：`id`、`title`、`mount`、`slot`、`icon`
- `themes[]`：`id`、`label`、`appearance`、`tokens_entry`、`preview_color`、`extends`
- `locales[]`：`locale`、`namespace`、`entry`、`version`
- `commands[]`：`id`、`title`、`action`、`shortcut`
- `providers[]`：`id`、`kind`、`entry`
- `hooks[]`：`event`、`handler`

`hooks[]` 是 `contributes` 里的标准 contribution，和页面、面板、主题、语言包走同一份 manifest 协议。

## 4. 当前 skill 能力边界

当前 `skill` 可以贡献：

- Agent 或 Task 可调用的能力
- 输入输出契约
- 执行入口
- 能力声明

## 5. 当前推荐目录

### system extension

```text
<ext_id>/
├─ ennoia.extension.toml
├─ src/
│  ├─ frontend/
│  ├─ backend/
│  ├─ locales/
│  └─ themes/
└─ dist/
```

### skill

```text
<skill_id>/
├─ skill.toml
├─ entry.js
└─ schemas/
```

## 6. 当前运行生命周期

当前已落地链路如下：

1. 开发时只执行 `ennoia dev`
2. CLI 自动扫描 `./extensions/*/ennoia.extension.toml` 并写入 `~/.ennoia/extensions/attached/workspaces.toml`
3. CLI 自动启动 Shell dev server 与每个扩展的 `frontend.dev_command`
4. Extension Runtime 托管每个扩展的 `backend.dev_command`
5. Server 启动后构建 `ExtensionRuntimeSnapshot`
6. 运行中通过轮询刷新磁盘上的 descriptor 变化
7. Server 通过 `/api/v1/extensions/runtime`、`/api/v1/extensions/events`、`/api/v1/extensions/events/stream`、`/api/v1/extensions/{id}` 和 `/api/v1/extensions/{id}/diagnostics` 暴露运行时协议
8. Shell 通过事件流自动刷新 runtime snapshot 与 UI runtime

当前开发原则：

- `system extension` 负责平台扩展
- `skill` 负责能力调用
- Theme 作为 UI contribution 提供
- 页面、面板、命令与主题文案统一通过 `LocalizedText` 表达
- 页面、面板、命令和 Provider 优先通过稳定的 `mount` / `entry` 协议接入

## 7. 当前统一描述协议

扩展 descriptor 当前统一描述：

- `source`
- `frontend`
- `backend`
- `build`
- `assets`
- `watch`
- `capabilities`
- `contributes`

它既可以表达：

- 扩展来自 workspace 还是 package
- 前端是 dev URL、模块入口还是 bundle 文件
- 前端 dev server 如何通过 `frontend.dev_command` 一键启动
- 后端的开发命令、健康检查和构建入口
- 当前有哪些页面、面板、主题、语言包、命令、Provider、Hook
