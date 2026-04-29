# web

## 职责

- 提供 VSCode 风格的 Web 工作台与多面板布局
- 提供工作台、Agent、技能、上游、扩展、任务、日志、设置页面
- API 上游渠道不再作为主导航独立视图，改为设置页中的配置区
- 定时器作为主导航独立视图，管理扩展 `schedule_actions` 对应的 schedule
- 统一承载 direct/group 会话创建与消息流展示
- 通过 registry 合并内建视图与扩展视图，并用 `dockview` 提供拖拽停靠布局
- Observatory 已作为正式 Web 视图接入真实运行数据
- 承接前端运行时日志上报

## 行为规范

- 路由入口位于 `web/src/router.tsx`
- 主工作台位于 `web/src/App.tsx`
- 样式入口位于 `web/src/styles.css`
- 共享 API 访问统一收口到 `web/packages/api-client/src/index.ts`
- API client 已提供 interface binding 与 scheduler 封装：`interfaces.ts`、`schedules.ts`
- `web` 提供 `bun run --cwd web lint`，基于 ESLint flat config 检查 `src` 与 `packages`
- `web` 构建脚本与 `scripts/build-extension-ui.mjs` 优先从 `web/node_modules` 解析 `vite`、`@vitejs/plugin-react`、`typescript` 与 `@pandacss/dev`，避免依赖根目录 `node_modules` 的隐式假设
- 定时器视图位于 `web/src/pages/schedules.tsx`，依赖 `listScheduleActions` / `listSchedules` 等 API client 方法；创建时支持“交给 AI 执行”和“直接运行命令”
- 工作台支持 `@agent` 消息路由，不再使用“目标”输入框
- 内置前端 i18n namespace 使用 `web`
- 扩展 UI 模块加载使用 `/api/extensions/{extension_id}/ui/module`
- Docker Web 运行时通过 nginx 同源反代 `/api/*` 到 `api:3710`，其中 `/api/extensions/events/stream` 关闭 buffering 以保证 SSE 实时性
- Docker API 镜像在构建时会先产出 Linux 版内置 process worker，并覆盖 `builtins/extensions/*/bin/` 的运行时入口，避免扩展详情页因宿主平台 `.exe` 资产被误打包而出现 `has no worker`

## 依赖关系

- 依赖 React / Vite / Bun
- 依赖 `@ennoia/api-client`
- 依赖 `@ennoia/i18n`、`@ennoia/theme-runtime`
