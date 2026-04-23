# 任务清单: wasm-extension-runtime

> **@status:** completed | 2026-04-23 11:32

```yaml
@feature: wasm-extension-runtime
@created: 2026-04-23
@status: completed
@mode: R3-DELEGATED
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 7 | 0 | 0 | 7 |

---

## 任务列表

### 1. 共享模型

- [√] 1.1 在 `crates/kernel/src/extension.rs` 中新增 `ui`、`worker`、`permissions`、`runtime` 和 RPC 类型 | depends_on: []
- [√] 1.2 在 `crates/kernel/src/lib.rs` 中更新导出 | depends_on: [1.1]

### 2. 扩展宿主

- [√] 2.1 在 `crates/extension-host/src/registry.rs` 中删除子进程 Runner 并按 Worker 解析快照 | depends_on: [1.1]
- [√] 2.2 在 `crates/extension-host/src/worker.rs` 中建立 Worker RPC 边界 | depends_on: [2.1]

### 3. Server 路由

- [√] 3.1 在 `crates/server/src/routes/extensions.rs` 中新增统一 RPC 并移除端口代理 | depends_on: [2.2]
- [√] 3.2 在 `crates/server/src/routes/behavior.rs` 与 `crates/server/src/routes/memory.rs` 中改用 Worker RPC 分发 | depends_on: [3.1]

### 4. 协议迁移与验证

- [√] 4.1 迁移内置扩展 descriptor、更新文档、运行格式化和 workspace 检查 | depends_on: [3.2]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-23 11:11 | 方案设计 | in_progress | 采用 Rust Host + 可选 UI + 可选 Wasm Worker |

---

## 执行备注

本轮不保留 process runner 兼容层；旧 HTTP service 代码暂作为源码历史留存，但不再由扩展宿主启动。

| 2026-04-23 11:40 | 开发实施 | completed | Rust fmt/check/test 通过；Web typecheck/build 因缺少 @pandacss/dev 依赖阻断 |
