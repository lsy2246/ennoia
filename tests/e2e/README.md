# E2E Tests

本目录存放 Web 工作台与主链路的端到端测试。

当前入口：

- `platform-smoke.mjs`：通过真实 server 启动链路验证 bootstrap、direct/group conversation、job、memory 与 artifact 落盘

执行方式：

- 仓库根目录执行 `bun ./tests/e2e/platform-smoke.mjs`
