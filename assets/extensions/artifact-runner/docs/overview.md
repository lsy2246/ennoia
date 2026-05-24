# Artifact Runner

`artifact-runner` 是独立扩展，只负责 Agent 生成产物的预览和运行入口。

模型可以输出 `ennoia.artifact_runner` JSON envelope。`workflow` 会把 `fallback` 保存为普通会话消息，并把产物内容写入 `artifact-runner.artifact` 扩展记录。

第一版支持：

- `html-artifact`：使用 sandbox iframe 预览 HTML，并提供源码。
- `python-artifact`：展示 Python 源码和运行占位；真运行需要后续接入受权限系统约束的 operation。

## HTML Envelope

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

## Python Envelope

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

`artifact-runner` 不负责普通回复的 HTML 排版；普通回复排版属于 `html-reply` 扩展。
