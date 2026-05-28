# Artifact Runner

`artifact-runner` 是独立扩展，只负责 Agent 生成产物的预览和运行入口。

模型可以输出 `ennoia.artifact_runner` JSON envelope。`workflow` 会把 `fallback` 保存为普通会话消息，并把产物内容写入 `artifact-runner.artifact` 扩展记录。

第一版支持：

- `html-artifact`：使用 sandbox iframe 预览 HTML，并提供源码。预览允许内联脚本实现 canvas、小游戏和页面交互，但不放开同源权限。聊天卡片提供紧凑预览，也可以打开扩展内大预览查看完整页面。
- `html-source`：默认展示 HTML 源码，并允许用户手动切换到预览，适合用户明确要求源代码的场景。
- `python-artifact`：展示 Python 源码，并通过本扩展的 `artifact.run_python` operation 手动运行，返回 stdout、stderr、退出码和耗时。

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

## HTML Source Envelope

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
