# Integration Tests

本目录存放 Rust 服务层、配置加载、扩展扫描和主链路集成测试。

当前入口：

- `runtime-smoke.mjs`：通过 `cargo run -p ennoia-cli -- init/start` 初始化临时 runtime，验证健康检查、extension registry、page/panel 挂载协议

执行方式：

- 仓库根目录执行 `bun run test:integration`
