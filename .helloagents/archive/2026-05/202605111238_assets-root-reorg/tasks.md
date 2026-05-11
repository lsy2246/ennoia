# 任务清单: assets-root-reorg

> **@status:** completed | 2026-05-11 12:49

```yaml
@feature: assets-root-reorg
@created: 2026-05-11
@status: completed
@mode: R3
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 4 | 0 | 0 | 4 |

---

## 任务列表

### 1. 目录与资源收集

- [√] 1.1 将 `builtins/extensions` 与 `builtins/skills` 迁移到 `assets/extensions`、`assets/skills`，并压平 `assets/templates/config/*` 到 `assets/templates/*` | depends_on: []
- [√] 1.2 更新 `crates/assets` 与 `crates/cli` 中的 builtin/template 路径解析和测试断言 | depends_on: [1.1]

### 2. 工具链与文档

- [√] 2.1 更新脚本、前端配置和测试中的源码路径引用 | depends_on: [1.1]
- [√] 2.2 更新 README 与 `docs/` 中关于内置源码根和模板路径的说明，并执行仓库校验 | depends_on: [1.2, 2.1]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-05-11 12:40 | 方案包初始化 | 完成 | 已确认 assets 重组范围，packaging 暂不处理 |
| 2026-05-11 12:49 | 1.1 | 完成 | 已迁移内置扩展/技能源码目录，并压平模板路径 |
| 2026-05-11 12:52 | 1.2 | 完成 | 已更新 crates/assets 与 crates/cli 的路径解析、watcher 和测试断言 |
| 2026-05-11 12:55 | 2.1 | 完成 | 已更新 Cargo workspace、Dockerfile、构建脚本和前端配置中的路径引用 |
| 2026-05-11 13:00 | 2.2 | 完成 | README、docs、知识库已同步；cargo/web/worker 校验已通过 |

---

## 执行备注

> 记录执行过程中的重要说明、决策变更、风险提示等

- `web lint` 首次执行时因 `panda:codegen` 触发 Windows `EPERM` 失败，复跑后通过，判断为环境锁冲突而非路径回归。
