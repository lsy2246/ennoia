# 任务清单: shell-console-full-completion

> **@status:** completed | 2026-04-20 17:43

```yaml
@feature: shell-console-full-completion
@created: 2026-04-20
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 18 | 0 | 0 | 18 |

---

## 任务列表

### 1. 控制台壳层与路由重构

- [√] 1.1 重构 `web/apps/shell/src/shell/AppShell.tsx` 与 `web/apps/shell/src/router.tsx`，建立正式控制台导航与页面分组 | depends_on: []
- [√] 1.2 新增控制台共享导航/页面头部/空态组件，统一列表页骨架 | depends_on: [1.1]
- [√] 1.3 调整 `web/apps/shell/src/styles.css`，让控制台结构、表格、卡片、状态标签和响应式布局匹配新壳层 | depends_on: [1.1,1.2]

### 2. i18n 与主题体系收口

- [√] 2.1 扩充 `web/packages/i18n/src/modules/*`，补齐导航、页面标题、动作按钮、空态与状态文案 key | depends_on: [1.1]
- [√] 2.2 重构 `web/apps/shell/src/stores/ui.ts` 与相关页面消费方式，移除页面散落硬编码 | depends_on: [2.1]
- [√] 2.3 调整 `web/packages/theme-runtime/src/index.ts` 与设置页逻辑，统一内置主题与扩展主题展示、切换和应用 | depends_on: [2.2]

### 3. 控制台能力补完

- [√] 3.1 重构 `web/apps/shell/src/pages/ConversationsPage.tsx`，补齐会话删除入口、刷新策略和更清晰的会话管理视图 | depends_on: [1.2,2.2]
- [√] 3.2 新增 `JobsPage` 并接入 `/api/v1/jobs` 的列表与创建能力，显式展示 schedule 机制 | depends_on: [1.2,2.2]
- [√] 3.3 新增 `MemoriesPage`，接入列表、recall、review 入口，让记忆能力可见可操作 | depends_on: [1.2,2.2]
- [√] 3.4 新增 `ExtensionsPage`，展示 extensions/pages/panels/themes/locales，并纳入技能与扩展目录视图 | depends_on: [1.2,2.2,2.3]
- [√] 3.5 升级 `AgentsPage.tsx` 与 `SettingsPage.tsx`，把 agent 配置、skills 路径、运行目录、logging 设置整合成正式管理页 | depends_on: [1.2,2.2]
- [√] 3.6 新增 `LogsPage`，形成基础日志浏览与问题定位视图 | depends_on: [1.2,2.2]

### 4. 后端与 API 补口

- [√] 4.1 在 `crates/server/src/routes.rs`、`crates/server/src/db.rs` 与相关模型中补齐会话删除链路 | depends_on: [3.1]
- [√] 4.2 在 `crates/server` 中补日志读取接口，暴露 shell 所需最小日志查询能力 | depends_on: [3.6]
- [√] 4.3 在 `web/packages/api-client/src/index.ts` 中补齐会话删除、日志查询及新增控制台页面所需类型/请求函数 | depends_on: [4.1,4.2]

### 5. 文档与验证收口

- [√] 5.1 同步 `README.md`、`docs/architecture.md`、`docs/runtime-layout.md`、`docs/api-surface.md`，更新正式控制台能力描述 | depends_on: [3.2,3.3,3.4,3.5,3.6,4.1,4.2]
- [√] 5.2 执行 `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`、`bun run --cwd web/apps/shell typecheck`、`bun run --cwd web/apps/shell build` 并修正本轮问题 | depends_on: [5.1]
- [√] 5.3 回归检查最终控制台闭环，确认用户提出的 4 类问题均被覆盖解决 | depends_on: [5.2]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-20 17:23 | 方案设计 | 已完成 | 已确认采用“一次性控制台重构”方案，进入开发实施 |
| 2026-04-20 17:49 | 1.1-3.6 | 已完成 | 控制台导航、骨架页面、i18n/主题收口和 Jobs/Memories/Extensions/Logs 全部接入完成 |
| 2026-04-20 17:58 | 4.1-5.3 | 已完成 | 会话删除、日志 API、文档同步与全量验证链通过 |

---

## 执行备注

> 记录执行过程中的重要说明、决策变更、风险提示等

- 本轮不为旧壳结构保留兼容层，允许直接重组导航、页面与文案体系
- 技能页优先以“配置与目录视图”纳入扩展管理面，不额外虚构未存在的独立技能后端
- 日志能力以“最小正式可用”接口为第一目标，先解决可见性与排障问题
- 所有计划任务均已完成，待归档到 `archive/2026-04/`
