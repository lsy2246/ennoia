---
name: web-search
description: 基于本地自动化浏览器或 MCP 浏览器连接的网页搜索与候选页面提取技能。用于搜索网页资料、打开候选页面并提取正文。
---

# Web Search

这个技能是 Ennoia 默认的网页搜索技能包，使用浏览器完成搜索、开页和正文提取。

- 对外能力：网页搜索、候选页面抓取、正文提取
- 浏览器控制方式：`local` 或 `mcp`
- 本地自动化驱动：CloakBrowser
- 本地浏览器 API：Playwright 风格接口
- 本地浏览器内核来源：`builtin`、`system_auto`、`system_path` 三选一且互斥
- 对外动作入口：
  - `search` -> `scripts/search-runner.mjs`
- 内部辅助脚本：
  - `scripts/browser-open.mjs`
  - `scripts/prepare-browser.mjs`
  - `scripts/doctor.mjs`

## 用途

适合这些任务：

- 搜网页资料
- 找官网、文档、公告页
- 继续抓取候选结果页并提取正文
- 从页面里拿标题、摘要、时间、链接和主内容

它不负责任务编排和调度。MCP 浏览器连接作为 `web-search` 的一种控制方式存在，不再拆成独立技能。

## 浏览器控制方式

`browser_control` 是浏览器控制方式，默认值是 `local`：

- `local`：使用本地自动化浏览器。该模式下继续使用 `browser_kernel` 选择内核来源。
- `mcp`：直接连接本技能配置的 MCP 浏览器服务。该模式需要填写 `mcp_transport` 和 `mcp_url`，不填写浏览器工具名。

两种方式互斥。选择 `mcp` 后，`browser_kernel` 和 `browser_executable_path` 不参与运行。

## 本地内核模式

`browser_kernel` 只在 `browser_control = "local"` 时生效，默认值是 `builtin`：

- `builtin`：使用 Ennoia 管理的 CloakBrowser Chromium 缓存。运行搜索前必须先准备缓存；搜索过程中不会临时下载内核。
- `system_auto`：自动查找系统中的 Chrome、Edge、Brave 或 Chromium。
- `system_path`：只使用 `browser_executable_path` 指定的浏览器可执行文件。

选择 `system_path` 时，`browser_executable_path` 必须指向存在的可执行文件；路径不存在时直接报配置错误，不回退到其他模式。

`builtin` 的准备入口：

```bash
ennoia skill prepare web-search
```

这个命令会先确保运行目录中的 `web-search` 技能依赖可用，再把 CloakBrowser Chromium 下载并解压到：

```text
~/.ennoia/data/skills/web-search/cloakbrowser/
```

如果下载失败，搜索运行时不会自动切换到系统浏览器，也不会在搜索过程中重试下载；需要重新执行 prepare，或显式把 `browser_kernel` 改成 `system_auto` / `system_path`。

## MCP 模式

选择 `browser_control = "mcp"` 时，配置里只需要：

```toml
browser_control = "mcp"
mcp_transport = "streamable-http"
mcp_url = "http://127.0.0.1:12306/mcp"
```

`mcp_url` 可以是本地服务地址，也可以是远程 MCP 服务地址，例如 `https://example.com/mcp`。当前脚本会返回结构化的 `mcp_provider_unavailable` 结果，表示技能配置已进入 MCP 模式，但实际浏览器工具发现与调用还需要 MCP provider 接通。

## 调用约定

系统侧现在只关心：

- `skill_id = "web-search"`
- `action_id = "search"`
- `input`
- `output`

`input` / `output` 的具体字段不在 `config.toml` 里强定义，直接以这里的说明和示例为准。

### search

适合：

- 先搜结果，再决定要不要继续抓页面
- 限定搜索词、结果数、抓取页数、输出格式

常见输入示例：

```json
{
  "query": "cloakbrowser playwright",
  "limit": 5,
  "pages": 3,
  "format": "json"
}
```

当 `pages > 0` 时，这个入口会继续抓取候选页，并返回提取后的正文、标题、摘要、时间和链接。

## 输出

默认输出 JSON，包含：

- `query`：查询词
- `engine`：对外统一为 `builtin-browser`
- `browser_control`：浏览器控制方式。MCP 模式下会出现该字段；本地模式下以本地内核字段表达运行状态。
- `browser_kernel_mode`：本地内核来源模式，值为 `builtin`、`system_auto` 或 `system_path`
- `browser_kernel`：本地实际使用的内核，例如 `builtin`、`chrome`、`edge`、`brave`、`chromium` 或 `custom`
- `browser_kernel_name`：本地实际使用的内核名称
- `runtime.driver`：本地模式固定为 `cloakbrowser`
- `mcp_transport`：MCP 模式下使用的传输方式
- `mcp_url`：MCP 模式下连接的服务地址
- `status`：MCP provider 未接通时为 `mcp_provider_unavailable`
- `results`：搜索候选结果
- `pages`：候选页面正文提取结果或错误信息

也可以通过 `--format markdown` 输出 Markdown。

## 命令行入口

### 1. 对外统一搜索入口

```bash
node scripts/search-runner.mjs "cloakbrowser playwright"
```

常用参数：

```bash
node scripts/search-runner.mjs "openai api responses" --limit 5 --pages 3
node scripts/search-runner.mjs "ennoia skill system" --format markdown
```

### 2. 内部浏览器辅助脚本

```bash
node scripts/browser-open.mjs "https://example.com"
node scripts/browser-open.mjs "https://example.com" --screenshot example.png
```

适合调试 local 模式下浏览器内核能否开页、取标题和截图。它不是对外 action。

### 3. 环境自检

```bash
node scripts/doctor.mjs
```

自检会检查浏览器控制方式。local 模式检查依赖包、当前内核模式、可选手动路径和自动发现结果；mcp 模式检查 `mcp_transport` 与 `mcp_url`，并提示 MCP provider 连通性由后续接入负责。

### 4. 准备内置浏览器

```bash
node scripts/prepare-browser.mjs
```

这个脚本只负责准备 local + `builtin` 模式使用的 CloakBrowser Chromium 缓存。运行目录推荐使用 `ennoia skill prepare web-search`，因为 CLI 会先同步内置技能并安装技能依赖。

## 本地依赖

如果你要在本地直接调试这些脚本，请在技能目录安装依赖：

```bash
bun install
```

默认 `builtin` 模式不需要执行 `playwright install chromium`。它使用 `ennoia skill prepare web-search` 准备的 Ennoia 托管 CloakBrowser Chromium 缓存。

详情见：

- [tooling.md](references/tooling.md)
- [platforms.md](references/platforms.md)
