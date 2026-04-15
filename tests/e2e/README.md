# E2E Tests

本目录存放 Shell、私聊、群聊、扩展挂载和面板行为的端到端测试。

当前入口：

- `platform-smoke.mjs`：通过真实 server 启动链路验证 overview、private run、space run、job、memory 与 artifact 落盘

执行方式：

- 仓库根目录执行 `bun run test:e2e`
