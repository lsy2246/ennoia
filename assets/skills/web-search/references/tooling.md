# 工具分工

## 浏览器控制方式

本技能只有一个对外入口：`web-search`。浏览器控制方式由 `browser_control` 决定：

- `local`：默认方式，使用本地自动化浏览器执行搜索、开页和正文提取。
- `mcp`：直接连接本技能配置的 MCP 浏览器服务。

MCP 模式读取 `mcp_transport` 与 `mcp_url`。具体浏览器工具名、工具发现和调用路由由后续 MCP provider 接入负责，技能配置里不保存工具名。

## 本地浏览器内核

`browser_control = "local"` 时，本技能使用“内核自动化浏览器”这个概念。浏览器内核来源只有三种，并且每次只能选择一种：

- `builtin`：使用 Ennoia 管理的 CloakBrowser Chromium 缓存，运行搜索前通过 `ennoia skill prepare web-search` 准备。
- `system_auto`：自动查找系统已安装浏览器。
- `system_path`：使用用户填写的 `browser_executable_path`。

自动查找会检测：

- Google Chrome
- Microsoft Edge
- Brave
- Chromium

选择 `system_path` 后，技能只使用 `browser_executable_path`；路径不存在时直接报配置错误。

## 本地自动化驱动

当前自动化驱动使用 CloakBrowser 的 Playwright 风格 API。

它负责：

- 启动选定浏览器内核
- 打开页面
- 读取 HTML
- 截图
- 后续扩展点击、表单、快照等浏览器自动化能力

CloakBrowser 在这里提供自动化驱动封装和 Chromium 下载/缓存能力。`builtin` 会把已准备好的缓存路径通过 `CLOAKBROWSER_BINARY_PATH` 注入给 CloakBrowser，系统浏览器路径也通过同一个环境变量注入。

MCP 模式不加载 CloakBrowser，也不检查本地浏览器依赖。

## prepare-browser

这是 local + `builtin` 模式的准备入口。

它会：

1. 设置 `CLOAKBROWSER_CACHE_DIR` 到 `~/.ennoia/data/skills/web-search/cloakbrowser/`
2. 调用 CloakBrowser 的 `ensureBinary()`
3. 下载并解压当前平台的 CloakBrowser Chromium
4. 输出缓存目录、内核路径和版本

运行态推荐使用：

```bash
ennoia skill prepare web-search
```

这个 CLI 会先同步内置技能并安装技能依赖，再执行 `scripts/prepare-browser.mjs`。

## search-runner

这是本技能的统一搜索入口。

local 模式下它会：

1. 解析浏览器内核模式
2. 启动自动化浏览器
3. 打开 DuckDuckGo HTML 搜索结果页
4. 解析候选结果
5. 继续抓取候选详情页
6. 用 Readability 提取正文
7. 产出统一 JSON / Markdown 结果

MCP 模式下它会读取 `mcp_transport` 和 `mcp_url` 并返回结构化 MCP provider 状态；真实浏览器动作由 MCP provider 接通该服务后负责。

## browser-open

这是 skill 内部调试入口，不作为对外 action 暴露。

它用于：

- 验证浏览器内核能否启动
- 验证页面是否能打开
- 读取页面标题
- 按需保存截图

这个脚本面向 local 模式调试，不代表 MCP provider 的连接状态。

## doctor

这是本地环境自检入口。

它会检查：

- skill 依赖清单
- 当前浏览器控制方式
- local 模式下的 `cloakbrowser`
- local 模式下的 `playwright-core`
- local 模式下的正文提取依赖
- local 模式下的当前浏览器内核模式
- local 模式下的用户配置浏览器路径
- local 模式下自动发现到的系统浏览器内核
- mcp 模式下的 `mcp_transport`
- mcp 模式下的 `mcp_url`
