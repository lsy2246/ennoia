# Ennoia 开发约定

本文档定义 `Ennoia` 源码仓库的长期开发约定。开发规范总入口见根目录 `AGENTS.md`。

## 1. 目标

本规范解决三个问题：

- 让仓库内各模块保持统一的工程边界
- 让代码、运行目录、配置协议、文档持续一致
- 让“什么放在代码、什么放在配置、什么放在 migration、什么走结构化层”有明确标准

## 2. 基本原则

### 2.1 变更原则

- 优先最小化变更范围
- 优先修根因
- 改动范围聚焦当前任务
- 新增复杂度必须有明确收益

### 2.2 一致性原则

- 文档、代码、目录、配置协议必须同步
- 运行时对外暴露的行为必须稳定、清晰、可验证
- 同类问题采用同一解决方式，数据库运行时查询统一走结构化生成

### 2.3 验证原则

- 声称“可运行”“可启动”“热加载正常”时，必须实际启动验证
- 声称“已修复某个报错”时，必须针对原始复现场景复测
- 构建通过不等于运行通过

## 3. 仓库边界

- `crates/`：Rust 核心与后端模块
- `web/`：前端主壳与前端扩展接入层
- `sdk/`：扩展与技能开发 SDK
- `packaging/`：打包、发布、运行目录模板
- `docs/`：架构、开发、协议、运行文档

## 4. 文档分层

### 4.1 文档职责

- `README.md`：项目介绍、启动方式、验证命令、文档入口
- `AGENTS.md`：开发规范总入口与 Agent 协作执行约束
- `docs/development/conventions.md`：详细开发规则

### 4.2 文档更新规则

以下情况必须同步更新文档：

- 架构变化：更新 `docs/architecture.md`
- 运行目录变化：更新 `docs/runtime-layout.md`
- 扩展开发协议变化：更新 `docs/extension-development.md`
- 开发规范变化：更新 `AGENTS.md` 与本文档
- 使用方式变化：更新 `README.md`

### 4.3 文档写作风格

- 使用正向表达，写清楚“应该做什么、如何做、做到什么标准”
- 规则描述以可执行动作开头
- 说明约束时给出推荐路径、默认做法和判断标准
- 需要表达边界时，优先写“采用什么方式”“放到哪里”“由谁负责”

## 5. Rust 后端约定

### 5.1 代码组织

- `kernel` 承载共享协议、配置结构和扩展 manifest 模型
- `extension-host` 承载扩展扫描、attach、reload、restart、诊断、Worker 解析与 Worker RPC 分发
- `server` 承载 API 暴露、TOML 配置文件、日志、Hook 派发、能力路由与启动流程
- 内置扩展实现放在 `builtins/extensions/<extension_id>/ui|worker|data`，不得把 session、memory、workflow、任务编排等业务实现混回核心 crate
- Wasm Worker 使用 `ennoia.worker` ABI，宿主默认不注入 WASI/import；需要宿主能力时必须先在 `permissions` 中声明，并通过统一 capability bridge 接入
- 公共转换逻辑提取为函数或模块级 helper

### 5.2 命名

- 类型名使用清晰的领域名词
- 使用完整、可读的命名
- 布尔函数优先使用 `is_*` / `has_*` / `should_*`
- 数据库表枚举、列枚举、运行时模型名称保持可直接对应

### 5.3 错误处理

- 错误必须保留真实原因
- 错误向上传递时保留底层上下文
- 对用户可见的错误要可理解，对开发者可见的错误要可定位

### 5.4 日志边界

- 长生命周期后端模块统一使用 `tracing` 记录运行日志，需要进入产品可查询日志时再写入 `system_log`
- `server` 路由、扩展调度、Hook 派发、Worker RPC 和后台轮询不得使用 `println!`、`eprintln!` 或 `dbg!`
- `cli` 的 `stdout` / `stderr` 只用于直接面向操作者的终端输出，不把它当作系统日志通道
- `ennoia dev` 始终把 API、Web 和扩展 UI watcher 输出写入日志文件，并通过 `config/server.toml` 的 `logging.dev_console` 控制是否按级别镜像到命令窗口
- 需要统一前后端系统日志级别时，优先使用环境变量 `ENNOIA_LOG_LEVEL`；开发态命令窗口镜像继续只走配置文件与可视化面板
- `build.rs` 只输出 Cargo 约定的构建指令，例如 `cargo:rerun-if-changed=...`

## 6. 前端约定

### 6.1 主壳原则

- `web/` 是当前唯一正式 Web 工作台
- 路由入口组件放在 `web/src/pages/`
- 页面内部资源视图放在 `web/src/views/<page>/`
- 扩展页面、面板、导航注册统一通过运行时快照接入
- 扩展 UI 实现、文案、主题和页面描述必须保留在扩展包内；Web 主壳不得静态注册某个扩展的页面组件、主题或 i18n namespace
- Web 主壳加载扩展 UI 时只使用 runtime snapshot 与 `/api/extensions/{extension_id}/ui/module`，不得重新引入 `import.meta.glob` 或源码路径白名单来绑定具体扩展
- 扩展主题必须遵循 `ennoia.theme`，只通过稳定 CSS 变量贡献观感，不得覆盖主壳内部 class 或 DOM 结构

### 6.2 状态与 API

- 前端 API 访问统一走 `web/packages/api-client`
- 运行时扩展接入统一走 `web/packages/ui-sdk`
- 全局状态、路由、扩展刷新机制必须保持单一路径

### 6.3 前端日志

- 前端运行时代码统一通过 `@ennoia/observability` 的 `createLogger()` 输出日志
- 业务包、运行时包、扩展 UI 和主题运行时不得直接调用 `console.log/info/warn/error/debug`
- 浏览器控制台只作为 `web/packages/observability` 的底层适配层，不作为业务代码直接依赖的接口
- 前端日志级别优先读取 `VITE_ENNOIA_LOG_LEVEL`；若未提供，则回退到统一变量 `ENNOIA_LOG_LEVEL`
- 需要面向用户展示状态时，使用 UI 提示、状态栏或调试面板；不要用控制台消息代替产品反馈

## 7. 存储与迁移规范

系统级配置只走 TOML 文件。核心不维护主业务 SQLite；扩展按自己的私有数据目录管理存储与迁移。

### 7.1 扩展私有迁移

扩展 Worker 如果使用数据库，schema、迁移和初始化入口必须放在该扩展自己的 `data/` 或 `worker/` 边界内，例如：

- `builtins/extensions/memory/data/schema.sql`
- `builtins/extensions/workflow/data/schema.sql`

核心仓库根目录不再提供主库基线文件。

### 7.2 运行时查询规范

扩展内数据库读写默认遵循：

- 使用结构化查询生成器构建 SQL
- SQLite 执行层使用 `sqlx`
- 业务层通过结构化查询表达读写意图

当前仓库的默认方案是：

- 查询构建：`SeaQuery`
- 执行：`sqlx`

### 7.3 允许保留原始 SQL 的例外

以下场景使用原始 SQL：

- migration 文件内容本身
- migration 执行器逐条执行 migration asset
- 当前结构化工具表达能力不足且已说明原因的极少数例外

例外需要满足：

- 原因明确
- 影响范围小
- 普通业务查询继续采用结构化查询

### 7.4 查询边界

- 表结构定义在 migration
- 运行时 CRUD 在 Rust 结构化查询层
- 业务代码采用统一的结构化查询表达
- `facts` crate 承载主系统事实数据访问层与初始化 / 迁移执行入口

## 8. 配置与路径约定

- 面向用户的默认运行目录固定为 `~/.ennoia/`
- 路径优先通过 `paths` 模块和配置推导
- 配置协议变更必须同步更新模板与文档
- 开发入口优先统一走 `cargo run -p ennoia-cli -- dev`
- 系统配置只表达宿主、接口绑定、资源实例和实例偏好；扩展私有业务配置不得上浮为核心配置结构或 `config/` 根目录文件

## 9. 扩展与热加载约定

- 扩展扫描、attach、reload、restart 必须统一走 Extension Runtime
- 开发来源扩展与已安装扩展共用统一描述协议
- Web dev server、Extension Runtime 扫描和 Worker RPC 行为必须纳入一键开发链路
- 热加载相关问题必须以真实启动链路验证

## 10. 测试与校验

### 10.1 默认校验

提交前默认执行：

```bash
cargo fmt --all
cargo check --workspace
cargo test --workspace
bun run --cwd web typecheck
bun run --cwd web build
```

### 10.2 验证策略

- 后端改动：优先集成验证、最小复现实验、真实接口调用
- 前端改动：至少验证 Web 工作台加载、页面挂载、无控制台错误
- 扩展改动：验证扫描、attach、reload、重启生效链路
- 开发链路改动：必须实际验证 `cargo run -p ennoia-cli -- dev`

### 10.3 测试放置规则

- 小型纯函数、解析器和局部状态机允许使用 `#[cfg(test)] mod tests` 与源码同文件放置
- 涉及文件系统、进程、网络、跨模块装配或长样例数据的测试，优先放到 crate 的 `tests/` 集成测试目录或独立测试支持模块
- 当单个源码文件的测试开始明显挤占实现阅读空间时，应把测试迁出实现文件，避免模块职责混杂
- 新增测试时优先按“单元测试贴近实现、集成测试贴近系统边界”的原则分层

## 11. 提交与清理

提交前必须清理：

- 临时测试文件
- 本地调试脚本
- 日志文件
- 无关构建产物

提交前至少检查：

```bash
git status
```

若出现范围外文件，应先清理或补充忽略规则。

## 12. 结果汇报要求

每次开发完成后的汇报至少包含：

- 改动文件
- 执行命令
- 命令结果
- 未完成项 / 风险 / 未接入校验

如果声称已修复运行问题，必须附带真实复测结论。
