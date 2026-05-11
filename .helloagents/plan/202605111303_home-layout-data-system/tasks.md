# 任务清单: home-layout-data-system

```yaml
@feature: home-layout-data-system
@created: 2026-05-11
@status: completed
@mode: R2
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 4 | 0 | 0 | 4 |

---

## 任务列表

### 1. 方案与路径模型

- [x] 1.1 在 `.helloagents/plan/202605111303_home-layout-data-system/proposal.md` 中补齐目录模型决策与影响范围
- [x] 1.2 在 `crates/paths/src/lib.rs` 中把日志与 pid 路径收敛到 `data/system/logs`、`data/system/pids`

### 2. 调用方与文档同步

- [x] 2.1 在 `scripts/cli-launcher.mjs` 中同步 stop 读取的 pid 文件路径
- [x] 2.2 在 `README.md`、`docs/runtime-layout.md`、`docs/data-model.md`、`docs/architecture.md` 中同步新的 `config` / `data` 目录语义
- [x] 2.3 运行格式化与 Rust 校验，确认路径重构没有破坏现有行为

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-05-11 | 1.1 | 已完成 | 补齐本轮目录收敛方案与决策记录 |
| 2026-05-11 | 1.2 | 已完成 | `RuntimePaths` 新增 `system_logs_dir()` / `system_pids_dir()`，并让日志与 pid 接口全部转向 `data/system/*` |
| 2026-05-11 | 2.1 | 已完成 | `npm run stop` 的 pid 读取路径同步到 `data/system/pids/*.pid` |
| 2026-05-11 | 2.2 | 已完成 | README 与运行目录/数据模型/架构文档全部切换到新的目录语义 |
| 2026-05-11 | 2.3 | 已完成 | 已执行 `cargo fmt --all`、`cargo check --workspace`、`cargo test --workspace`、`bun run --cwd web lint`、`bun run --cwd web typecheck`、`bun run --cwd web build` |

---

## 执行备注

> 用户已确认本轮不讨论 packaging，仅聚焦用户目录的 `config` / `data` 模型收敛。
