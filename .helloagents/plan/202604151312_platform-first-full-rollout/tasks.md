# 任务清单: platform-first-full-rollout

```yaml
@feature: platform-first-full-rollout
@created: 2026-04-15
@status: in_progress
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 19 | 0 | 0 | 19 |

---

## 任务列表

### 1. Phase 1 平台基础设施正式化

- [√] 1.1 统一 `extension-host` 的 registry 输出模型，补 page/panel/command/hook/provider 的正式挂载协议 | depends_on: []
- [√] 1.2 强化 `crates/cli` 的 `init/dev/start`，让 runtime 初始化、server 启动和默认扩展装配形成正式入口 | depends_on: [1.1]
- [√] 1.3 校正 `packaging/home-template` 与 `packaging/npm`，使 runtime 模板、扩展安装位和 npm 入口一致 | depends_on: [1.2]
- [√] 1.4 将 `tests/integration`、`tests/e2e` 从空壳升级为真实测试入口，并扩展 `.github/workflows/ci.yml` 覆盖基础校验链 | depends_on: [1.2,1.3]
- [√] 1.5 同步 Phase 1 涉及文档：`README`、`runtime-layout`、`extension-development`、`api-surface` | depends_on: [1.1,1.2,1.3]

### 2. Phase 2 领域与后端正式化

- [√] 2.1 在 `kernel` 中补齐正式 `Thread`、`Message`、`RunStatus`、`TaskStatus` 及 extension contribution 契约 | depends_on: [1.5]
- [√] 2.2 升级 SQLite schema 与 `server/db` repository，新增 `threads/messages/artifacts` 等正式表与查询接口 | depends_on: [2.1]
- [√] 2.3 重构 `server/routes` 与服务分层，提供 conversation、runs/tasks、memory、extension registry 正式 API | depends_on: [2.2]
- [√] 2.4 重构 `orchestrator`，让私聊/群聊进入统一 run/task 生命周期，而不是仅创建 planned run | depends_on: [2.1,2.3]
- [√] 2.5 重构 `memory`，形成按 owner/thread/run 的 recall/remember/context assembly 规范 | depends_on: [2.1,2.3]
- [√] 2.6 为私聊、群聊、memory、run/task、extension registry API 补 integration tests | depends_on: [2.3,2.4,2.5]
- [√] 2.7 同步 Phase 2 文档：`data-model`、`architecture`、`api-surface` | depends_on: [2.2,2.3,2.4,2.5]

### 3. Phase 3 产品工作台正式化

- [√] 3.1 重构 `web` 为正式 workspace，拆出导航、会话区、运行面板、记忆面板和 extension 容器 | depends_on: [2.7]
- [√] 3.2 接入私聊正式链路：thread/message 查询、消息发送、run/task 状态展示、memory 摘要展示 | depends_on: [3.1]
- [√] 3.3 接入群聊正式链路：Space 线程、参与 Agent、群聊运行态与共享记忆展示 | depends_on: [3.1,3.2]
- [√] 3.4 建立 extension page/panel 动态挂载容器，并消费后端 registry 协议完成正式挂载 | depends_on: [1.1,2.3,3.1]
- [√] 3.5 在 `web/builtins` / `web/ui-sdk` 中补齐最小正式接入点，避免 web 直接写死扩展实现 | depends_on: [3.4]
- [√] 3.6 为 workspace、私聊、群聊、extension mount 补 e2e / smoke 验证 | depends_on: [3.2,3.3,3.4,3.5]
- [√] 3.7 完成最终文档收口与回归验证，确认方案范围内功能已一次性推进完成 | depends_on: [3.6]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-15 15:35 | 1.1-1.3 | 已完成 | extension registry 正式化、CLI runtime 路径解析与幂等 init、runtime 模板与 npm 入口对齐 |
| 2026-04-15 15:48 | 1.4-1.5 | 已完成 | integration/e2e smoke 与 CI 接入完成，README/runtime/API/extension 文档同步完成 |
| 2026-04-15 16:30 | 2.1-2.5 | 已完成 | kernel/memory/orchestrator/db 统一为正式线程、消息、run、task、artifact、memory 模型 |
| 2026-04-15 16:42 | 2.6-2.7 | 已完成 | conversation API、Phase 2 smoke 验证与 data-model/architecture/api-surface 文档同步完成 |
| 2026-04-18 10:28 | 3.1-3.5 | 已完成 | web 重构为正式 workspace，私聊/群聊/extension surface 接入完成，builtins 与 ui-sdk 提供最小正式挂载描述 |
| 2026-04-18 10:43 | 3.6-3.7 | 已完成 | workspace smoke/e2e 全绿，README/architecture/api-surface/data-model/runtime-layout 与方案包状态同步完成 |

---

## 执行备注

- 本方案为“全量总体方案 + 分阶段落地”，阶段是依赖顺序，不是范围缩减
- Phase 1 完成前不进入 UI 正式化，避免前端与不稳定后端契约耦合
- 当前进度已完成 Phase 1、Phase 2 与 Phase 3，方案范围内的正式能力已在同一方案包中一次性推进完成
- 最终完成定义以 3.7 为准，当前状态已满足“三阶段全部完成”的收口条件
