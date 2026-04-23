# web

## 职责

- 提供 VSCode 风格的 Web 工作台与多面板布局
- 提供工作台、Agent、技能、上游、扩展、任务、日志、设置页面
- 统一承载 direct/group 会话创建与消息流展示
- 通过 registry 合并内建视图与扩展视图，并用 `dockview` 提供拖拽停靠布局
- Observatory 已作为正式 Web 视图接入真实运行数据
- 承接前端运行时日志上报

## 行为规范

- 路由入口位于 `web/src/router.tsx`
- 主工作台位于 `web/src/App.tsx`
- 样式入口位于 `web/src/styles.css`
- 共享 API 访问统一收口到 `web/packages/api-client/src/index.ts`
- 工作台支持 `@agent` 消息路由，不再使用“目标”输入框
- 内置前端 i18n namespace 使用 `web`
- 扩展 UI 模块加载使用 `/api/extensions/{extension_id}/ui/module`

## 依赖关系

- 依赖 React / Vite / Bun
- 依赖 `@ennoia/api-client`
- 依赖 `@ennoia/i18n`、`@ennoia/theme-runtime`
