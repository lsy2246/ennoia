# 变更日志

## [0.2.0] - 2026-04-23

### 重构
- **[extension-runtime]**: 一次性切换为扩展能力包模型，使用可选 `ui`、可选 Wasm `worker` 与宿主 Worker RPC，移除端口型扩展后端主路径 — by lsy
  - 方案: [202604231111_wasm-extension-runtime](archive/2026-04/202604231111_wasm-extension-runtime/)
  - 决策: wasm-extension-runtime#D001(采用 Rust Host + Wasm Worker 能力包模型)
- **[api-surface]**: 移除 REST 路径中的 `/v1` 前缀，统一使用 `/api/*`，并把扩展开发监听覆盖到 `builtins/extensions/` 与 `.wasm` 文件 — by Codex
- **[extension-runtime]**: 接入 `wasmtime` Worker runtime，支持 `ennoia.worker.v1` ABI、Module 缓存热失效、每次 RPC 隔离实例、方法前缀权限校验、内存与 fuel 预算限制 — by Codex
- **[builtin-workers]**: 为 `memory` 与 `workflow` 新增内置 Wasm Worker crate，编译生成 `memory.wasm` 与 `workflow.wasm`，并补充 `bun run build:workers` 构建入口 — by Codex

## [0.1.5] - 2026-04-22

### 重构
- **[extension-core-split]**: 核心收敛为扩展宿主，移除主业务 SQL/调度/SDK，会话、记忆、编排改为内置扩展私有存储 — by Codex
  - 方案: [202604221537_pipeline-extension-core-split](plan/202604221537_pipeline-extension-core-split/)
  - 验证: `cargo check --workspace`, `cargo test --workspace`, `bun run --cwd web typecheck`, `bun run --cwd web build`, integration/e2e smoke

- **[workspace-scripts]**: 重命名安装与打包脚本，收敛根目录脚本为安装、热开发、Web 构建/检查、npm 打包和 Docker 构建入口 — by Codex
  - 方案: 快速修改（无方案包）
  - 决策: 使用 `install:workspace` 避免 `install` 生命周期脚本歧义

## [0.1.4] - 2026-04-22

### 重构
- **[web]**: 一次性完成前端旧命名重构，统一 Docker、CLI、后端 i18n namespace、测试、文档和知识库为 Web — by Codex
  - 方案: [202604221033_web-name-refactor](archive/2026-04/202604221033_web-name-refactor/)
  - 决策: web-name-refactor#D001(不保留旧前端名称兼容层)

## [0.1.3] - 2026-04-21

### 重构
- **[platform-workbench]**: 一次性完成工作台领域重构，统一 direct/group 会话入口，加入 Agent/Skill/Provider CRUD、统一日志与 VSCode 风格 Web — by Codex
  - 方案: [202604211156_platform-workbench-total-refactor](archive/2026-04/202604211156_platform-workbench-total-refactor/)
  - 关键变更:
    - Agent 模型收敛为 `system_prompt/provider_id/model_id/reasoning_effort/workspace_root/skills/enabled`
    - Skill 与 Extension 完全分离
    - 新增 Provider 配置目录与 CRUD
    - 前后端统一日志流并支持筛选
    - Web 重构为 `工作台 / Agents / 技能 / 上游 / 扩展 / 任务 / 日志 / 设置`

- **[registry-first-web-workbench]**: 第二轮把工作台升级为 Registry-First Web Workbench，补齐 i18n、动态扩展视图、Observatory 与统一工作区根路径 — by Codex
  - 方案: [202604211308_registry-first-web-workbench](archive/2026-04/202604211308_registry-first-web-workbench/)
  - 关键变更:
    - 新增 `web.*` i18n 词条，补齐工作区、Agents、上游、扩展、任务、日志、设置与 Observatory 文案
    - `dockview` 正式承载主视图、Inspector 与扩展面板，并修复长页面分区滚动
    - 上游页改为“已实现接口类型 + 高级连接配置”，Agent 页改为选择上游并显示派生工作区
    - 设置页新增全局 `workspace_root`、时区下拉和运行时表单入口
    - `Observatory` 接入扩展事件、Runs、Jobs 与日志的真实数据聚合

## [0.1.2] - 2026-04-20

### 修复
- **[web]**: 一次性补完正式控制台，收口 i18n/主题并补齐 jobs、memories、extensions、logs 与会话删除链路 — by lsy
  - 方案: [202604201723_web-console-full-completion](archive/2026-04/202604201723_web-console-full-completion/)
  - 决策: web-console-full-completion#D001(采用“一次性控制台重构”而非增量补洞)
