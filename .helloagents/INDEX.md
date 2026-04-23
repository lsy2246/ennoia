# Ennoia 知识库

```yaml
kb_version: 1
project: Ennoia
updated_at: 2026-04-23
```

## 当前状态

- 系统核心收敛为配置、路径、日志、扩展宿主、接口绑定和 scheduler
- 扩展执行单元采用 Wasm Worker，能力通过 manifest 贡献声明
- 会话、运行、任务、产物和定时业务通过细粒度 `interfaces` / `schedule_actions` 路由到扩展
- 根目录提供 `bun run build:workers`、`bun run --cwd web typecheck`、`cargo check --workspace` 等验证入口

## 活跃方案包

- 当前无活跃方案包
