# Ennoia 路线图

本文件是 `Ennoia` 当前阶段的唯一实施清单。

## 1. 产品目标

Ennoia 的首版目标是形成一个长期可演进的 AI Agent 平台基础盘：

- 支持私聊与群聊
- 支持多 Agent 协作
- 支持可视化工作区、任务、日志与记忆
- 支持系统级扩展与技能生态
- 支持高性能 Rust 后端和可扩展前端主壳

## 2. 模块清单

### 平台核心

- `kernel`
- `memory`
- `orchestrator`
- `scheduler`
- `extension-host`
- `server`
- `cli`

### 前端与 SDK

- `web/shell`
- `web/ui-sdk`
- `web/builtins`
- `sdk/extension-sdk`
- `sdk/skill-sdk`

### 交付与运维

- `migrations`
- `packaging/home-template`
- `packaging/npm`
- `.github/workflows`
- `tests`

## 3. 主链路

首版主链路包含六条：

1. 私聊链路
2. 群聊链路
3. Run / Task 编排链路
4. Memory 组装与回写链路
5. Scheduler 定时与后台任务链路
6. Extension 注册与前端挂载链路

## 4. 阶段拆分

### Phase 1：事实源与协议

- 架构文档
- 路线图
- 数据模型
- 配置模型
- Hook / event 字典
- API 边界
- Extension 开发指南

验收：

- `docs/` 能完整描述系统的结构、边界和实现方向

### Phase 2：Rust 核心骨架

- `kernel`：对象模型、配置协议、manifest 契约
- `memory`：记忆模型和上下文组装接口
- `orchestrator`：run、task、owner 和编排接口
- `scheduler`：job、cron、delay、retry 模型
- `extension-host`：manifest、registry、扫描接口
- `server`：应用状态、路由、健康检查、注册表输出
- `cli`：`init`、`dev`、`print-config` 等入口

验收：

- `cargo check --workspace`
- `cargo test --workspace`

### Phase 3：前端主壳骨架

- Vite + React + Bun + Panda CSS
- 主壳布局
- 子页面容器
- 面板区域占位
- 扩展注册表消费模型
- UI SDK 和 builtins 占位包

验收：

- `bun run --cwd web/shell build`

### Phase 4：配置、打包与模板

- `~/.ennoia/` 模板
- Agent 配置模板
- Extension 配置模板
- npm 打包目录骨架

验收：

- 初始化模板完整
- 文档与模板字段一致

### Phase 5：测试与 CI

- Rust 单元测试
- 测试目录骨架
- GitHub Actions 基线

验收：

- CI 能执行 Rust 与 Web 的基础校验

## 5. 模块完成定义

### kernel

- 有稳定对象模型
- 有 owner / thread / run / task / extension 基本契约
- 有配置结构和 manifest 结构

### memory

- 有记忆实体结构
- 有 context view 结构
- 有 recall / remember / review 基本接口

### orchestrator

- 有私聊 run 和群聊 run 两类入口
- 有 task 生成与 owner 归属
- 有最小执行计划结构

### scheduler

- 有 delay / cron / maintenance 三类 job 模型
- 有 job 注册接口

### extension-host

- 有 manifest 解析
- 有 registry
- 有扫描目录接口

### server

- 有应用状态
- 有健康检查
- 有系统概览接口
- 有扩展注册表接口

### cli

- 有 `init`
- 有 `dev`
- 有 `print-config`

### web/shell

- 有主壳布局
- 有子页面区域
- 有面板区域
- 有示例导航和示例页面

## 6. 当前迭代交付

本轮交付覆盖：

- 全部模块文档补齐
- 全部核心模块代码骨架补齐
- Web Shell 骨架落地
- 配置模板、迁移、测试、CI 骨架补齐

本轮的目标是“完整骨架版”，重点是边界清晰、链路完整、后续可持续扩展。
