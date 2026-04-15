# 项目上下文

## 基本信息

- 项目名称：`Ennoia`
- 仓库定位：AI Agent 工作台源码仓库
- 发布目标：`npm` 包 + `~/.ennoia` 配置目录

## 技术上下文

- 后端：Rust workspace
- 首版数据库：`SQLite`
- 前端：React + Vite + Bun + Panda CSS
- 运行目录：`~/.ennoia/`

## 开发约定

- 文档与代码保持一致
- 根目录 `package.json` 维护仓库级快捷命令
- `web/shell` 使用 `Panda CSS` 生成 `styled-system`

## 当前约束

- 仍处于完整骨架阶段，数据层和 UI 都以首版落地成本优先
- 未引入远程数据库依赖，避免首版部署复杂度过高
