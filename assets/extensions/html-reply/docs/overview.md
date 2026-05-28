# HTML Reply

`html-reply` 是独立扩展，只负责 Agent 普通回复的 HTML 富排版展示。

模型可以输出 `ennoia.html_reply` JSON envelope。`workflow` 会把 `fallback` 保存为普通会话消息，并把 HTML 内容写入 `html-reply.message` 扩展记录。扩展禁用或渲染失败时，普通消息仍然可读。这个扩展只呈现排版结果，不展示源码、不运行脚本。

## Envelope

```json
{
  "kind": "ennoia.html_reply",
  "version": 1,
  "profile": "html-message",
  "placement": "message",
  "content_type": "text/html",
  "fallback": "普通文本摘要",
  "body": "<section>安全静态 HTML 片段</section>"
}
```

`html-reply` 不负责 HTML 源码、完整网页、Python 或其他产物预览运行；这些内容属于 `artifact-runner` 扩展。
