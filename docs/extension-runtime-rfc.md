# Extension Runtime RFC

本文档记录当前 Extension Runtime 的已落地约定。历史设计中的 `global/extensions`、`packages/extensions`、旧附加来源注册表、Skill/Extension 混合模型、端口型扩展后端已经废弃。

## 当前目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 安装扩展包：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 扩展私有数据：`<ENNOIA_HOME>/data/extensions/<extension_id>/`

## 当前协议

Extension 使用 `extension.toml` 描述系统能力包。Skill 使用 `skill.toml` 描述 Agent 能力包，两者互不兼容、互不混用。

Extension descriptor 包含：

- `source`
- `ui`
- `worker`
- `permissions`
- `runtime`
- `build`
- `assets`
- `watch`
- `capabilities`
- `contributes`

贡献类型包含：页面、面板、主题、语言包、命令、Provider、Behavior、Memory 和 Hook。

`ui` 是可选界面入口；`worker` 是可选 Wasm 执行单元。宿主按声明装配能力，不要求扩展同时包含 UI 和 Worker。

## 运行流程

1. CLI 初始化运行目录和默认配置。
2. CLI 同步内置扩展到 `<ENNOIA_HOME>/extensions/*`，并写入 `config/extensions.toml`。
3. 开发模式下 CLI 把仓库内 `builtins/extensions/*` 追加为开发来源。
4. Extension Host 扫描 `config/extensions.toml` 中启用且未移除的扩展来源。
5. Extension Host 解析 `ui`、`worker`、权限和贡献清单，生成 runtime snapshot。
6. Server 暴露 runtime snapshot、事件、诊断、日志、资源贡献接口和 Worker RPC。
7. Web 工作台通过 runtime snapshot 动态挂载扩展贡献。

扩展源码推荐目录为 `ui/`、`worker/`、`data/` 和 `provider-presets/`。这些目录不是必备项，扩展包只声明实际提供的能力。

## 开发热加载

- CLI 开发模式监听 `crates/`、`assets/`、`builtins/extensions/`、`Cargo.toml` 和 `Cargo.lock`。
- `builtins/extensions/` 内的 `.wasm`、manifest、UI 与资源文件变化会触发 Host 重新构建并重启 API 进程。
- Server 运行时按 2 秒轮询刷新扩展注册表和 manifest，更新 runtime snapshot。
- Worker runtime 会缓存编译后的 Wasm Module，并在 `.wasm` mtime 或文件大小变化时自动重新编译。
- 每次 RPC 调用创建新的 Wasm 实例，避免跨请求共享线性内存状态。

## Worker ABI

当前宿主支持 `ennoia.worker.v1`。Wasm Worker 必须导出：

- `memory`：线性内存。
- `ennoia_worker_alloc(len: i32) -> i32`：分配输入/输出缓冲区。
- `ennoia_worker_dealloc(ptr: i32, len: i32)`：释放缓冲区；无 GC 语言可以实现为空操作。
- `ennoia_worker_handle(ptr: i32, len: i32) -> i64`：处理一次 RPC 调用。

宿主写入 `ennoia_worker_handle` 的输入是 UTF-8 JSON：

```json
{
  "method": "memory/recall",
  "params": {},
  "context": {}
}
```

`ennoia_worker_handle` 返回值按高 32 位为 `ptr`、低 32 位为 `len` 打包，指向 UTF-8 JSON 响应。响应推荐直接使用 `ExtensionRpcResponse`：

```json
{
  "ok": true,
  "data": {},
  "error": null
}
```

如果 Worker 返回普通 JSON，宿主会把它包装为 `ok=true` 的 `data`。

内置 `memory` 与 `workflow` 都提供 `ennoia.worker.v1` Worker crate。执行 `bun run build:workers` 会构建两个 release Wasm，并写入 manifest 中声明的 `worker/memory.wasm` 与 `worker/workflow.wasm`。

## 沙箱与权限

- 默认不注入 WASI，也不允许任意 host import；声明了 import 的模块会被拒绝实例化。
- RPC 方法必须匹配 manifest 中 Provider、Behavior、Memory 或 Hook 贡献声明的 `entry` / `handler` 前缀；没有声明贡献的纯 Worker 扩展允许调用任意安全方法名。
- `runtime.memory_limit_mb` 映射为 Wasm store 内存上限。
- `runtime.timeout_ms` 映射为 Wasm fuel 预算，防止无限循环长期占用 Host。
- `permissions` 当前作为能力声明和后续 host capability bridge 的唯一来源；在 host capability bridge 接入前，Worker 没有文件、网络、环境变量或数据库的宿主访问能力。

## API

- `GET /api/extensions`
- `GET /api/extensions/runtime`
- `GET /api/extensions/events`
- `GET /api/extensions/events/stream`
- `GET /api/extensions/{extension_id}`
- `GET /api/extensions/{extension_id}/diagnostics`
- `GET /api/extensions/{extension_id}/ui/module`
- `POST /api/extensions/{extension_id}/rpc/{method}`
- `PUT /api/extensions/{extension_id}/enabled`
- `POST /api/extensions/{extension_id}/reload`
- `POST /api/extensions/{extension_id}/restart`
- `POST /api/extensions/attach`
- `DELETE /api/extensions/attach/{extension_id}`
