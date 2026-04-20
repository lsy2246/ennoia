# web-shell

## 职责

- 提供 `Ennoia` 正式控制台主壳、导航与核心管理页面
- 承载会话、空间、工作流、定时任务、记忆、扩展、Agent、产物、日志、设置等控制台视图
- 通过 `Panda CSS` 维护样式 token 与布局规则
- 生成 `styled-system` 供前端代码复用

## 行为规范

- 样式入口位于 `web/apps/shell/src/styles.css`
- 路由入口位于 `web/apps/shell/src/router.tsx`，导航壳层位于 `web/apps/shell/src/shell/AppShell.tsx`
- 共享 API 访问通过 `web/packages/api-client`，页面快照加载通过 `useWorkspaceSnapshot` 统一收口
- 多语言与主题切换依赖 `web/packages/i18n` 与 `web/packages/theme-runtime`
- `package.json` 通过 `panda:codegen`、`prepare` 保证样式代码生成

## 依赖关系

- 依赖 React / Vite / Bun
- 依赖 `@pandacss/dev` 与 `postcss`
- 依赖 `@ennoia/api-client`、`@ennoia/i18n`、`@ennoia/theme-runtime`、`@ennoia/builtins`
