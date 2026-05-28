# Markdown Renderer

`markdown-renderer` 是内置消息渲染扩展，负责把会话普通消息中的 Markdown 正文渲染成富文本。

它通过 `message_renderers` 注册 `markdown` 格式的挂载点。Web 主壳只负责选择渲染器、传入消息正文和上下文，并在扩展缺失或渲染失败时回退到纯文本展示。

这个扩展只处理消息正文的 Markdown 排版，不负责 HTML 回复排版、HTML/Python 产物预览或脚本运行；这些能力分别属于 `html-reply` 与 `artifact-runner`。
