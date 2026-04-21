# 变更日志

## [0.1.3] - 2026-04-21

### 重构
- **[platform-workbench]**: 一次性完成工作台领域重构，统一 direct/group 会话入口，加入 Agent/Skill/Provider CRUD、统一日志与 VSCode 风格 Shell — by Codex
  - 方案: [202604211156_platform-workbench-total-refactor](archive/2026-04/202604211156_platform-workbench-total-refactor/)
  - 关键变更:
    - Agent 模型收敛为 `system_prompt/provider_id/model_id/reasoning_effort/workspace_root/skills/enabled`
    - Skill 与 Extension 完全分离
    - 新增 Provider 配置目录与 CRUD
    - 前后端统一日志流并支持筛选
    - Shell 重构为 `工作台 / Agents / 技能 / 上游 / 扩展 / 任务 / 日志 / 设置`

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
- **[web-shell]**: 一次性补完正式控制台，收口 i18n/主题并补齐 jobs、memories、extensions、logs 与会话删除链路 — by lsy
  - 方案: [202604201723_shell-console-full-completion](archive/2026-04/202604201723_shell-console-full-completion/)
  - 决策: shell-console-full-completion#D001(采用“一次性控制台重构”而非增量补洞)
