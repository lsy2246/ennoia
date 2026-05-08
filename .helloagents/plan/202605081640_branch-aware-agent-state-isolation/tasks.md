# 任务清单: branch-aware-agent-state-isolation

```yaml
@feature: branch-aware-agent-state-isolation
@created: 2026-05-08
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 6 | 0 | 0 | 6 |

---

## 任务列表

### 1. workflow 状态模型重构

- [√] 1.1 在 `builtins/extensions/workflow/plugins/workflow-service/src/conversation_hooks.rs` 中把 active workflow session、receipt 和恢复上下文升级为 branch-aware 状态模型 | depends_on: []
- [√] 1.2 在 `builtins/extensions/workflow/plugins/workflow-service/src/conversation_hooks.rs` 中修正基于消息/分支的 session 解析与 resume 路径，避免同 agent 跨分支串线 | depends_on: [1.1]

### 2. Web thinking 隔离与会话展示

- [√] 2.1 在 `web/src/views/conversations/Session.tsx` 中为 pending reply marker 增加 branch 维度，并在读取/持久化/渲染时只保留当前分支 thinking | depends_on: [1.1]
- [√] 2.2 在 `web/src/views/conversations/chat-entry-builder.ts` 中按 branch-aware 规则构建状态条目，避免旧分支状态污染当前流 | depends_on: [2.1]

### 3. 查询链与验证

- [√] 3.1 审查 `conversation.get` / `run.list` / stream 装配路径，补齐 branch-aware 依赖点，降低卡住时对新会话和分支查询的拖累 | depends_on: [1.2]
- [√] 3.2 执行格式化、编译与测试验证，确认本次修复覆盖 thinking 串扰与 API 稳定性回归 | depends_on: [2.2, 3.1]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-05-08 16:40 | 方案设计 | completed | 已选定“后端状态模型优先重构”方案并创建 implementation 方案包 |
| 2026-05-08 17:35 | 开发实施 | completed | 已完成 branch-aware workflow session / pending thinking 隔离，并通过 cargo + web 验证 |

---

## 执行备注

> 记录执行过程中的重要说明、决策变更、风险提示等

- 本次不做旧 session key / 旧 localStorage 结构兼容，按用户要求一次性切换到 branch-aware 语义。
- 如果开发阶段发现 API 超时并非全部来自状态串扰，而是 extension 查询链还有额外阻塞，将在任务 3.1 中顺手补齐。
