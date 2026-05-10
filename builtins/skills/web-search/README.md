# Web Search

这个技能不是空文档，而是一个带本地工具的真实技能包。

- 搜索与开页工具：`agent-browser`
- 页面抓取引擎：`Lightpanda`
- 动作入口：
  - `search` -> `scripts/search-runner.mjs`
  - `open` -> `scripts/agent-browser-lightpanda.mjs`
  - `extract` -> `scripts/agent-browser-lightpanda.mjs`

## 用途

适合这些任务：

- 搜网页资料
- 找官网、文档、公告页
- 打开候选结果页并提取正文
- 从页面里拿标题、摘要、时间、链接和主内容

它不负责任务编排，也不负责调度；只负责“接到搜索任务后怎么搜、怎么开页、怎么提取”。

## 调用约定

系统侧现在只关心：

- `skill_id = "web-search"`
- `action_id`
- `input`
- `output`

`input` / `output` 的具体字段不在 `skill.toml` 里强定义，直接以这里的说明和示例为准。

### search

适合：

- 先搜结果，再决定要不要继续抓页面
- 限定搜索词、结果数、抓取页数、输出格式

常见输入示例：

```json
{
  "query": "lightpanda browser",
  "limit": 5,
  "pages": 3,
  "format": "json"
}
```

### open

适合：

- 需要继续手动或 Agent 交互浏览
- 需要点击、快照、截图、继续探索页面

常见输入示例：

```json
{
  "args": ["https://example.com"]
}
```

### extract

适合：

- 已经有候选 URL，需要直接拿正文与元信息
- 想复用 `agent-browser + Lightpanda` 的开页能力

目前 `extract` 和 `open` 共用同一入口；若要做更强的抽取行为，优先在这个 skill 里继续扩展，而不是新起一套假 skill。

## 安装

先在技能目录执行：

```bash
node scripts/setup.mjs
```

这会做三件事：

1. 用 `bun` 安装 skill 私有依赖
2. 信任 `agent-browser` / `@lightpanda/browser` 的 postinstall
3. 运行环境检查

## 命令行入口

### 1. 统一搜索入口

```bash
node scripts/search-runner.mjs "lightpanda browser"
```

常用参数：

```bash
node scripts/search-runner.mjs "agent-browser" --limit 5 --pages 3
node scripts/search-runner.mjs "openai api responses" --format markdown
```

### 2. 直接打开 agent-browser

```bash
node scripts/agent-browser-lightpanda.mjs --help
```

这个入口会固定注入：

- `AGENT_BROWSER_ENGINE=lightpanda`

适合需要人工或 Agent 继续做点击、快照、截图时使用。

## 平台说明

- macOS / Linux：`@lightpanda/browser` 可直接下载原生二进制
- Windows：官方 npm 包不提供原生二进制；请使用：
  - `WSL2` 内运行本技能，或
  - 自己提供 `LIGHTPANDA_EXECUTABLE_PATH`

详情见：

- [tooling.md](references/tooling.md)
- [platforms.md](references/platforms.md)
