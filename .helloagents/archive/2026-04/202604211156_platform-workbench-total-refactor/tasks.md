# 任务清单: platform-workbench-total-refactor

```yaml
@feature: platform-workbench-total-refactor
@created: 2026-04-21
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 16 | 0 | 0 | 16 |

---

## 任务列表

### 1. 方案包与领域建模

- [√] 1.1 更新方案包中的 proposal/tasks，固化本次一次性重构范围与最终模型 | depends_on: []
- [√] 1.2 重构 `crates/kernel/src/config.rs` 与相关共享类型，补齐 Agent / Skill / Provider 配置模型 | depends_on: [1.1]
- [√] 1.3 扩展 `crates/paths/src/lib.rs` 与初始化模板目录结构，加入 skills/providers 配置目录与平台感知路径展示 | depends_on: [1.2]

### 2. 后端接口与持久化

- [√] 2.1 在 `crates/server/src/app.rs` 和 `crates/server/src/routes.rs` 中加入 Agent / Skill / Provider 的读取、创建、更新、删除接口 | depends_on: [1.2,1.3]
- [√] 2.2 重构会话相关后端辅助逻辑，支持工作台统一会话创建与基于实时 Agent 列表的校验 | depends_on: [2.1]
- [√] 2.3 扩展日志能力，加入前端日志写入与统一查询过滤 | depends_on: [2.1]
- [√] 2.4 调整任务接口的数据表达，使任务页可表示 AI 任务 / 命令任务 / 超时 / 删除 / 会话投递 | depends_on: [2.1]

### 3. API Client 与前端数据层

- [√] 3.1 重构 `web/packages/api-client/src/index.ts`，新增 Agent / Skill / Provider / Workbench / Log / Task 所需类型与请求函数 | depends_on: [2.1,2.2,2.3,2.4]
- [√] 3.2 为前端增加日志上报与会话消息 `@agent` 路由辅助逻辑 | depends_on: [3.1]

### 4. Web 工作台与页面重构

- [√] 4.1 重构 `web/src/router.tsx` 与 `web/src/web/AppWeb.tsx`，建立 VSCode 风格工作台导航与布局 | depends_on: [3.1]
- [√] 4.2 重写工作台会话页，统一新建会话入口、消息流、右侧检查器与底部面板 | depends_on: [4.1,3.2]
- [√] 4.3 重写 Agent、技能、扩展、Provider、任务、日志、设置页面，使之匹配新的领域边界 | depends_on: [4.1,3.1]
- [√] 4.4 重写 `web/src/styles.css` 与相关共享组件样式，形成完整控制台视觉系统 | depends_on: [4.1,4.2,4.3]

### 5. 模板、文档与示例同步

- [√] 5.1 更新 `assets/templates/config/*`、`.env.example` 与初始化逻辑，修正默认模板与跨平台路径说明 | depends_on: [1.3]
- [√] 5.2 更新 `README.md`、`docs/architecture.md`、`docs/runtime-layout.md`、`docs/api-surface.md`，同步新模型和新页面结构 | depends_on: [4.3,5.1]

### 6. 验证与归档

- [√] 6.1 执行 Rust 与 Web 验证链，修复本轮回归问题 | depends_on: [2.4,4.4,5.2]
- [√] 6.2 更新知识库模块文档与 CHANGELOG，完成方案包归档 | depends_on: [6.1]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-21 11:56 | 1.1 | 已完成 | 已创建方案包并固化本次一次性重构范围 |
| 2026-04-21 14:20 | 1.2-5.2 | 已完成 | 完成模型、API、Web、文档与模板重构 |
| 2026-04-21 14:26 | 6.1-6.2 | 已完成 | 已执行 Rust/Web 验证链并同步知识库 |

---

## 执行备注

> 记录执行过程中的重要说明、决策变更、风险提示等

- 本轮明确不保留兼容层，允许旧路由、旧页面、旧配置心智模型被整体替换
- 技能与扩展边界已经固定：技能只服务 Agent，扩展只服务系统
- 本轮不设计 handoff_constraints 静态约束模型，转交能力留作运行时行为
