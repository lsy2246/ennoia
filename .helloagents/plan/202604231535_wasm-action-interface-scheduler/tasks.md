# 任务清单: wasm-action-interface-scheduler

```yaml
@feature: wasm-action-interface-scheduler
@created: 2026-04-23
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 12 | 0 | 0 | 12 |

## 任务列表

### 1. 核心协议

- [x] 1.1 在 `crates/kernel/src/extension.rs` 增加 `interfaces` 与 `schedule_actions` 贡献模型
- [x] 1.2 在 `crates/kernel/src/config.rs` 增加 `InterfaceBindingsConfig` 并删除 `JournalConfig`
- [x] 1.3 在 `crates/paths/src/lib.rs` 增加 `interfaces_config_file` 与 `schedules_file`，移除 journal 路径

### 2. 扩展运行时

- [x] 2.1 在 `crates/extension-host/src/registry.rs` 展开接口实现和定时动作 snapshot
- [x] 2.2 在 `crates/extension-host/src/worker.rs` 把接口 method 与 schedule action method 纳入 RPC 授权

### 3. Server 路由

- [x] 3.1 删除 `crates/server/src/routes/journal.rs` 与 `journal_lock`
- [x] 3.2 新增 `crates/server/src/routes/interfaces.rs`，让 Conversation/Run API 通过接口绑定调用 Wasm Worker
- [x] 3.3 新增 `crates/server/src/routes/schedules.rs`，实现 schedule CRUD、手动运行、暂停恢复和后台 tick

### 4. 内置扩展与前端包

- [x] 4.1 更新 `builtins/extensions/memory/extension.toml` 与 Worker 会话/消息接口响应
- [x] 4.2 更新 `builtins/extensions/workflow/extension.toml` 与 Worker run/task/artifact/schedule 响应
- [x] 4.3 更新 `web/packages/ui-sdk`、`web/packages/api-client` 类型与 API 封装

### 5. 文档与验证

- [x] 5.1 同步 README、架构、API、运行目录、数据模型、扩展开发和 runtime RFC
- [x] 5.2 同步 `.helloagents` 知识库和本方案包

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-23 | 核心接口与 scheduler 重构 | 完成 | 删除 journal，新增 interfaces/schedules |
| 2026-04-23 | 文档与 API client 同步 | 完成 | 补齐无 `/api/v1`、Wasm Worker 扩展语义 |

## 执行备注

- 内置 `memory` / `workflow` Worker 当前是 Wasm 示例实现，后续真实持久化应通过扩展私有存储或 host capability bridge 落地。
- 旧误建方案包 `.helloagents/plan/202604231049_openclaw-inspired-timer-system` 保留未删除，可后续清理或归档为废弃。
