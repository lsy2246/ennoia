# 平台约束

## 原生支持

- macOS
- Linux

`@lightpanda/browser` 的 npm postinstall 会在这些平台自动下载 Lightpanda 二进制。

## Windows

当前官方 npm 包不会为 Windows 下载原生 Lightpanda 二进制。

在 Windows 上使用本技能时，推荐两种方式：

1. 在 `WSL2` 里运行 `node scripts/setup.mjs` 和 `node scripts/search-runner.mjs`
2. 手动提供一个可执行的 Lightpanda 二进制，并设置：

```powershell
$env:LIGHTPANDA_EXECUTABLE_PATH = "D:\\path\\to\\lightpanda.exe-or-wrapper"
```

如果没有 `LIGHTPANDA_EXECUTABLE_PATH`，`doctor` 会直接报错。
