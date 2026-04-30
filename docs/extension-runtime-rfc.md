# Extension Runtime RFC

本文档记录当前 Extension Runtime 的已落地约定。历史设计中的 `global/extensions`、`packages/extensions`、旧附加来源注册表、Skill/Extension 混合模型、端口型扩展后端已经废弃。

## 当前目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 安装扩展：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 扩展私有数据：`<ENNOIA_HOME>/data/extensions/<extension_id>/`

## 当前协议

Extension 使用 `extension.toml` 描述系统能力。Skill 使用 `skill.toml` 描述工具与用法，两者互不兼容、互不混用。

Extension descriptor 包含：

- `description`
- `docs`
- `conversation`
- `source`
- `ui`
- `worker`
- `permissions`
- `runtime`
- `build`
- `assets`
- `watch`
- `resource_types`
- `capabilities`
- `surfaces`
- `locales`
- `themes`
- `commands`
- `subscriptions`

Skill descriptor 包含：

- `description`
- `docs`
- `keywords`
- `entry`

主声明模型统一只有一层：`resource_types`、`capabilities`、`surfaces`、`locales`、`themes`、`commands`、`subscriptions`。页面、面板、Provider、Behavior、Memory、Action、Hook 和 Schedule Action 都是宿主运行时根据声明派生的视图。

`ui` 是可选界面入口；`worker` 是可选执行单元，可为 Wasm，也可为进程型 stdio RPC。宿主按声明装配能力，不要求扩展同时包含 UI 和 Worker。

Skill 不声明系统能力入口。它只提供最小目录元信息和 `docs` 入口；CLI、参数和完整用法都保留在文档中。

Extension 默认不进入会话目录。只有显式声明了 `conversation.inject = true` 时，宿主才会把它作为会话可见目录项暴露给模型；`conversation.resource_types` 和 `conversation.capabilities` 用于限定进入会话时附带的资源范围和能力入口。进入会话时复用扩展唯一那份 `description`，`docs` 仍然只作为按需查阅入口。

## 运行流程

1. CLI 初始化运行目录和默认配置。
2. CLI 同步内置扩展到 `<ENNOIA_HOME>/extensions/*`，并写入 `config/extensions.toml`。
3. 开发模式下 CLI 把仓库内 `builtins/extensions/*` 追加为开发来源。
4. Extension Host 扫描 `config/extensions.toml` 中启用且未移除的扩展来源。
5. Extension Host 解析 `ui`、`worker`、权限和贡献清单，生成 runtime snapshot。
6. Server 暴露 runtime snapshot、事件、诊断、日志、资源贡献接口、动作规则视图、scheduler API 和 Worker RPC。
7. Web 工作台通过 runtime snapshot 动态挂载扩展贡献。

## Action 与 Schedule Action

`capabilities[].metadata.action` 用于把扩展能力挂到系统稳定动作键上。典型 key 包括 `conversation.list`、`conversation.create`、`message.append`、`run.create`、`task.list`。

`capabilities[].metadata.schedule_action` 用于声明可被系统 scheduler 调用的动作。Scheduler 只保存计划和触发到期动作，不解释业务语义；业务参数通过 `params` 原样传入 Worker。

```toml
[[capabilities]]
id = "run.create"
contract = "run.create"
kind = "action"
entry = "workflow/runs/create"
metadata = { action = { key = "run.create", phase = "execute", priority = 100, result_mode = "last" } }

[[capabilities]]
id = "workflow.run"
contract = "workflow.run"
kind = "action"
entry = "workflow/schedules/run"
metadata = { schedule_action = { id = "workflow.run" } }
```

扩展源码推荐目录为 `ui/`、`worker/`、`data/` 和 `provider-presets/`。这些目录不是必备项，扩展只声明实际提供的能力。

## 开发热加载

- CLI 开发模式监听 `crates/`、`assets/`、`Cargo.toml` 和 `Cargo.lock`；扩展 UI 由独立 watcher 构建，不再因为 UI 资源变化而重编译 API 二进制。
- `node scripts/build-extension-ui.mjs --watch` 会把 `builtins/extensions/*/ui/entry.*` 构建到各自的 `ui/dist/entry.js`。
- Server 运行时按 2 秒轮询刷新扩展注册表和 manifest；UI bundle 文件版本变化会更新 runtime snapshot。
- Worker runtime 会缓存编译后的 Wasm Module，并在 `.wasm` mtime 或文件大小变化时自动重新编译。
- Process Worker 会按扩展维度常驻并在异常退出后自动重启。
- 每次 Wasm RPC 调用创建新的 Wasm 实例，避免跨请求共享线性内存状态。

## Worker ABI

当前宿主支持 `ennoia.worker`。Wasm Worker 必须导出：

- `memory`：线性内存。
- `ennoia_worker_alloc(len: i32) -> i32`：分配输入/输出缓冲区。
- `ennoia_worker_dealloc(ptr: i32, len: i32)`：释放缓冲区；无 GC 语言可以实现为空操作。
- `ennoia_worker_handle(ptr: i32, len: i32) -> i64`：处理一次 RPC 调用。

宿主写入 `ennoia_worker_handle` 的输入是 UTF-8 JSON：

```json
{
  "method": "memory/recall",
  "params": {},
  "context": {
    "trace": {
      "request_id": "req_xxx",
      "trace_id": "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx",
      "span_id": "xxxxxxxxxxxxxxxx",
      "parent_span_id": "xxxxxxxxxxxxxxxx",
      "sampled": true,
      "source": "action_rpc",
      "traceparent": "00-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx-xxxxxxxxxxxxxxxx-01"
    }
  }
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

宿主当前会在跨边界调用上写入 trace 上下文。Process Worker 和 Wasm Worker 都只消费 `context.trace` 这组普通 JSON 字段；链路追踪落库、查询和采样由宿主负责。

内置 `conversation` 与 `memory` 当前都采用 `jsonrpc-stdio` process Worker；内置 `workflow` 仍提供 `ennoia.worker` Wasm Worker。执行 `bun run build:workers` 会构建两个 release 进程 Worker 和一个 release Wasm Worker，并复制到各自 manifest 声明的位置。

## 沙箱与权限

- 默认不注入 WASI，也不允许任意 host import；声明了 import 的模块会被拒绝实例化。
- RPC 方法必须匹配 manifest 中 Provider、Behavior、Memory、Hook、Action 或 Schedule Action 贡献声明的 `entry` / `handler` / `method` 前缀；没有声明贡献的纯 Worker 扩展允许调用任意安全方法名。
- `runtime.memory_limit_mb` 映射为 Wasm store 内存上限。
- `runtime.timeout_ms` 映射为 Wasm fuel 预算，防止无限循环长期占用 Host。
- `permissions` 当前作为能力声明和后续 host capability bridge 的唯一来源；在 host capability bridge 接入前，Worker 没有文件、网络、环境变量或数据库的宿主访问能力。

## API

- `GET /api/extensions`
- `GET /api/extensions/runtime`
- `GET /api/extensions/events`
- `GET /api/extensions/events/stream`
- `GET /api/extensions/actions`
- `GET /api/extensions/schedule-actions`
- `GET /api/actions`
- `GET /api/schedule-actions`
- `GET /api/schedules`
- `POST /api/schedules`
- `GET /api/extensions/{extension_id}`
- `GET /api/extensions/{extension_id}/diagnostics`
- `GET /api/extensions/{extension_id}/ui/module`
- `GET /api/extensions/{extension_id}/ui/assets/{*asset_path}`
- `POST /api/extensions/{extension_id}/rpc/{method}`
- `PUT /api/extensions/{extension_id}/enabled`
- `POST /api/extensions/{extension_id}/reload`
- `POST /api/extensions/{extension_id}/restart`
- `POST /api/extensions/attach`
- `DELETE /api/extensions/attach/{extension_id}`
