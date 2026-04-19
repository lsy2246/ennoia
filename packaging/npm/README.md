# Ennoia npm Packaging

本目录承载 `ennoia` 的 npm 发布入口。

当前约定：

- npm 包负责暴露统一的 `ennoia` 命令
- 运行目录模板由 Rust CLI 内嵌
- `vendor/` 承载当前平台已打包的 `ennoia` 可执行文件
- `bin/ennoia.js` 优先使用包内 `vendor` 二进制，并兼容本地源码联调

本地联调方式：

1. 在仓库根目录执行 `bun run setup`
2. 进入 `packaging/npm/`
3. 执行 `node ./bin/ennoia.js init`
4. 执行 `node ./bin/ennoia.js start`

当前平台 npm 包打包方式：

1. 在仓库根目录执行 `bun run package:npm`
2. 产物输出到 `dist/npm/`
3. 用 `npm install -g <tarball>` 安装

补充说明：

- 打包脚本会先构建当前平台 release CLI
- `vendor/` 目录由打包脚本临时生成，打包完成后自动清理
- 当前 tarball 仅适用于打包时所在的平台
4. 直接执行 `ennoia init` / `ennoia start`

后续发布阶段会继续补充：

- 多平台二进制矩阵构建
- 自动发布流水线校验
