# 任务清单: web-name-refactor

@feature: web-name-refactor
@created: 2026-04-22
@status: completed
@mode: implementation

## 进度概览

- 完成: 6
- 失败: 0
- 跳过: 0
- 总数: 7

## 任务列表

### 1. 完整扫描与方案记录

- [√] 1.1 扫描路径、文件名、变量、i18n namespace、测试、Dockerfile、文档和知识库中的旧前端命名残留 | depends_on: []
- [√] 1.2 记录不兼容一次性重构方案与系统命令语义边界 | depends_on: [1.1]

### 2. 配置与运行入口

- [√] 2.1 修正 `Dockerfile`、`.dockerignore` 和 CLI 前端 dev 进程命名，统一使用 `web` 路径与进程语义 | depends_on: [1.2]

### 3. 前端源码与 i18n

- [√] 3.1 删除旧 i18n 模块，切换内置消息 namespace 和前端代码引用到 `web` | depends_on: [2.1]
- [√] 3.2 重命名前端路由变量、route id、CSS class 和配置写入者标识 | depends_on: [3.1]

### 4. 测试、文档与知识库

- [√] 4.1 更新测试请求、文档说明、知识库模块和变更记录，删除未跟踪旧前端目录 | depends_on: [3.2]

### 5. 验证

- [√] 5.1 执行 Rust 与前端校验命令，修正本轮引入的问题 | depends_on: [4.1]

## 执行日志

- 2026-04-22 10:33 | 1.1-1.2 | 已完成 | 完成残留扫描并创建方案包
- 2026-04-22 10:52 | 2.1-4.1 | 已完成 | 完成代码、配置、文档、知识库与历史记录命名重构
- 2026-04-22 11:00 | 5.1 | 已完成 | `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`、`bun run --cwd web typecheck`、`bun run --cwd web build` 通过

## 执行备注

- PowerShell、进程执行选项和 Linux 登录解释器语义保持不变。
