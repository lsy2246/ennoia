# HTML Reply 输出协议

当当前 Agent 或会话启用 HTML 回复排版时，可以用 `ennoia.html_reply` JSON 作为最终回复。不要把 JSON 外再包 Markdown 代码块。

```json
{
  "kind": "ennoia.html_reply",
  "version": 1,
  "profile": "html-message",
  "placement": "message",
  "content_type": "text/html",
  "body": "<section><h2>标题</h2><p>静态 HTML 内容。</p></section>"
}
```

要求：

- 只写静态 HTML 片段，不写 `<script>`、事件属性或外链脚本。
- HTML `body` 会作为同一条 `format = "html"` 的消息正文保存，不要再输出一份 Markdown 或纯文本正文。
- HTML 源码、完整 HTML 页面、Python 脚本或其他可预览产物不要放在这里，交给 `artifact-runner`。
