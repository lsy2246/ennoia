# Ennoia Web

`web/` 是 Ennoia 当前唯一 Web 工作台工程。

它通过 workspace package 消费共享能力：

- `@ennoia/api-client`
- `@ennoia/contract`
- `@ennoia/observability`
- `@ennoia/ui-sdk`

当前阶段提供：

- 主壳布局
- 路由页面与资源视图拆分
- 面板区域与多视图工作台
- Extension Registry 运行时挂载
- Bun + Vite + React 构建入口
- Panda CSS 样式系统与 `styled-system` 代码生成

扩展页面组件、文案和主题归扩展自身所有。Web 主壳只通过运行时快照发现扩展贡献，并从 `/api/extensions/{extension_id}/ui/module` 动态加载扩展 UI bundle。

常用命令：

- `bun install`
- `bun run panda:codegen`
- `bun run dev`
- `bun run typecheck`
