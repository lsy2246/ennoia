# 项目上下文

## 基本信息

- 项目名称：`Ennoia`
- 仓库定位：本地多 Agent 工作台源码仓库
- 发布目标：`npm` 包 + 平台运行目录（默认 `.ennoia`）

## 当前技术上下文

- 后端：Rust workspace
- 数据层：`SQLite`
- 前端：React + Vite + Bun
- 运行目录：`ENNOIA_HOME` 或默认用户目录下 `.ennoia`

## 当前产品上下文

- 工作台统一承载 `direct/group` 会话
- `1 Agent = direct`，`2+ Agents = group`
- Agent / Skill / Provider / Extension 已拆分成独立边界
- Web 术语已经替代旧前端名称
- 工作区根路径只在全局设置维护，Agent/Space 路径自动派生
- 计划任务分为 `AI 任务` 与 `命令任务`
- 日志统一聚合前后端

## 开发约定

- 文档与代码保持一致
- 运行时配置优先表单化，而不是直接编辑 JSON
- Windows 展示路径不应默认显示 `~/.ennoia`
