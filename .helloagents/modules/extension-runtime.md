# extension-runtime

## 职责

- 维护扩展能力包协议、运行时扫描、快照、诊断和 Worker RPC 分发
- 统一解析可选 `ui` 与可选 `worker` 声明
- 禁止扩展自行开放私有端口或通过宿主启动长期子进程
- 为 Provider、Behavior、Memory、Hook、Interface 和 Schedule Action 贡献提供统一 Worker 调用边界

## 行为规范

- 扩展 manifest 使用 `ui`、`worker`、`permissions`、`runtime`、`contributes` 表达能力
- `worker.kind` 当前目标为 `wasm`，ABI 默认 `ennoia.worker.v1`
- Server 使用 `/api/extensions/{extension_id}/rpc/{method}` 作为统一 RPC 入口
- Behavior 与 Memory 路由先解析 active provider，再转换为 Worker RPC 调用
- 稳定 `/api/conversations`、`/api/runs`、`/api/schedules` 等入口不直接内置业务存储；先解析细粒度接口绑定或 schedule action，再调用扩展 Worker
- `config/interfaces.toml` 保存显式接口绑定；没有显式绑定且只有一个实现时自动选中，多实现时返回冲突
- `data/system/schedules.json` 保存 scheduler 计划；到期 tick 触发对应扩展的 `schedule_actions.method`
- 内置扩展资源允许同时包含文本资源和 `.wasm` 二进制资源
- 开发模式监听 `builtins/extensions/` 与 `.wasm`，变更会触发 Host 重启
- Worker runtime 使用 `wasmtime` 装载 `ennoia.worker.v1`，缓存 Module，每次 RPC 新建实例
- `.wasm` mtime 或大小变化会让 Module 缓存失效，下一次 RPC 自动重新编译
- Host 默认不注入 WASI/import，按贡献 `entry` / `handler` / `method` 前缀校验 RPC 方法，并使用 `runtime.memory_limit_mb` 与 `runtime.timeout_ms` 约束执行
- 内置 `memory` 与 `workflow` 都有 `worker/` crate；`bun run build:workers` 编译并复制 `memory.wasm` / `workflow.wasm`

## 依赖关系

- 依赖 `crates/kernel` 的 manifest、权限、Worker 与 RPC DTO
- 依赖 `crates/extension-host` 的 registry 与 Worker 分发边界
- 依赖 `crates/server` 的接口绑定、能力路由、scheduler 和 HTTP API
- 依赖 `crates/assets` 与 `crates/cli` 同步内置扩展资源
