# web-shell

## 职责

- 提供 `Ennoia` 主壳页面和运行时面板
- 通过 `Panda CSS` 维护样式 token 与布局规则
- 生成 `styled-system` 供前端代码复用

## 行为规范

- 样式入口位于 `web/shell/src/styles.css`
- 主题 token 和全局样式定义在 `web/shell/panda.config.ts`
- `package.json` 通过 `panda:codegen`、`prepare` 保证样式代码生成

## 依赖关系

- 依赖 React / Vite / Bun
- 依赖 `@pandacss/dev` 与 `postcss`
