# Ennoia 扩展开发指南

## 1. 扩展分类

Ennoia 对外统一使用 `Extension` 作为扩展总称，内部区分：

- `system extension`
- `skill`

## 2. system extension manifest

`system extension` 统一通过 `ExtensionManifest.contributes` 暴露能力。

当前 manifest 贡献字段：

- Child Page
- Panel
- Command
- Theme
- Locale Bundle
- Provider
- Hook

- `pages[]`：`id`、`title`、`route`、`mount`、`icon`
- `panels[]`：`id`、`title`、`mount`、`slot`、`icon`
- `themes[]`：`id`、`label`、`appearance`、`tokens_entry`、`preview_color`、`extends`
- `locales[]`：`locale`、`namespace`、`entry`、`version`
- `commands[]`：`id`、`title`、`action`、`shortcut`
- `providers[]`：`id`、`kind`、`entry`
- `hooks[]`：`event`、`handler`

`hooks[]` 是 `contributes` 里的一个标准 contribution，和页面、面板、主题、语言包走同一份 manifest 协议。

## 3. skill 可以贡献

- Agent 或 Task 可调用的能力
- 输入输出契约
- 执行入口
- 能力声明

## 4. 推荐目录

### system extension

```text
<ext_id>/
├─ manifest.toml
├─ backend/
└─ frontend/
```

### skill

```text
<skill_id>/
├─ skill.toml
├─ entry.js
└─ schemas/
```

## 5. 生命周期

1. 安装到 `~/.ennoia/global/extensions/` 或技能目录
2. 在 `config/extensions/*.toml` 中启用
3. Server 启动时扫描并注册
4. Server 通过 `/api/v1/extensions/registry`、`/api/v1/extensions/pages`、`/api/v1/extensions/panels` 和 `/api/v1/ui/messages` 暴露运行时协议
5. Shell 读取注册表后挂载页面、面板、命令并装载 locale bundle

## 6. 开发原则

- system extension 负责平台扩展
- skill 负责能力调用
- Theme 作为 UI contribution 提供
- 页面、面板、命令与主题文案统一通过 `LocalizedText` 表达
- 扩展采用编译安装、重启生效
- 页面、面板、命令和 Provider 优先通过稳定的 `mount` / `entry` 协议接入
