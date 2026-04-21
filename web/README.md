# Ennoia Web

`web/` 是 Ennoia 当前唯一 Web 工作台工程。

它通过 workspace package 消费共享能力：

- `@ennoia/api-client`
- `@ennoia/contract`
- `@ennoia/observability`
- `@ennoia/ui-sdk`
- `@ennoia/builtins`

当前阶段提供：

- 主壳布局
- 路由页面与资源视图拆分
- 面板区域与多视图工作台
- Extension Registry 挂载
- Bun + Vite + React 构建入口
- Panda CSS 样式系统与 `styled-system` 代码生成

常用命令：

- `bun install`
- `bun run panda:codegen`
- `bun run dev`
- `bun run typecheck`
