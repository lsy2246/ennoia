# 任务清单: plugin-host-capability-rpc

```yaml
@feature: plugin-host-capability-rpc
@created: 2026-05-08
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 7 | 0 | 0 | 7 |

---

## 任务列表

### 1. 协议与运行时

- [x] 1.1 在 `crates/kernel/src/extension.rs` 中定义 host capability DTO 与 process worker 控制消息
- [x] 1.2 在 `crates/extension-host/src/worker.rs` 与 `crates/extension-host/src/registry.rs` 中实现 process worker 的 host call / host result 处理
  - 依赖: 1.1

### 2. 宿主分发层

- [x] 2.1 在 `crates/server/src/host_capabilities.rs` 中实现统一 host capability dispatcher
  - 依赖: 1.2
- [x] 2.2 在 `crates/server/src/routes/extensions.rs` 中提取 provider 共享调用逻辑，供 HTTP 与 host capability 复用
  - 依赖: 2.1

### 3. workflow 迁移

- [x] 3.1 在 `builtins/extensions/workflow/plugins/workflow-service/src` 中实现 host capability client 与 process bridge
  - 依赖: 2.2
- [x] 3.2 在 `conversation_hooks.rs` 中移除 localhost HTTP `HostApiClient`，改为新 capability client
  - 依赖: 3.1

### 4. 文档与验证

- [x] 4.1 更新扩展运行时相关文档与知识库记录
  - 依赖: 3.2
- [x] 4.2 执行 `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`
  - 依赖: 4.1

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-05-08 15:22 | 方案包初始化 | completed | 已确定平台级 host capability 重构路线 |
| 2026-05-08 15:40 | 协议与运行时 | completed | 已新增 kernel DTO、process worker 控制消息与 host dispatcher 接口 |
| 2026-05-08 15:46 | workflow 迁移 | completed | 已移除 workflow localhost HTTP 回环，改为统一 host capability 通道 |
| 2026-05-08 15:49 | 验证与文档 | completed | `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace` 全部通过 |

---

## 执行备注

- 本次不保留 localhost HTTP 兼容层
- 不新增 workflow 专用桥，统一落到插件平台协议
