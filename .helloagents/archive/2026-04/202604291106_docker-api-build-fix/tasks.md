# 任务清单: docker-api-build-fix

> **@status:** completed | 2026-04-29 11:30

```yaml
@feature: docker-api-build-fix
@created: 2026-04-29
@status: completed
@mode: R2
```

## 进度概览

| 完成 | 失败 | 跳过 | 总数 |
|------|------|------|------|
| 3 | 0 | 0 | 3 |

---

## 任务列表

### 1. Docker 构建修复

- [√] 1.1 复现 `api` Docker 构建失败并确认根因是 workspace 成员目录未复制进镜像 | depends_on: []
- [√] 1.2 在 `Dockerfile` 中补齐 `api/web` 构建所需目录与安装策略，确保容器内 workspace 和前端构建输入完整 | depends_on: [1.1]

### 2. 验证与收尾

- [√] 2.1 重新执行 Docker API 目标构建并记录验证结果，必要时同步知识库变更记录 | depends_on: [1.2]

---

## 执行日志

| 时间 | 任务 | 状态 | 备注 |
|------|------|------|------|
| 2026-04-29 11:10:00 | 1.1 | completed | 复现 `docker build --target api -f Dockerfile .`，确认 `/app` 缺少 `builtins/extensions/*` workspace 成员 |
| 2026-04-29 11:15:00 | 2.1 | info | 追加验证发现 `docker compose build` 还会在 `web` 的 `bun install` 阶段失败，缺少 `scripts/` 与 `builtins/` 构建输入且 `prepare` 触发过早 |
| 2026-04-29 11:18:00 | 1.2 | completed | 完成 `Dockerfile`、`web/package.json`、`web/vite.config.ts` 与 `scripts/build-extension-ui.mjs` 调整 |
| 2026-04-29 11:31:00 | 2.1 | completed | `docker compose build` 成功；本地 `bun run --cwd web build` 通过；后续一次复跑因 Docker Hub DNS 解析失败未纳入代码回归 |

---

## 执行备注

> 记录执行过程中的重要说明、决策变更、风险提示等
- 工作区存在用户已有未提交改动：`builtins/extensions/memory/bin/memory-service.exe`、`bun.lock`，本次不处理。
- 验证期间 `bun run --cwd web lint` 仅发现仓库已有 4 条 warning，无新增 error。
