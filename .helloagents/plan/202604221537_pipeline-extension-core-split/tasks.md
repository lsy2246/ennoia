@feature: pipeline-extension-core-split
@created: 2026-04-22 15:37:51
@status: completed
@mode: interactive

## 进度概览

- 完成: 6
- 失败: 0
- 跳过: 0
- 总数: 6

## 任务列表

- [√] 1.1 统一扩展 descriptor 为 `extension.toml` | depends_on: []
- [√] 1.2 按 `plugins/hooks/timers/ui/data` 规范重组内置扩展 | depends_on: [1.1]
- [√] 1.3 新增 `session` 扩展承接会话私有存储与 API | depends_on: [1.2]
- [√] 1.4 移除核心 facts/scheduler/SDK/主业务 SQL | depends_on: [1.3]
- [√] 1.5 收窄 server 与 web 核心入口 | depends_on: [1.4]
- [√] 1.6 同步文档、测试并清理空目录 | depends_on: [1.5]

## 执行日志

- 2026-04-22 15:37:51 创建方案包并进入实施。
- 2026-04-22 17:12:00 完成核心下沉到 facts + SDK，memory/workflow 物理迁移到 builtins/extensions，并通过全量验证。
- 2026-04-22 18:40:00 按用户最终决策移除 SDK/facts/scheduler，核心收敛为扩展宿主；session/memory/workflow 改为内置扩展私有存储，并通过完整验证。

## 执行备注

- 用户最终选择：极简核心宿主 + 可替换扩展；不保留 SDK、不保留主业务 SQL、不考虑兼容旧 descriptor。
