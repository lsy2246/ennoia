# Extension 开发约定

## 定位

Extension 负责系统能力，Skill 负责 Agent 工具与用法。Extension manifest 只描述系统可见契约，不描述扩展内部实现细节。

系统关心的扩展契约只有五类：

- `views`：主壳可以打开或挂载的界面。
- `operations`：系统可以调用的操作。
- `events`：系统事件投递到 operation 的关系。
- `settings`：宿主需要渲染和保存的扩展级配置字段。
- `conversation`：扩展是否进入会话上下文，以及暴露哪些资源和 operation 名称。

UI 入口、service 入口、SQLite、缓存、内部权限边界、构建产物和运行脚本都属于扩展内部实现。宿主可以按目录约定发现入口，但这些内容不进入 manifest 契约，也不在扩展设计界面展示。

## 源码放置

- 官方内置扩展源码放在 `assets/extensions/<extension_id>/`。
- 官方内置技能源码放在 `assets/skills/<skill_id>/`。
- 运行目录里的真实包内容分别落在 `~/.ennoia/extensions/<id>/` 与 `~/.ennoia/skills/<id>/`。
- 是否启用、是否阻止内置同步统一登记在 `~/.ennoia/config/extensions.toml` 与 `~/.ennoia/config/skills.toml`。
- 开发态把 `assets/extensions/*` 与 `assets/skills/*` 注册到 `.dev/config/extensions.toml` 与 `.dev/config/skills.toml` 的 `dev_sources`，不把内置包复制到 `.dev/extensions/*` 或 `.dev/skills/*`，并会清理旧版本留下的内置包副本目录。

内置浏览器能力由 `web-search` 一个技能承载：

- `web-search` 是默认网页搜索技能，`browser_control = "local"` 时使用 CloakBrowser 自动化浏览器；本地浏览器内核来源在内置 Chromium、系统自动查找、手动路径之间三选一。
- `browser_control = "mcp"` 时，`web-search` 直接连接本技能配置的 MCP 浏览器服务；技能配置保存 `mcp_transport` 与 `mcp_url`，不保存浏览器工具名。

推荐扩展目录：

```text
<extension_id>/
├─ extension.toml
├─ docs/                 # 可选：扩展说明
├─ ui/                   # 可选：页面、面板、会话卡片、主题、语言
├─ bin/                  # 可选：process service
├─ worker/               # 可选：Wasm worker
├─ data/                 # 可选：schema、私有模型、资源
└─ model-endpoint-presets/ # 可选：初始化模型接入实例
```

推荐技能目录：

```text
<skill_id>/
├─ SKILL.md              # 必需：传统 Skill 入口，frontmatter 包含 name / description
├─ config.toml           # 必需：Ennoia 增强配置
├─ scripts/              # 可选：action、diagnostics、prepare 等脚本入口
├─ references/           # 可选：较长参考资料
└─ assets/               # 可选：技能私有静态资源
```

Skill 的产品化配置只放在 `config.toml`。`SKILL.md` 保持传统技能语义，描述使用场景、调用方式、输入输出和注意事项；宿主在运行时把两者合成为内部 `SkillManifest`。

## Manifest

系统扩展只使用 `extension.toml`。顶层字段固定为：

- `id`
- `version`
- `name`
- `description`
- `docs`
- `compat`
- `views`
- `operations`
- `events`
- `settings`
- `conversation`

示例：

```toml
id = "workflow"
version = "0.1.0"
name = "Workflow"
description = "提供 run、task、artifact 与调度动作的编排实现。"
docs = "docs/overview.md"

[compat]
ennoia = ">=0.1.0"

[conversation]
visible = true
resources = ["workflow.run", "workflow.task", "workflow.artifact"]
operations = ["run.create", "run.get", "run.list", "task.list", "artifact.list"]

[[views]]
name = "workflow.page"
type = "page"
title = { key = "ext.workflow.page", fallback = "工作编排" }
nav = "sidebar"
order = 35
icon = "workflow"
route = "/workflow"

[[operations]]
name = "run.create"
description = "创建 workflow run。"
agent = true

[[operations]]
name = "workflow.run"
title = { key = "ext.workflow.schedule.run", fallback = "Run workflow" }
description = "由 scheduler 触发 workflow run。"
agent = true
schedule = true

[[operations]]
name = "workflow.conversation.message.created"
description = "处理会话消息创建事件。"

[[events]]
on = "conversation.message.created"
operation = "workflow.conversation.message.created"
priority = 100
```

## Views

`views[]` 声明主壳可以打开或挂载的界面。当前稳定类型是页面与面板。

字段：

- `name`：视图唯一名，也是 UI module 中的 mount key。
- `type`：视图类型，例如 `page`、`panel`。
- `title`：本地化标题。
- `nav`：可选导航位置；`sidebar` 表示可进入主导航。
- `order`：可选排序。
- `slot`：面板槽位，例如 `right`。
- `icon`：可选图标名。
- `route`：页面路由。
- `priority`：同类挂载优先级。

Web 主壳按 runtime snapshot 挂载 view，不需要在 manifest 里重复描述 UI 或 service 入口。

## Operations

`operations[]` 声明系统可调用的操作。`operation.name` 是唯一调用名，同时作为：

- `/api/actions/{action}` 的 action key。
- Worker RPC method。
- `events[].operation` 的投递目标。
- Agent 权限裁决里的 action 名。

字段：

- `name`
- `title`
- `description`
- `agent`
- `input`
- `output`
- `provider`
- `schedule`

扩展不在 manifest 中声明权限目标、SQLite、网络、文件或环境变量。Agent 权限由宿主按 operation 名称、调用参数和 `permission_actor` 上下文统一裁决。

## Events

`events[]` 只声明系统事件投递关系：

- `on`：系统事件名。
- `operation`：事件发生后调用的 operation 名称。
- `priority`：同一事件下的投递优先级，默认 `0`。宿主按 `priority` 降序投递，同优先级按扩展 ID 和 handler 名稳定排序。

系统先把事件写入宿主持久化事件总线，再异步投递到目标 operation。扩展临时离线不会阻塞主业务写入。

内置 `workflow` 通过事件钩子进入多个编排时机，而不是接管唯一会话入口：

```toml
[[events]]
on = "conversation.message.created"
operation = "workflow.conversation.message.created"
priority = 100

[[events]]
on = "operation.updated"
operation = "workflow.operation.updated"
priority = 100

[[events]]
on = "permission.approval.resolved"
operation = "workflow.permission.approval.resolved"
priority = 100

[[events]]
on = "run.requested"
operation = "workflow.run.requested"
priority = 100

[[events]]
on = "run.stage.changed"
operation = "workflow.run.stage.changed"
priority = 100

[[events]]
on = "artifact.created"
operation = "workflow.artifact.created"
priority = 100

[[events]]
on = "job.due"
operation = "workflow.job.due"
priority = 100
```

其中 `conversation.message.created`、`operation.updated`、`permission.approval.resolved`、`run.requested` 和 `job.due` 由宿主动作管道、权限路由或 scheduler 发布；`run.stage.changed` 和 `artifact.created` 由 workflow worker 在内部持久化对应事实后，通过宿主 `HookEventPublish` capability 写回事件总线。

## Settings

`settings[]` 声明扩展级配置表单。宿主渲染表单并保存到 `~/.ennoia/config/extensions/{extension_id}.toml`。

支持字段类型：

- `text`
- `textarea`
- `boolean`
- `select`
- `number`

扩展私有数据库、缓存和业务运行数据保留在 `~/.ennoia/data/extensions/{extension_id}/`，由扩展自行解释。

## Conversation

扩展默认不进入会话上下文。需要进入时声明：

```toml
[conversation]
visible = true
resources = ["memory.item"]
operations = ["memory.query", "memory.build_context"]
```

进入会话时，宿主只暴露扩展的 `description`、`docs` 入口、`conversation.resources` 和 `conversation.operations`。宿主不自动注入 `docs` 正文，也不展示扩展内部实现细节。

## UI Module ABI

扩展 UI 源码入口推荐放在 `ui/entry.tsx`，构建产物推荐输出到 `ui/dist/entry.js`。UI bundle 导出 `ExtensionUiModule`：

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
  panels: {
    "memory.context": (container, context) => {
      return {
        unmount() {},
      };
    },
  },
};

export default ui;
```

页面和面板导出 `mount(container, context)` / `unmount()`；扩展 UI 可以自带自己的 React runtime，不和主壳 hooks 冲突。需要渲染会话时间线记录时导出 `conversationRecords`，它使用扩展记录的 `kind` 作为 mount key。`context.helpers` 提供 `apiBaseUrl`、`locale`、`themeId`、`t()`、`formatDateTime()` 等宿主运行时能力。

内置 `html-reply` 与 `artifact-runner` 扩展使用这一机制承载两类输出体验：`workflow` 识别 `ennoia.html_reply` envelope 后只把普通 fallback 写入会话消息，把 HTML 排版内容写入 `html-reply.message`；识别 `ennoia.artifact_runner` envelope 后把 HTML/Python 产物写入 `artifact-runner.artifact`。这些协议属于扩展内部约定，不新增系统级 manifest 字段，也不改变核心消息模型。

## Worker ABI

Process service 推荐放在 `bin/`。Wasm worker 推荐放在 `worker/`，当前宿主支持 `ennoia.worker` ABI：

- `memory`
- `ennoia_worker_alloc(len: i32) -> i32`
- `ennoia_worker_dealloc(ptr: i32, len: i32)`
- `ennoia_worker_handle(ptr: i32, len: i32) -> i64`

宿主传入的缓冲区是 UTF-8 JSON：

```json
{
  "method": "memory.query",
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

返回缓冲区应是 `ExtensionRpcResponse` JSON；如果返回普通 JSON，宿主会包装为成功响应。RPC 方法必须匹配 manifest 中声明的 `operation.name`。

## 运行链路

1. CLI 初始化运行目录。
2. 运行态 `init/start/serve` 同步未被 `blocked_builtin_sync` 屏蔽的内置扩展和技能到 `<ENNOIA_HOME>/extensions/<extension_id>/` 与 `<ENNOIA_HOME>/skills/<skill_id>/`，并更新 `config/extensions.toml` 与 `config/skills.toml`。
3. 开发态 `dev` 只初始化配置、日志、pid 和数据目录，把仓库内 `assets/extensions/*` 与 `assets/skills/*` 追加到对应注册表的 `dev_sources`，不复制内置扩展/技能包，并清理旧版本留下的内置包副本目录。
4. Extension Host 扫描扩展并解析 manifest 契约；Server 加载技能时在开发态优先使用启用的 `dev_sources`。
5. Extension Host 按目录约定发现 UI / service 入口，生成 runtime snapshot。
6. Server 暴露 runtime snapshot、事件流、诊断、日志、scheduler API、action API、技能资源 API 和 Worker RPC。
7. Web 工作台根据 runtime snapshot 动态导入扩展 UI 模块，并按 view name 挂载页面和面板；会话时间线记录按扩展 record kind 挂载。

## 沙箱与权限

- Host 默认不注入 WASI，也不允许任意 import。
- Wasm 内存和超时预算使用宿主运行时默认值。
- Agent 权限由宿主按 operation 和上下文裁决。
- 扩展 manifest 不声明底层权限、SQLite、网络、文件或环境变量。
