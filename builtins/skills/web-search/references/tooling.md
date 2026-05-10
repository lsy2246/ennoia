# 工具分工

## agent-browser

负责浏览器操作层能力：

- 打开页面
- 快照
- 点击
- 截图
- 继续交互

本技能通过 `scripts/agent-browser-lightpanda.mjs` 固定把它绑到 `Lightpanda` 引擎。

## Lightpanda

负责页面抓取与轻量浏览器执行：

- 结果页抓取
- 页面 HTML 获取
- 页面正文提取的底层内容来源

## search-runner

这是本技能的统一搜索入口。

它会：

1. 用 Lightpanda 拉取搜索结果页
2. 解析候选结果
3. 继续抓取候选详情页
4. 产出统一 JSON / Markdown 结果

如果需要继续交互浏览，再交给 `agent-browser-lightpanda`。
