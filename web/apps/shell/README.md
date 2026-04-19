# Ennoia Shell

`web/apps/shell` 是 Ennoia 的前端主壳工程。

它通过 workspace package 消费共享能力：

- `@ennoia/api-client`
- `@ennoia/contract`
- `@ennoia/observability`
- `@ennoia/ui-sdk`
- `@ennoia/builtins`

当前阶段提供：

- 主壳布局
- 子页面导航
- 面板区域占位
- Extension Registry 占位数据
- Bun + Vite + React 构建入口
- Panda CSS 样式系统与 `styled-system` 代码生成

常用命令：

- `bun install`
- `bun run panda:codegen`
- `bun run dev`
- `bun run typecheck`
