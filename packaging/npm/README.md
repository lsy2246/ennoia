# Ennoia npm Packaging

本目录承载 `ennoia` 的 npm 发布入口。

当前约定：

- npm 包负责暴露统一的 `ennoia` 命令
- 运行目录模板由 Rust CLI 内嵌
- `bin/ennoia.js` 负责定位本地已构建的 `ennoia` 可执行文件并转发参数

本地联调方式：

1. 在仓库根目录执行 `bun run setup`
2. 进入 `packaging/npm/`
3. 执行 `node ./bin/ennoia.js init`
4. 执行 `node ./bin/ennoia.js start`

后续发布阶段会继续补充：

- 平台原生二进制分发目录
- npm 安装后的本地可执行文件装配
- 发布流水线校验
