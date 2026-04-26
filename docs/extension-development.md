# Ennoia 扩展开发指南

## 定位

Extension 负责系统能力，Skill 负责工具与用法。Extension 不再表示“前端 + 独立后端服务”，而是由宿主装配的一组能力声明：`ui`、`worker`、`resource_types`、`capabilities`、`surfaces`、`themes`、`locales`、`commands`、`subscriptions`。

## 源码放置

- 官方内置扩展源码放在 `builtins/extensions/<extension_id>/`
- 官方内置技能源码放在 `builtins/skills/<skill_id>/`
- 运行目录里的真实包内容分别落在 `~/.ennoia/extensions/<id>/` 与 `~/.ennoia/skills/<id>/`
- 是否启用、是否卸载、来源路径统一登记在 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`

## Manifest

系统扩展只使用 `extension.toml`。推荐字段：

- `description`
- `docs`
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

扩展可以带能力说明文档，但这类说明仍然属于扩展本身，不进入 skill 语义。推荐通过 `description`、`docs`、`links[]` 和 `examples[]` 表达“这个扩展提供什么能力、适合怎么被系统调用”。

`ui` 和 `worker` 都是可选声明。纯 UI 扩展不需要 `worker`，纯能力扩展不需要 `ui`。`worker.kind` 当前支持 `wasm` 和 `process`；`process` Worker 通过 stdin/stdout 的 JSON 文本协议接入宿主，不需要自行开放 HTTP 端口。需要本地 SQLite、文件和后台任务的扩展，推荐直接使用 `process` Worker。

```toml
[worker]
kind = "process"
entry = "bin/conversation-service"
protocol = "jsonrpc-stdio"

[ui]
runtime = "browser-esm"
hmr = true

[build]
ui_bundle = "ui/dist/entry.js"

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

如果使用 Wasm Worker，则把 `worker.kind` 改为 `wasm`，并额外声明 `abi = "ennoia.worker"` 与对应的 `.wasm` 入口文件。

Manifest 主声明只有一层：

- `resource_types[]`：声明扩展理解或产出的资源模型。
- `capabilities[]`：声明系统能力入口。
- `surfaces[]`：声明 page、panel 等 UI 挂载点。
- `locales[]`、`themes[]`、`commands[]`：声明静态 UI 资源。
- `subscriptions[]`：声明事件订阅关系。

运行时会从 `capabilities[].metadata` 和 `subscriptions[]` 派生旧的产品视图：

- `metadata.provider` -> Provider
- `metadata.behavior` -> Behavior
- `metadata.memory` -> Memory
- `metadata.interface` -> Interface
- `metadata.schedule_action` -> Schedule Action
- `subscriptions[] + capability.entry` -> Hook

`surfaces[]` 里的 `kind = "page"` 是可选 UI 页面贡献。声明页面后，Web 的扩展详情页会提供“打开视图”；只有页面额外声明 `nav.default_pinned = true` 时才默认进入主导航。

`themes[]` 是可选主题贡献。扩展主题遵循 `ennoia.theme`，通过 `tokens_entry` 提供 CSS 变量文件，详细 token 规范见 [主题协议](theme-contract.md)。

```toml
[[themes]]
id = "acme.sunrise"
contract = "ennoia.theme"
label = { key = "ext.acme.theme.sunrise", fallback = "Sunrise" }
appearance = "light"
tokens_entry = "ui/themes/sunrise.css"
preview_color = "#f59e0b"
extends = "system"
category = "extension"
```

Hook 不再直接作为 manifest 顶层声明，而是拆成“能力入口 + 订阅关系”：

```toml
[[capabilities]]
id = "acme.conversation_message"
contract = "hook.conversation_message"
kind = "event_handler"
entry = "hooks/conversation-message-created"

[[subscriptions]]
event = "conversation.message.created"
capability = "acme.conversation_message"
```

系统先把 Hook 事件写入宿主事件总线，再异步转换为 Worker RPC 调用。扩展返回 `HookDispatchResponse`：

- `handled=true`：扩展已处理该事件。
- `result`：可选结构化结果，供调用方继续返回或落库。
- `message`：可选诊断说明。

Interface 实现通过 capability metadata 声明：

```toml
[[capabilities]]
id = "conversation.list"
contract = "conversation.list"
kind = "query"
entry = "conversation/conversations/list"
metadata = { interface = { key = "conversation.list" } }

[[capabilities]]
id = "message.append_user"
contract = "message.append_user"
kind = "action"
entry = "conversation/messages/append-user"
metadata = { interface = { key = "message.append_user" } }
```

Schedule Action 也通过 capability metadata 声明：

```toml
[[capabilities]]
id = "workflow.run"
contract = "workflow.run"
kind = "action"
entry = "workflow/schedules/run"
metadata = { schedule_action = { id = "workflow.run" } }
```

Provider、Behavior、Memory、Hook、Interface 和 Schedule Action 都只声明能力入口；实际执行统一通过宿主 Worker RPC 分发或宿主事件总线投递，不允许扩展自行开放端口。
扩展自己的配置、UI 文案、主题、页面实现和业务运行态都属于扩展边界，不得放入 Web 主壳的系统模块、核心配置模型或 `config/` 根目录。

## 推荐目录

```text
<extension_id>/
├─ extension.toml
├─ ui/               # 可选：页面、面板、主题、语言
├─ bin/              # 可选：process Worker
├─ worker/           # 可选：Wasm Worker
├─ data/             # 可选：schema、私有模型、资源
└─ provider-presets/ # 可选：初始化上游渠道实例
```

Skill 目录独立：

```text
<skill_id>/
├─ skill.toml
├─ README.md
├─ entry.js
└─ schemas/
```

`skill.toml` 推荐额外声明：

- `docs`
- `requires[]`
- `examples[]`
- `tool`

其中 `requires[]` 只依赖能力契约，例如 `llm.generate`、`run.create`，不要依赖某个具体扩展 ID。

## 运行链路

1. `cargo run -p ennoia-cli -- dev` 初始化运行目录。
2. CLI 把内置扩展同步到 `<ENNOIA_HOME>/extensions/<extension_id>/`，并更新 `config/extensions.toml`。
3. CLI 扫描内置扩展中的 `provider-presets/*.toml`，把默认渠道实例写入 `config/providers/`。
4. CLI 把仓库内 `builtins/extensions/*` 追加为开发来源，供开发模式覆盖安装目录。
5. Extension Host 扫描扩展，解析 `ui`、`worker` 和贡献能力，不启动扩展私有进程。
6. Server 暴露 runtime snapshot、事件流、诊断、日志、资源贡献接口、接口绑定 API、scheduler API，以及 `/api/extensions/{extension_id}/rpc/{method}` Worker RPC 入口。
7. Core 只维护稳定接口、绑定、计划与 Hook 派发；扩展内部按 Worker ABI 和 capability 组织自己的业务逻辑。
8. Web 工作台根据 runtime snapshot 动态导入扩展 UI 模块，并按 mount id 挂载页面、面板、主题、语言和命令。

## UI Module ABI

- 扩展 UI 源码入口推荐放在 `ui/entry.tsx`
- 构建产物推荐输出到 `ui/dist/entry.js`
- UI bundle 必须导出一个 ESM 模块对象，按 mount id 暴露页面和面板挂载器：

```ts
import type { ExtensionUiModule } from "@ennoia/ui-sdk";

const ui: ExtensionUiModule = {
  pages: {
    "memory.page": (container, context) => {
      return {
        unmount() {},
      };
    },
  },
};

export default ui;
```

- 页面和面板不再向主壳导出 React 组件本身，而是导出 `mount(container, context)` / `unmount()`；这样扩展 UI 可以自带自己的 React runtime，不和主壳 hooks 冲突。
- `context.helpers` 会提供 `apiBaseUrl`、`locale`、`themeId`、`t()`、`formatDateTime()` 等宿主运行时能力。

## 开发热加载

- `ennoia dev` 监听 `crates/`、`assets/`、`builtins/extensions/`、`Cargo.toml` 和 `Cargo.lock`。
- `ennoia dev` 会额外启动扩展 UI watcher，自动把 `ui/entry.*` 构建到 `ui/dist/entry.js`。
- Server 运行时每 2 秒刷新一次扩展注册表与 manifest；UI bundle 版本变化会进入 runtime snapshot，并通过 `/api/extensions/events/stream` 触发 Web 重新加载当前扩展模块。
- Worker runtime 会缓存编译后的 Wasm Module；`.wasm` mtime 或文件大小变化后，下一次 RPC 调用会自动重新编译。
- 每次 RPC 调用都会创建新的 Wasm 实例，避免线性内存状态跨请求泄漏。

## Worker ABI

当前宿主支持 `ennoia.worker`，Worker 需要导出：

- `memory`
- `ennoia_worker_alloc(len: i32) -> i32`
- `ennoia_worker_dealloc(ptr: i32, len: i32)`
- `ennoia_worker_handle(ptr: i32, len: i32) -> i64`

宿主传入的缓冲区是 UTF-8 JSON：

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
      "source": "interface_rpc",
      "traceparent": "00-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx-xxxxxxxxxxxxxxxx-01"
    }
  }
}
```

`ennoia_worker_handle` 返回高 32 位为 `ptr`、低 32 位为 `len` 的打包值。返回缓冲区应是 `ExtensionRpcResponse` JSON；如果返回普通 JSON，宿主会包装为成功响应。

`context.trace` 表示当前跨边界调用的链路上下文。扩展不需要理解宿主内部数据库结构，但如果扩展内部还会继续拆子步骤、写自己的日志或继续调用其他能力，应该优先透传这组字段。

内置 `conversation` 与 `memory` 当前都采用 process Worker，`workflow` 仍采用 Wasm Worker。

执行 `bun run build:workers` 会：

- 构建 `conversation` 与 `memory` 的 release 进程 Worker，并复制到各自扩展目录下的 `bin/`
- 构建 `workflow` 的 `wasm32-unknown-unknown` release 产物，并复制到 `builtins/extensions/workflow/worker/workflow.wasm`

## 沙箱与权限

- Host 默认不注入 WASI，也不允许任意 import。
- RPC 方法必须匹配 manifest 中 Provider、Behavior、Memory、Hook、Interface 或 Schedule Action 贡献声明的 `entry` / `handler` / `method` 前缀；纯 Worker 扩展没有贡献前缀时允许调用安全方法名。
- `runtime.memory_limit_mb` 控制 Wasm store 内存上限。
- `runtime.timeout_ms` 控制 Wasm fuel 预算，防止无限循环长期占用 Host。
- `permissions` 是后续 host capability bridge 的唯一声明来源；在 bridge 接入前，Worker 没有文件、网络、环境变量或数据库的宿主访问能力。

## 安装与扫描目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 技能注册表：`<ENNOIA_HOME>/config/skills.toml`
- 扩展目录：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 技能目录：`<ENNOIA_HOME>/skills/<skill_id>/`
- 扩展私有数据目录：`<ENNOIA_HOME>/data/extensions/<extension_id>/`

扩展自己的数据库、缓存和私有运行态文件都应放在扩展私有数据目录。核心不提供主业务 SQLite；扩展通过 Worker capability 使用宿主授予的存储、SQLite、网络、事件和日志能力。
扩展私有业务配置也应放在扩展私有数据目录，并由扩展自行解释；核心只负责扩展生命周期、能力发现和 Worker RPC 分发。
