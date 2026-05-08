# 任务清单: conversation-not-found-fix

> **@status:** completed | 2026-05-07 16:45

```yaml
@feature: conversation-not-found-fix
@created: 2026-05-07
@status: completed
@mode: R2
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 4 | 0 | 0 | 4 |

---

## 任务列表

### 1. 后端错误语义修复

- [√] 1.1 在 `crates/server/src/routes/actions.rs` 中补充扩展错误到 `ApiError` 的 not found 映射 | depends_on: []

### 2. 前端会话失效兜底

- [√] 2.1 在 `web/src/views/conversations/Session.tsx` 中抽取会话失效判断与统一处理逻辑 | depends_on: []
- [√] 2.2 在会话加载、刷新、发消息、分支切换与删除等入口复用统一处理逻辑 | depends_on: [2.1]

### 3. 验证与知识库同步

- [√] 3.1 运行相关格式化与检查，并同步 `.helloagents` 记录 | depends_on: [1.1, 2.2]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-05-07 16:34 | 方案包初始化 | completed | 已创建 proposal.md 与 tasks.md |
| 2026-05-07 16:38 | 1.1 | completed | `actions.rs` 已将扩展 `*_not_found` 错误映射为 `ApiError::not_found`，并补充单元测试 |
| 2026-05-07 16:41 | 2.1/2.2 | completed | `SessionView` 已统一识别会话失效错误并回收 stale panel |
| 2026-05-07 16:44 | 3.1 | completed | 已执行 `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`、`bun run --cwd web lint`、`bun run --cwd web typecheck`、`bun run --cwd web build` |

---

## 执行备注

> 本次修复采用“后端保留错误语义 + 前端按业务语义优先识别”的双端方案。
> `bun run --cwd web lint` 通过，但保留仓库中既有的 3 条 `react-refresh/only-export-components` warning，和本次改动无关。
