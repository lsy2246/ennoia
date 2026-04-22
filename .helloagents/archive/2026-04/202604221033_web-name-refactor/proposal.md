# 变更提案: web-name-refactor

## 元信息

- 方案包名称: `202604221033_web-name-refactor`
- 创建日期: 2026-04-22
- 类型: implementation
- 流程: R2 简化流程

## 1. 需求

### 背景

前端项目已经迁移为 `web`，但仓库内仍残留旧前端名称相关的目录、Docker 构建路径、i18n namespace、路由变量、CSS class、测试请求、文档和知识库模块命名。用户要求完整找出，包括变量和文件名，不考虑兼容，一次性重构。

### 目标

- 将前端产品/模块语义统一重构为 `web`。
- 修正 `Dockerfile` 和 `.dockerignore` 中仍指向旧前端目录的过时路径。
- 移除未被 Git 跟踪的旧前端目录，保留当前有效入口 `web/`。
- 将内置 i18n namespace 收敛到 `web`，删除旧消息模块。
- 同步更新测试、文档、历史记录和知识库中仍指向旧前端名称的内容。

### 约束条件

- 不保留旧 namespace/路径的兼容层。
- 不修改与 PowerShell、进程执行选项、Linux 登录解释器相关的系统命令语义。
- 历史记录同步改名，确保当前仓库中只保留 `web` 作为前端产品名称。

### 验收标准

- 当前源码、配置、测试、文档、知识库和历史记录中不再存在前端产品旧名称残留。
- `Dockerfile` 使用当前 `web/` 入口完成构建并复制 `web/dist`。
- i18n 内置 namespace 使用 `web`，测试请求同步更新为 `web`。
- Rust 和前端校验命令通过，或明确记录阻断原因。

## 2. 方案

### 技术方案

- 清理旧构建路径: 将 `Dockerfile` 的 web build/copy 改为 `bun run --cwd web build` 与 `/app/web/dist`，`.dockerignore` 改为忽略 `web/dist`、`web/styled-system`。
- 收敛 i18n: 从 registry 中移除旧消息模块注册，删除旧消息模块文件，在 `web.ts` 补齐 `web.loading.connecting`。
- 重命名前端内部符号: 路由父节点、route id、CSS class、配置更新者标识统一为 `web` 语义。
- 更新后端静态消息: `builtin_message_namespaces()` 与 `builtin_message_bundle()` 的内置前端 namespace 改为 `web`。
- 更新 CLI 前端进程语义: 前端目录常量、dev 启动函数、日志名与进程标签改为 web 语义。
- 删除未跟踪旧目录: 安全校验路径位于仓库 `web/` 下后，移除旧前端目录。
- 更新测试和文档: 将 UI message namespace 请求改为 `web`，将扩展/测试/开发说明中的前端产品语义统一为 Web。
- 同步知识库和历史记录: 将当前模块文档改为 `web` 模块，并同步归档记录中的旧前端命名。

### 影响范围

- `Dockerfile`、`.dockerignore`
- `crates/cli`、`crates/server`、`crates/kernel`
- `web/src`、`web/packages/i18n`、`web/packages/ui-sdk`
- `tests/e2e`、`tests/integration`
- `docs/`、`AGENTS.md`、`.helloagents/modules`、`.helloagents/CHANGELOG.md`、`.helloagents/archive`

### 风险评估

- i18n namespace 直接改为 `web` 会破坏旧请求；这是用户明确要求的不兼容重构。
- 删除未跟踪旧目录可能影响本地缓存，但它们不是当前 Git 管理的源码，且当前有效前端入口为 `web/`。
- 需要区分前端产品命名与系统命令语义，避免误改 PowerShell、process execution 等跨平台启动逻辑。

## 3. 技术决策

### web-name-refactor#D001: 不保留旧前端命名兼容层

**背景**: 用户明确要求“不考虑兼容，一次性重构”。

**决策**: 删除旧 i18n 模块和旧目录，测试与后端 namespace 同步切到 `web`。

**影响**: 旧前端 namespace 请求不再作为内置消息入口；当前产品统一使用 `web`。
