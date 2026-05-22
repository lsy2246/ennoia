# 平台约束

## 控制方式

`web-search` 支持两种浏览器控制方式：

- `browser_control = "local"`：使用本地自动化浏览器，适用下面的本地内核来源、自动查找和系统依赖约束。
- `browser_control = "mcp"`：直接连接本技能配置的 MCP 浏览器服务；平台约束由对应本地或远程 MCP 服务负责。

## 本地内核来源

local 模式下，浏览器内核来源必须三选一：

- `builtin`
- `system_auto`
- `system_path`

默认使用 `builtin`。这个模式只使用 Ennoia 管理的 CloakBrowser Chromium 缓存；运行搜索前需要先执行：

```bash
ennoia skill prepare web-search
```

准备后的内核位于：

```text
~/.ennoia/data/skills/web-search/cloakbrowser/
```

搜索运行时不会临时下载内核，也不会因为缓存缺失自动回退到系统浏览器。

## 自动查找规则

选择 local + `system_auto` 时才会自动查找系统浏览器。

### Windows

按常见安装目录查找：

- Google Chrome
- Microsoft Edge
- Brave
- Chromium

### macOS

按 `/Applications` 下的常见应用路径查找：

- Google Chrome
- Microsoft Edge
- Brave
- Chromium
- Google Chrome Canary

### Linux

按 `PATH` 中的可执行文件查找：

- `google-chrome`
- `google-chrome-stable`
- `chromium`
- `chromium-browser`
- `microsoft-edge`
- `microsoft-edge-stable`
- `brave-browser`

## 手动配置

选择 local + `system_path` 时填写：

```toml
browser_kernel = "system_path"
browser_executable_path = "C:/Program Files/Google/Chrome/Application/chrome.exe"
```

路径不存在时技能直接报配置错误，不回退到内置或自动查找模式。

## 系统依赖

local 模式在精简 Linux 环境中仍然依赖 Chromium 所需的系统库和字体。缺失系统依赖时，浏览器可能无法启动或页面渲染异常。

需要补齐系统依赖时，优先使用 Playwright 的系统依赖安装命令：

```bash
playwright install-deps chromium
```

这个命令只安装系统依赖，不下载 Playwright 自带 Chromium。
