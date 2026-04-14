# Ennoia 扩展开发指南

## 1. 扩展分类

Ennoia 对外统一使用 `Extension` 作为扩展总称，内部区分：

- `system extension`
- `skill`

## 2. system extension 可以贡献

- Hook
- Child Page
- Panel
- Command
- Theme
- Provider

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
4. Shell 读取注册表后挂载页面、面板和命令

## 6. 开发原则

- system extension 负责平台扩展
- skill 负责能力调用
- Theme 作为 UI contribution 提供
- 扩展采用编译安装、重启生效
