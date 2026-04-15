# 变更提案: sqlite-panda-bootstrap

## 元信息
```yaml
类型: 优化
方案类型: implementation
优先级: P1
状态: 已完成
创建: 2026-04-15
```

---

## 1. 需求

### 背景
当前仓库文档里仍保留 “PostgreSQL 为主” 的表述，但打包模板和运行目录已经明显偏向本地 `SQLite`。前端主壳也还停留在手写全局 CSS 阶段，缺少后续做 shell token、主题约束和布局复用的统一样式工具。

### 目标
- 把首版数据层方向收敛为 `SQLite-first`
- 在 `web/shell` 引入 `Panda CSS`
- 在根目录提供新机器首次拉仓库可直接执行的 bootstrap 命令

### 约束条件
```yaml
时间约束: 本轮在现有骨架基础上最小改动完成
性能约束: 不改变现有 API 行为，仅调整样式实现与开发入口
兼容性约束: 保持 React + Vite + Bun 现有开发方式
业务约束: 暂不引入远程数据库或更重的前端组件库
```

### 验收标准
- [x] README 和相关文档改为 `SQLite-first`
- [x] `web/shell` 接入 `Panda CSS` 并通过类型检查与构建
- [x] 根目录新增 `bun run bootstrap`

---

## 2. 方案

### 技术方案
在 `web/shell` 增加 `@pandacss/dev`、`postcss`、`panda.config.ts`、`postcss.config.cjs` 和 `app.styles.ts`，把原本的全局类样式改为 `Panda CSS` 生成的样式对象。根目录 `package.json` 增加 `bootstrap` 命令，串联根依赖安装、前端依赖安装、Rust 校验和前端 typecheck。文档层将 README、架构、路线图和配置模型统一更新为 `SQLite-first + Panda CSS` 方案。

### 影响范围
```yaml
涉及模块:
  - web-shell: 接入 Panda CSS 并重写主壳样式入口
  - workspace-docs: 收敛数据库与前端方案描述
预计变更文件: 10+
```

### 风险评估
| 风险 | 等级 | 应对 |
|------|------|------|
| Panda 代码生成未接入脚本 | 中 | 在 `prepare` / `typecheck` / `build` 中显式调用 `panda:codegen` |
| 文档与模板再次不一致 | 中 | 同步更新 README、架构文档和配置模型 |

---

## 3. 技术设计

### 前端样式层

- `panda.config.ts`：维护 token 与全局样式
- `src/styles.css`：注入 Panda layer
- `src/app.styles.ts`：沉淀主壳布局与表单面板样式
- `src/App.tsx`：改为消费样式对象

### 开发入口

- 根目录 `package.json`：新增 `bootstrap`
- `web/shell/package.json`：新增 `panda:codegen`、`prepare`

---

## 4. 核心场景

> 执行完成后同步到对应模块文档

### 场景: 新开发者初始化仓库
**模块**: workspace-docs
**条件**: 已安装 Bun 与 Rust 工具链
**行为**: 在仓库根目录执行 `bun run bootstrap`
**结果**: 完成前端依赖安装、Panda 代码生成、Rust 校验和前端 typecheck

### 场景: Shell 页面开发
**模块**: web-shell
**条件**: 进入 `web/shell`
**行为**: 使用 `Panda CSS` token 和样式对象维护 shell 页面布局
**结果**: 样式更易复用，后续主题和布局规则更容易收敛

---

## 5. 技术决策

> 本方案涉及的技术决策，归档后成为决策的唯一完整记录

### sqlite-panda-bootstrap#D001: 首版采用 SQLite-first + Panda CSS
**日期**: 2026-04-15
**状态**: ✅采纳
**背景**: 首版目标是本地优先骨架，不希望因为数据库和前端样式基础设施把落地成本抬高。
**选项分析**:
| 选项 | 优点 | 缺点 |
|------|------|------|
| A: PostgreSQL + 手写 CSS | 后续服务化路径明确 | 首版过重，样式体系容易分散 |
| B: SQLite + Panda CSS | 本地优先、样式可收敛、接入成本更低 | 需要增加代码生成步骤 |
**决策**: 选择方案 B
**理由**: 更符合当前骨架阶段“先跑通本地工作台、后扩远程能力”的节奏。
**影响**: README、架构文档、根目录命令和 `web/shell` 样式实现同步变化

---

## 6. 成果设计

> 含视觉产出的任务由 DESIGN Phase2 填充。非视觉任务整节标注"N/A"。

### 设计方向
- **美学基调**: 温暖纸感工作台，强调磨砂层次和编辑台氛围
- **记忆点**: 带铜色眉题和纸张质感的 shell 面板
- **参考**: 延续现有米色 + 天空蓝发光背景，但改成 token 化样式系统

### 视觉要素
- **配色**: 米色背景 + 深墨主色 + 铜色眉题 + 天空蓝冷光
- **字体**: 标题使用衬线展示字，正文使用系统无衬线，提高工作台辨识度
- **布局**: 左侧导航 + 右侧工作台，卡片面板保持柔和层次
- **动效**: 按钮 hover 轻微上浮，避免大量微动效
- **氛围**: 径向光斑背景 + 毛玻璃卡片 + 低饱和阴影

### 技术约束
- **可访问性**: 保持按钮、输入框、选择框的 focus 可见状态
- **响应式**: 小屏时 shell 改为单列布局
