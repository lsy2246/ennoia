# Web Search

这个技能不是空文档，而是一个带本地工具的真实技能包。

- 搜索与开页工具：`agent-browser`
- 页面抓取引擎：`Lightpanda`
- 对外动作入口：
  - `search` -> `scripts/search-runner.mjs`
- 内部辅助脚本：
  - `scripts/agent-browser-lightpanda.mjs`

## 用途

适合这些任务：

- 搜网页资料
- 找官网、文档、公告页
- 继续抓取候选结果页并提取正文
- 从页面里拿标题、摘要、时间、链接和主内容

它不负责任务编排，也不负责调度；只负责“接到搜索任务后怎么搜、怎么开页、怎么提取”。

## 调用约定

系统侧现在只关心：

- `skill_id = "web-search"`
- `action_id = "search"`
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

当 `pages > 0` 时，这个入口会继续抓取候选页，并返回提取后的正文、标题、摘要、时间和链接。

## 内部辅助脚本

### agent-browser-lightpanda

这个脚本仍然保留，但它现在是 skill 内部的低层浏览器助手，不再作为对外 action 单独暴露。

适合：

- 需要继续手动或 Agent 交互浏览
- 需要点击、快照、截图、继续探索页面
- 调试 `agent-browser + Lightpanda` 本身的行为

## 本地依赖

如果你要在本地直接调试这些脚本，请自行在技能目录安装依赖并准备运行环境。

这个 skill 不再向宿主声明安装或检测流程；宿主只负责发现动作和保存配置。

## 命令行入口

### 1. 对外统一搜索入口

```bash
node scripts/search-runner.mjs "lightpanda browser"
```

常用参数：

```bash
node scripts/search-runner.mjs "agent-browser" --limit 5 --pages 3
node scripts/search-runner.mjs "openai api responses" --format markdown
```

### 2. 内部浏览器辅助脚本

```bash
node scripts/agent-browser-lightpanda.mjs --help
```

这个脚本会固定注入：

- `AGENT_BROWSER_ENGINE=lightpanda`

适合需要人工或 Agent 继续做点击、快照、截图时使用，但它不再代表 skill 的独立 action。

## 平台说明

- macOS / Linux：`@lightpanda/browser` 通常可以直接提供原生二进制
- Windows：仍需自行准备可用的 Lightpanda 运行时；常见做法是：
  - 在 `WSL2` 内运行本技能，或
  - 自己提供 `LIGHTPANDA_EXECUTABLE_PATH`
  - 或在技能配置里填写 `lightpanda_executable_path`

详情见：

- [tooling.md](references/tooling.md)
- [platforms.md](references/platforms.md)
