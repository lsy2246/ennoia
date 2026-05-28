# Artifact Runner 输出协议

当当前 Agent 或会话启用产物预览运行时，可以用 `ennoia.artifact_runner` JSON 作为最终回复。不要把 JSON 外再包 Markdown 代码块。

HTML 产物：

```json
{
  "kind": "ennoia.artifact_runner",
  "version": 1,
  "profile": "html-artifact",
  "placement": "artifact",
  "content_type": "text/html",
  "title": "页面预览",
  "fallback": "我生成了一个 HTML 页面，可以在下方预览。",
  "body": "<!doctype html><html>...</html>"
}
```

HTML 源码产物：

```json
{
  "kind": "ennoia.artifact_runner",
  "version": 1,
  "profile": "html-source",
  "placement": "artifact",
  "content_type": "text/html",
  "title": "index.html",
  "fallback": "这是 HTML 源代码。",
  "body": "<!doctype html><html>...</html>"
}
```

Python 产物：

```json
{
  "kind": "ennoia.artifact_runner",
  "version": 1,
  "profile": "python-artifact",
  "placement": "artifact",
  "content_type": "text/x-python",
  "title": "Python 示例",
  "fallback": "我生成了一个 Python 示例。",
  "body": "print('hello')"
}
```

要求：

- 用户说“预览 / 运行 / 画出来 / 展示页面”时使用 `html-artifact`。
- 用户明确说“HTML 源码 / 源代码 / 代码 / 保存成 html 文件源码”时使用 `html-source`，默认展示源码，并允许用户手动切换到预览。
- HTML 预览可以写内联 `<script>` 或事件监听来实现 canvas、小游戏、表单交互等产物行为；不要使用外链脚本。
- HTML 预览在 sandbox iframe 中运行，不依赖宿主页面上下文；需要持久化状态时优先使用页面内变量，`localStorage`/`sessionStorage` 只作为临时预览状态使用。
- Python 代码默认展示源码，用户可以在产物卡片中手动运行；运行结果由 `artifact-runner` 扩展返回 stdout、stderr 和退出码。
- 不要在普通回复正文里伪造 Python 运行结果；只有真正运行后才描述输出。
- 始终提供有意义的 `fallback`。
- 普通回复的 HTML 排版不要放在这里，交给 `html-reply`。
