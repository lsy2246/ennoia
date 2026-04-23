# Ennoia 扩展开发指南

## 定位

Extension 是系统能力包，Skill 是 Agent 可引用的能力包。Extension 不再表示“前端 + 独立后端服务”，而是由宿主装配的一组可选贡献：`ui`、`worker`、主题、语言、命令、Provider、Behavior、Memory 和 Hook。

## 源码放置

- 官方内置扩展源码放在 `builtins/extensions/<extension_id>/`
- 官方内置技能源码放在 `builtins/skills/<skill_id>/`
- 运行目录里的真实包内容分别落在 `~/.ennoia/extensions/<id>/` 与 `~/.ennoia/skills/<id>/`
- 是否启用、是否卸载、来源路径统一登记在 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## Manifest

系统扩展只使用 `extension.toml`。推荐字段：

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

`ui` 和 `worker` 都是可选声明。纯 UI 扩展不需要 `worker`，纯能力扩展不需要 `ui`。

```toml
[ui]
runtime = "browser-esm"
entry = "ui/entry.js"

[worker]
kind = "wasm"
entry = "worker/plugin.wasm"
abi = "ennoia.worker.v1"

[permissions]
storage = "extension"
sqlite = true
network = []
events = ["publish", "subscribe"]
fs = []
env = []

[runtime]
startup = "lazy"
timeout_ms = 30000
memory_limit_mb = 128
```

`contributes` 可包含：

- `pages[]`
- `panels[]`
- `themes[]`
- `locales[]`
- `commands[]`
- `providers[]`
- `behaviors[]`
- `memories[]`
- `hooks[]`

`pages[]` 是可选 UI 贡献。声明页面后，Web 的扩展详情页会提供“打开视图”；只有页面额外声明 `nav.default_pinned = true` 时才默认进入主导航。

Hook 贡献声明扩展要接收的系统时机：

```toml
[contributes]
hooks = [
  { event = "conversation.message.created", handler = "hooks/conversation-message-created" }
]
```

系统把 Hook 事件转换为 Worker RPC 调用。扩展返回 `HookDispatchResponse`：

- `handled=true`：扩展已处理该事件。
- `result`：可选结构化结果，供调用方继续返回或落库。
- `message`：可选诊断说明。

Provider、Behavior 和 Memory 贡献只声明能力入口；实际执行统一通过宿主 Worker RPC 分发，不允许扩展自行开放端口。

## 推荐目录

```text
<extension_id>/
├─ extension.toml
├─ ui/               # 可选：页面、面板、主题、语言
├─ worker/           # 可选：Wasm Worker
├─ data/             # 可选：schema、私有模型、资源
└─ provider-presets/ # 可选：初始化上游渠道实例
```

Skill 目录独立：

```text
<skill_id>/
├─ skill.toml
├─ entry.js
└─ schemas/
```

## 运行链路

1. `cargo run -p ennoia-cli -- dev` 初始化运行目录。
2. CLI 把内置扩展同步到 `<ENNOIA_HOME>/extensions/<extension_id>/`，并更新 `config/extensions.toml`。
3. CLI 扫描内置扩展中的 `provider-presets/*.toml`，把默认渠道实例写入 `config/providers/`。
4. CLI 把仓库内 `builtins/extensions/*` 追加为开发来源，供开发模式覆盖安装目录。
5. Extension Host 扫描扩展包，解析 `ui`、`worker` 和贡献能力，不启动扩展私有进程。
6. Server 暴露 runtime snapshot、事件流、诊断、日志、资源贡献接口，以及 `/api/extensions/{extension_id}/rpc/{method}` Worker RPC 入口。
7. Core 只在自身生命周期时机派发 Hook；扩展内部按 Worker ABI 和 capability 组织自己的业务逻辑。
8. Web 工作台根据 runtime snapshot 挂载页面、面板、主题、语言和命令；如果某个 mount 在本地 registry 中存在实现，则直接渲染真实组件。

## 开发热加载

- `ennoia dev` 监听 `crates/`、`assets/`、`builtins/extensions/`、`Cargo.toml` 和 `Cargo.lock`。
- `builtins/extensions/` 下的 `extension.toml`、UI 资源和 `.wasm` 变更会触发 Host 重新构建并重启 API 进程。
- Server 运行时每 2 秒刷新一次扩展注册表与 manifest，用于让扩展启停、贡献声明和入口路径变化尽快反映到 runtime snapshot。
- Worker runtime 会缓存编译后的 Wasm Module；`.wasm` mtime 或文件大小变化后，下一次 RPC 调用会自动重新编译。
- 每次 RPC 调用都会创建新的 Wasm 实例，避免线性内存状态跨请求泄漏。

## Worker ABI

当前宿主支持 `ennoia.worker.v1`，Worker 需要导出：

- `memory`
- `ennoia_worker_alloc(len: i32) -> i32`
- `ennoia_worker_dealloc(ptr: i32, len: i32)`
- `ennoia_worker_handle(ptr: i32, len: i32) -> i64`

宿主传入的缓冲区是 UTF-8 JSON：

```json
{
  "method": "memory/recall",
  "params": {},
  "context": {}
}
```

`ennoia_worker_handle` 返回高 32 位为 `ptr`、低 32 位为 `len` 的打包值。返回缓冲区应是 `ExtensionRpcResponse` JSON；如果返回普通 JSON，宿主会包装为成功响应。

内置 `memory` 与 `workflow` Worker 分别位于：

- `builtins/extensions/memory/worker`
- `builtins/extensions/workflow/worker`

执行 `bun run build:workers` 会编译 `wasm32-unknown-unknown` release 产物，并复制到：

- `builtins/extensions/memory/worker/memory.wasm`
- `builtins/extensions/workflow/worker/workflow.wasm`

## 沙箱与权限

- Host 默认不注入 WASI，也不允许任意 import。
- RPC 方法必须匹配 manifest 中 Provider、Behavior、Memory 或 Hook 贡献声明的 `entry` / `handler` 前缀；纯 Worker 扩展没有贡献前缀时允许调用安全方法名。
- `runtime.memory_limit_mb` 控制 Wasm store 内存上限。
- `runtime.timeout_ms` 控制 Wasm fuel 预算，防止无限循环长期占用 Host。
- `permissions` 是后续 host capability bridge 的唯一声明来源；在 bridge 接入前，Worker 没有文件、网络、环境变量或数据库的宿主访问能力。

## 安装与扫描目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 技能注册表：`<ENNOIA_HOME>/config/skills.toml`
- 扩展包目录：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 技能包目录：`<ENNOIA_HOME>/skills/<skill_id>/`
- 扩展私有数据目录：`<ENNOIA_HOME>/data/extensions/<extension_id>/`

扩展自己的数据库、缓存和私有运行态文件都应放在扩展私有数据目录。核心不提供主业务 SQLite；扩展通过 Worker capability 使用宿主授予的存储、SQLite、网络、事件和日志能力。
