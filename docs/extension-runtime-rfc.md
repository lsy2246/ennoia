# Ennoia Workspace Extension Runtime RFC

## 1. 文档定位

本文档定义 Ennoia 扩展系统的目标态开发运行时方案。

它解决的问题不是“如何少打一次包”，而是把 **源码工作区扩展** 升级为平台的一等运行单元，使开发态与发布态共享同一套扩展协议、生命周期和运维模型。

本文档属于 **架构演进 RFC**，描述目标态设计，不代表当前仓库已经全部实现。

## 2. 背景与问题

当前扩展链路以“安装目录 + manifest + 启动时扫描”为核心：

1. 扩展安装到 `~/.ennoia/global/extensions/`
2. `config/extensions/*.toml` 声明启用状态与安装目录
3. Server 启动时扫描 manifest
4. Web 拉取 registry 后挂载页面、面板、主题和语言包

这套机制适合发布态，但不适合长期开发：

- 源码目录不是一等公民，开发时仍然在模拟安装产物
- 前端入口只有 bundle 概念，不适合直接对接 dev server / HMR
- 后端入口只有静态文件概念，不适合按扩展托管独立 runner
- 扩展注册表是启动时快照，而不是可热替换的运行时容器
- Web 主要基于主动拉取快照，缺少对扩展图谱变化的订阅能力

## 3. 设计目标

- 开发态与发布态共享同一份扩展描述协议
- 源码工作区扩展可直接被 Ennoia 发现、挂载、调试和诊断
- 扩展前端支持 URL / Module / File 三类解析结果
- 扩展后端支持独立 runner、热重启、健康检查与代次切换
- 扩展 registry 支持 generation 级原子替换
- Web 支持 snapshot + subscription 的动态挂载
- 单个扩展故障应局部降级，不影响主壳和其他扩展

## 4. 核心原则

### 4.1 开发态是正式模型

开发态不是发布态的降级模式，而是扩展系统的正式运行模式之一。

统一模型如下：

- `workspace extension`：源码工作区扩展
- `package extension`：可发布构建产物
- `runtime extension`：被当前实例解析并挂载的运行单元

### 4.2 协议描述来源，而不仅描述产物

扩展协议不能只描述 `frontend_bundle` / `backend_entry` 这样的产物入口，还要描述：

- 源码根目录
- 开发入口
- 运行时解析方式
- watcher 规则
- runner 约束
- 诊断与健康状态

### 4.3 扩展运行时必须支持原子切换

扩展图谱更新不能依赖整个 Server 重启。

每次扫描、编译或 runner 重新就绪后，系统应生成一个新的 `ExtensionGeneration`，并通过原子替换切换整个运行视图。

## 5. 名词定义

- `ExtensionDescriptor`：源码级扩展描述
- `ResolvedExtension`：解析后的运行时扩展实例
- `ExtensionGraph`：当前已启用扩展及其贡献、依赖、状态的全量视图
- `ExtensionGeneration`：一次成功装配后的 registry 代次
- `ExtensionHealth`：扩展健康状态，如 `discovering`、`building`、`ready`、`degraded`、`failed`
- `Workspace Extension Runtime`：负责发现、解析、watch、编排与热切换的扩展运行时

## 6. 目标态分层

```text
Web
  -> Server
    -> Extension Runtime
      -> Workspace Registry
      -> Watch Service
      -> Frontend Module Resolver
      -> Backend Runner Manager
      -> Extension Graph Store
    -> Kernel
    -> Memory
    -> Orchestrator
    -> Scheduler
```

说明：

- `Extension Runtime` 是独立子系统，不再只是一段启动时扫描逻辑
- `Workspace Registry` 负责聚合源码工作区扩展与已安装扩展
- `Watch Service` 负责感知 manifest、源码、资源和 runner 状态变化
- `Frontend Module Resolver` 负责把扩展前端入口解析成可挂载目标
- `Backend Runner Manager` 负责托管扩展后端进程与生命周期
- `Extension Graph Store` 负责保存当前 generation 的运行图谱

## 7. 扩展来源模型

目标态扩展来源至少分为两类：

- `installed source`：来自 `~/.ennoia` 的安装扩展
- `workspace source`：来自源码仓库、本地开发目录或显式 attach 的工作区扩展

运行时不应只扫描单一目录，而应聚合多个 provider：

- home 目录的安装声明
- 仓库内的 `extensions/*/ennoia.extension.toml`
- `~/.ennoia/dev/workspaces.toml` 中显式附加的开发工作区
- 后续可扩展的远程 registry / repo source provider

## 8. 扩展描述协议

目标态建议把扩展描述升级为“源码 + 开发 + 构建”三段式协议。

示意：

```toml
id = "observatory"
kind = "extension"
version = "0.1.0"

[source]
kind = "workspace"
root = "./"
manifest = "./ennoia.extension.toml"

[frontend]
runtime = "browser-esm"
entry = "./src/frontend/index.ts"

[backend]
runtime = "node"
entry = "./src/backend/index.ts"
host = "process"

[assets]
locales = "./src/locales"
themes = "./src/themes"

[dev]
frontend_dev_url = "http://127.0.0.1:4201/src/index.ts"
backend_dev_command = "bun --watch src/backend/index.ts"
watch = [
  "ennoia.extension.toml",
  "src/frontend/**",
  "src/backend/**",
  "src/locales/**",
  "src/themes/**"
]
hmr = true
self_load = true

[build]
frontend_bundle = "./dist/frontend/index.js"
backend_bundle = "./dist/backend/index.js"
manifest_out = "./dist/manifest.toml"
```

说明：

- `source` 描述源码来源
- `frontend` / `backend` 描述逻辑入口
- `dev` 描述开发态运行方法与 watch 行为
- `build` 描述发布产物输出

## 9. 运行时解析结果

扩展描述经解析后，生成 `ResolvedExtension`。

解析结果需要明确：

- 当前模式：`workspace` 或 `package`
- 当前 generation
- 当前 health
- 源码根目录与安装根目录
- 前端入口解析结果
- 后端入口解析结果
- 贡献清单
- watcher 状态
- runner 状态
- 最近诊断事件

前端入口建议支持：

- `file`：发布态产物文件
- `url`：开发态 dev server 地址
- `module`：由统一 loader 解析的模块标识

后端入口建议支持：

- `file`
- `command`
- `process`
- `service`

## 10. Extension Runtime 子系统

### 10.1 Workspace Registry

负责发现和聚合扩展来源：

- 扫描 installed source
- 扫描 workspace source
- 建立扩展 ID 到 source 的映射
- 处理冲突、遮蔽和优先级
- 输出待解析的扩展描述集

### 10.2 Watch Service

负责监听：

- 扩展描述文件
- 前端源码
- 后端源码
- locale / theme 资源
- 构建输出
- dev server 就绪状态
- backend runner 健康状态

标准事件建议包括：

- `extension.discovered`
- `extension.changed`
- `extension.rebuild_started`
- `extension.runtime_ready`
- `extension.runtime_failed`
- `extension.unloaded`
- `extension.registry_swapped`

### 10.3 Frontend Module Resolver

负责把扩展前端解析为 Web 可挂载入口：

- 开发态优先解析 `dev.frontend_dev_url`
- 无 dev URL 时可回退本地模块 loader
- 发布态解析 `build.frontend_bundle`
- 输出统一的前端模块句柄给 Web

### 10.4 Backend Runner Manager

负责托管扩展后端逻辑：

- 启动扩展独立 runner
- 执行健康检查
- 管理 stdout/stderr 与诊断
- 在新 generation ready 后切换流量
- 对旧 generation 执行 drain 和回收

### 10.5 Extension Graph Store

负责保存当前可用的扩展图谱：

- registry snapshot
- pages / panels / themes / locales / commands / providers / hooks
- generation
- per-extension health
- diagnostics

对外以只读快照和增量事件两种方式暴露。

## 11. Web 挂载模型

Web 应从“请求一次 registry 快照”演进为“快照 + 订阅”模型：

1. 启动时拉取 `ui runtime snapshot`
2. 建立 WS 或 SSE 订阅
3. 收到 `extension.registry_swapped` 后增量刷新导航、面板、命令、主题与语言包
4. 对支持 HMR 的扩展前端执行热替换或 remount
5. 对不可热替换的扩展执行局部卸载与重挂载

Web 至少需要支持：

- 动态页面注册
- 动态面板注册
- 主题资源刷新
- locale bundle 增量注入
- command palette 动态更新
- diagnostics 面板展示扩展健康状态

## 12. API 与运行时事件

目标态建议提供：

- `GET /api/v1/extensions/runtime`：运行时快照
- `GET /api/v1/extensions/runtime/events`：SSE 事件流
- `POST /api/v1/extensions/:id/reload`：重新解析扩展
- `POST /api/v1/extensions/:id/restart`：重启后端 runner
- `GET /api/v1/extensions/:id/diagnostics`：获取诊断
- `POST /api/v1/extensions/attach`：附加源码工作区
- `DELETE /api/v1/extensions/attach/:id`：解除附加

事件载荷中建议带上：

- `generation`
- `extension_id`
- `event`
- `health`
- `changed_capabilities`
- `diagnostics`
- `occurred_at`

## 13. CLI 目标态

`ennoia dev` 应演进为开发编排入口，而不是仅启动服务。

建议命令：

- `ennoia dev`
- `ennoia ext list`
- `ennoia ext inspect <id>`
- `ennoia ext reload <id>`
- `ennoia ext restart <id>`
- `ennoia ext logs <id>`
- `ennoia ext doctor <id>`
- `ennoia ext graph`
- `ennoia ext attach <path>`

其中：

- `dev` 负责启动主服务、Web dev server、watch graph 与 runner manager
- `attach` 负责把一个源码目录注册为 workspace source
- `doctor` 负责输出 manifest、依赖、watcher、runner、health 的综合诊断

## 14. 目录规划

目标态建议约定一套扩展工作区目录：

```text
extensions/
  observatory/
    ennoia.extension.toml
    package.json
    src/
      frontend/
      backend/
      locales/
      themes/
    dist/
    .ennoia/
      cache/
      logs/
      state/
```

实例级开发态目录建议补充：

```text
~/.ennoia/
├─ dev/
│  ├─ workspaces.toml
│  ├─ cache/
│  └─ logs/
```

这里的 `dev/` 只用于开发态运行时，不参与最终发布包。

## 15. 兼容与迁移原则

为了保证系统可演进，迁移期建议遵守：

- 旧的 `manifest.toml + install_dir` 安装协议继续可读
- 新的 workspace descriptor 作为优先级更高的来源
- 运行时统一输出新的 `ExtensionRuntimeSnapshot`
- 现有 `/api/v1/extensions/registry` 在过渡期内保留只读兼容视图

## 16. 分阶段落地建议

### 阶段一：协议与运行时容器

- 引入 `ExtensionDescriptor`、`ResolvedExtension`、`ExtensionGeneration`
- 把静态 registry 改为可热替换 runtime store
- 为 Web 提供新的 runtime snapshot

### 阶段二：开发态工作区接入

- 支持 workspace source provider
- 支持 `attach`、watch 与 reload
- 支持 dev diagnostics

### 阶段三：前后端开发运行时

- 前端支持 dev URL / HMR
- 后端支持 runner manager 与 generation 切换
- Web 支持 snapshot + subscription

### 阶段四：发布态统一

- 由同一份 descriptor 生成 package 产物
- 把开发态与发布态收敛到统一协议

## 17. 非目标

本文档当前不覆盖：

- 扩展权限模型的细粒度沙箱设计
- 第三方 marketplace 的签名、信任和分发协议
- 多实例共享 extension cache 的部署策略
- 远程云端 runner 的编排细节

这些能力可以在 `Workspace Extension Runtime` 稳定后继续扩展。
