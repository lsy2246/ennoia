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

- HTML 预览默认不写 `<script>`、事件属性或外链脚本。
- Python 代码当前只展示，不自动执行。
- 始终提供有意义的 `fallback`。
- 普通回复的 HTML 排版不要放在这里，交给 `html-reply`。
