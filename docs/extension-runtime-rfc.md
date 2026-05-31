# Extension Runtime RFC

本文档记录当前 Extension Runtime 的已落地约定。历史设计中的 `global/extensions`、`packages/extensions`、旧附加来源注册表、Skill/Extension 混合模型、端口型扩展后端已经废弃。

## 当前目录

- 扩展注册表：`<ENNOIA_HOME>/config/extensions.toml`
- 技能注册表：`<ENNOIA_HOME>/config/skills.toml`
- 安装扩展：`<ENNOIA_HOME>/extensions/<extension_id>/`
- 安装技能：`<ENNOIA_HOME>/skills/<skill_id>/`
- 扩展级宿主配置：`<ENNOIA_HOME>/config/extensions/<extension_id>.toml`
- 技能级宿主配置：`<ENNOIA_HOME>/config/skills/<skill_id>.toml`
- 扩展私有数据：`<ENNOIA_HOME>/data/extensions/<extension_id>/`
- 技能私有数据：`<ENNOIA_HOME>/data/skills/<skill_id>/`

## 当前协议

Extension 使用 `extension.toml` 描述系统能力。Skill 使用传统 `SKILL.md` 描述工具与用法，并使用同目录 `config.toml` 保存 Ennoia 增强配置；Extension 与 Skill 两者互不兼容、互不混用。

Extension descriptor 包含：

- `id`
- `version`
- `name`
- `description`
- `docs`
- `compat`
- `views`
- `operations`
- `events`
- `pipeline_handlers`
- `settings`
- `message_renderers`
- `conversation`

Skill 磁盘包包含：

- `SKILL.md`：传统技能入口，YAML frontmatter 必须包含 `name` 与 `description`
- `config.toml`：Ennoia 增强配置，包含 `version`、`mount.mode`、`actions[]`、`settings[]`、`diagnostics` 与可选 `prepare`

主声明模型统一只有一层：`views`、`operations`、`events`、`pipeline_handlers`、`settings`、`message_renderers`、`conversation`。

`views` 表达主壳可以打开或挂载的界面契约，当前稳定类型是 `page` 与 `panel`。`operations` 表达系统可调用动作；`operation.name` 是唯一调用名，同时作为 action key、Worker method 和事件投递目标。`events` 只表达 `on -> operation` 的异步投递关系。`pipeline_handlers` 表达生命周期入口的 handler、slot、优先级和 activation。`message_renderers` 表达消息正文格式到扩展 UI mount 的映射，主壳只做选择、挂载和纯文本兜底。

消息正文是单条消息的主内容，`format` 只是它的呈现约定。扩展不应把同一条普通回复拆成“正文消息 + 额外 HTML 记录”来实现排版；正确做法是让消息先落成 `format = "html"` 或 `format = "markdown"` 等主格式，再由对应 `message_renderers` 完成展示。

UI 与 service 入口由目录约定发现，属于宿主内部解析结果，不属于 manifest 契约，也不在扩展设计界面展示。

Skill 不声明系统能力入口。它只声明动作入口；CLI 参数、调用示例、平台限制和常见输入输出统一保留在 skill 目录下的 `SKILL.md` 中。

Extension 默认不进入会话目录。只有显式声明了 `conversation.visible = true` 时，宿主才会把它作为会话可见目录项暴露给模型；`conversation.resources` 和 `conversation.operations` 用于限定进入会话时附带的资源范围和操作入口。进入会话时复用扩展唯一那份 `description`，`docs` 仍然只作为按需查阅入口。

## 运行流程

1. CLI 初始化运行目录和默认配置。
2. 运行态 `init/start/serve` 同步未被 `blocked_builtin_sync` 屏蔽的内置扩展和技能到 `<ENNOIA_HOME>/extensions/*` 与 `<ENNOIA_HOME>/skills/*`，并写入 `config/extensions.toml` 与 `config/skills.toml`。
3. 开发态 `dev` 只初始化配置、日志、pid 和数据目录，把仓库内 `assets/extensions/*` 与 `assets/skills/*` 追加到对应注册表的 `dev_sources`，不复制内置扩展/技能包，并清理旧版本留下的内置包副本目录。
4. Extension Host 扫描 `<ENNOIA_HOME>/extensions/*` 中已安装扩展，并在开发态叠加 `config/extensions.toml` 的 `dev_sources`；Server 加载技能时在开发态优先使用 `config/skills.toml` 中启用的 `dev_sources`。
5. Extension Host 解析 manifest 契约，并按目录约定发现 UI / service 入口，生成 runtime snapshot。
6. Server 暴露 runtime snapshot、事件、诊断、日志、资源贡献接口、动作规则视图、技能资源接口、scheduler API 和 Worker RPC。
7. Web 工作台通过 runtime snapshot 动态挂载扩展贡献。
8. 会话消息正文按 runtime snapshot 中的 `message_renderers` 选择扩展 UI 渲染；没有可用渲染器时展示纯文本兜底。

## Operation 与事件

`operations[]` 用于把扩展操作挂到系统稳定动作键上。典型 name 包括 `conversation.list`、`conversation.create`、`message.append`、`run.create`、`task.list`。

设置 `schedule = true` 的 operation 可被系统 scheduler 调用。Scheduler 只保存计划和触发到期动作，不解释业务语义；业务参数通过 `params` 原样传入 Worker。

```toml
[[operations]]
name = "run.create"
description = "创建 workflow run。"
agent = true

[[operations]]
name = "workflow.run"
description = "由 scheduler 触发 workflow run。"
agent = true
schedule = true

[[pipeline_handlers]]
id = "workflow.task_response"
on = "conversation.operator_message.received"
stage = "drive"
slot = "conversation.response"
priority = 100
operation = "workflow.handle_operator_message"

[pipeline_handlers.activation]
scope = "conversation"
key = "workflow.task_mode"
default = false
label = { key = "ext.workflow.response_strategy", fallback = "处理策略" }
```

Pipeline handler 用于生命周期入口。当前已落地的主入口是 `conversation.operator_message.received` 的 `drive` 阶段和 `conversation.response` slot。宿主在 `message.append` 成功后后台驱动该 slot；消息发送接口只等待写入、事实事件和实时广播完成，不等待策略选择、模型回复、工具执行或 workflow run 推进。候选 handler 在后台任务内按 priority 降序调用；handler 返回 `claim` 或 `complete` 后，本次 slot 接管结束，返回 `skip`、`continue` 或 `fail` 时继续尝试后续 handler。

扩展源码推荐目录为 `ui/`、`bin/`、`worker/`、`data/` 和 `model-endpoint-presets/`。这些目录不是必备项，也不是 manifest 契约。

## 开发热加载

- CLI 开发模式监听 `crates/`、`assets/`、`Cargo.toml` 和 `Cargo.lock`，命中后重建并重启 API；内置扩展后端源码由独立 watcher 监听 `assets/extensions/*/(data|plugins|worker)/`，命中后重建并复制 builtin worker，不再把这类改动混进 API 热重载。
- 开发态的内置扩展和技能从 `assets/extensions/*` 与 `assets/skills/*` 直接加载；`ennoia dev` 初始化会清理旧版本留下的内置包副本，修改源码后不需要手动删除 `.dev/extensions/*` 或 `.dev/skills/*`。
- `node scripts/build-extension-ui.mjs --watch` 会把 `assets/extensions/*/ui/entry.*` 构建到各自的 `ui/dist/entry.js`。
- Server 运行时按 2 秒轮询刷新扩展注册表和 manifest；UI bundle 文件版本变化会更新 runtime snapshot。
- Worker runtime 会缓存编译后的 Wasm Module，并在 `.wasm` mtime 或文件大小变化时自动重新编译。
- Process Worker 会按扩展维度常驻，并在异常退出或目标二进制时间戳/大小变化后自动换新实例。
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

内置 `conversation`、`memory` 与 `workflow` 当前都采用 `jsonrpc-stdio` process Worker；`workflow` 仍保留 `ennoia.worker` Wasm Worker 构建产物，供独立 Wasm 场景复用。执行 `bun run build:workers` 会构建三个 release 进程 Worker 和一个 release Wasm Worker，并复制到各自目标位置。

## 沙箱与权限

- 默认不注入 WASI，也不允许任意 host import；声明了 import 的模块会被拒绝实例化。
- RPC 方法必须匹配 manifest 中声明的 `operation.name`。
- Wasm 内存和超时预算使用宿主运行时默认值。
- Agent 权限系统由宿主按 operation 和调用上下文统一裁决；扩展 manifest 不声明底层权限边界、SQLite、文件、网络或环境变量。

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
