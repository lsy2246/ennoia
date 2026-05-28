import React, { useEffect, useState } from "react";
import { createRoot, type Root } from "react-dom/client";
import type {
  ExtensionConversationRecord,
  ExtensionUiModule,
  ExtensionUiRenderHelpers,
} from "@ennoia/ui-sdk";

import "./artifact-runner.css";

const roots = new WeakMap<HTMLElement, Root>();

function renderIntoContainer(container: HTMLElement, node: React.ReactNode) {
  let root = roots.get(container);
  if (!root) {
    root = createRoot(container);
    roots.set(container, root);
  }
  root.render(<React.StrictMode>{node}</React.StrictMode>);
  return {
    unmount() {
      const current = roots.get(container);
      current?.unmount();
      roots.delete(container);
    },
  };
}

type ArtifactPayload = {
  type?: string;
  title?: string;
  mime_type?: string;
  content?: string;
  agent_id?: string;
};

type PythonRunResult = {
  ok?: boolean;
  command?: string;
  args?: string[];
  cwd?: string;
  exit_code?: number | null;
  stdout?: string;
  stderr?: string;
  duration_ms?: number;
};

const CONFIG_SCOPE_TYPE = "extension";
const CONFIG_SCOPE_ID = "default";
const CONFIG_KEY = "output";

function asRecordPayload<T extends Record<string, unknown>>(value: unknown): T {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return {} as T;
  }
  return value as T;
}

function textValue(value: unknown) {
  return typeof value === "string" ? value : "";
}

const PREVIEW_RUNTIME = `
(function artifactRunnerPreviewRuntime() {
  function describeError(error) {
    if (!error) return "未知脚本错误";
    if (typeof error === "string") return error;
    if (error.message) return String(error.message);
    return String(error);
  }

  function showPreviewError(error) {
    var message = describeError(error);
    function render() {
      var target = document.body || document.documentElement;
      if (!target) return;
      var node = document.getElementById("artifact-runner-preview-error");
      if (!node) {
        node = document.createElement("div");
        node.id = "artifact-runner-preview-error";
        node.className = "artifact-runner-preview-error";
        node.setAttribute("role", "alert");
        node.style.cssText = [
          "position:fixed",
          "left:12px",
          "right:12px",
          "bottom:12px",
          "z-index:2147483647",
          "border:1px solid rgba(185,28,28,.35)",
          "border-radius:8px",
          "padding:10px 12px",
          "color:#991b1b",
          "background:rgba(254,242,242,.96)",
          "font:13px/1.5 system-ui,-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif",
          "box-shadow:0 12px 32px rgba(15,23,42,.18)"
        ].join(";");
        target.appendChild(node);
      }
      node.textContent = "HTML 预览脚本错误：" + message;
    }
    if (document.body || document.documentElement) {
      render();
    } else {
      document.addEventListener("DOMContentLoaded", render, { once: true });
    }
  }

  function createStorageShim() {
    var store = new Map();
    return {
      get length() {
        return store.size;
      },
      key: function key(index) {
        return Array.from(store.keys())[Number(index)] || null;
      },
      getItem: function getItem(key) {
        key = String(key);
        return store.has(key) ? store.get(key) : null;
      },
      setItem: function setItem(key, value) {
        store.set(String(key), String(value));
      },
      removeItem: function removeItem(key) {
        store.delete(String(key));
      },
      clear: function clear() {
        store.clear();
      }
    };
  }

  function installStorageShim(name) {
    try {
      var storage = window[name];
      var testKey = "__artifact_runner_storage_probe__";
      storage.setItem(testKey, testKey);
      storage.removeItem(testKey);
      return;
    } catch (_) {
      try {
        Object.defineProperty(window, name, {
          value: createStorageShim(),
          configurable: true
        });
      } catch (error) {
        showPreviewError("无法安装 " + name + " 兼容层：" + describeError(error));
      }
    }
  }

  installStorageShim("localStorage");
  installStorageShim("sessionStorage");

  window.addEventListener("error", function (event) {
    showPreviewError(event.error || event.message);
  });
  window.addEventListener("unhandledrejection", function (event) {
    showPreviewError(event.reason);
  });
})();
`;

function previewRuntimeScript() {
  return `<script>${PREVIEW_RUNTIME}</script>`;
}

function injectPreviewRuntime(content: string) {
  const runtime = previewRuntimeScript();
  if (/<head[\s>]/i.test(content)) {
    return content.replace(/<head(\s[^>]*)?>/i, (match) => `${match}${runtime}`);
  }
  if (/<body[\s>]/i.test(content)) {
    return content.replace(/<body(\s[^>]*)?>/i, `<head>${runtime}</head>$&`);
  }
  if (/<html[\s>]/i.test(content)) {
    return content.replace(/<html(\s[^>]*)?>/i, `$&<head>${runtime}</head>`);
  }
  return `${runtime}${content}`;
}

function buildSandboxHtml(content: string) {
  const baseStyle = `
    <style>
      :root { color-scheme: light dark; }
      body {
        margin: 0;
        padding: 16px;
        font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
        color: #18202f;
        background: #ffffff;
      }
      * { box-sizing: border-box; max-width: 100%; }
      img, svg, video, canvas { max-width: 100%; height: auto; }
      table { border-collapse: collapse; width: 100%; }
      th, td { border: 1px solid #d6dce8; padding: 8px; text-align: left; }
      pre, code { white-space: pre-wrap; word-break: break-word; }
    </style>
  `;
  if (/<html[\s>]/i.test(content)) {
    return injectPreviewRuntime(content);
  }
  return `<!doctype html><html><head>${baseStyle}${previewRuntimeScript()}</head><body>${content}</body></html>`;
}

type HtmlArtifactMode = "preview" | "source";

function HtmlArtifactBlock({
  title,
  content,
  initialMode,
}: {
  title: string;
  content: string;
  initialMode: HtmlArtifactMode;
}) {
  const [mode, setMode] = useState<HtmlArtifactMode>(initialMode);
  const [largePreviewOpen, setLargePreviewOpen] = useState(false);
  const [previewRevision, setPreviewRevision] = useState(0);
  const isPreview = mode === "preview";
  const previewDocument = buildSandboxHtml(content);
  return (
    <section className="artifact-runner-card artifact-runner-card--html">
      <header className="artifact-runner-card__header">
        <strong>{title}</strong>
        <span>HTML</span>
      </header>
      <div className="artifact-runner-tabs" role="tablist" aria-label="HTML 产物视图">
        <button
          type="button"
          role="tab"
          aria-selected={isPreview}
          onClick={() => setMode("preview")}
        >
          预览
        </button>
        <button
          type="button"
          role="tab"
          aria-selected={!isPreview}
          onClick={() => setMode("source")}
        >
          源码
        </button>
        <button
          className="artifact-runner-tabs__spacer-action"
          type="button"
          onClick={() => {
            setMode("preview");
            setLargePreviewOpen(true);
          }}
        >
          打开大预览
        </button>
      </div>
      <div className="artifact-runner-viewport">
        {isPreview ? (
          <iframe
            className="artifact-runner-frame artifact-runner-frame--preview"
            sandbox="allow-scripts"
            title={title}
            srcDoc={previewDocument}
          />
        ) : (
          <pre className="artifact-runner-code artifact-runner-source-scroll"><code>{content}</code></pre>
        )}
      </div>
      {largePreviewOpen ? (
        <div
          className="artifact-runner-large-preview"
          role="dialog"
          aria-modal="true"
          aria-label={`${title} 大预览`}
        >
          <div className="artifact-runner-large-preview__shell">
            <header className="artifact-runner-large-preview__toolbar">
              <strong>{title}</strong>
              <div className="artifact-runner-large-preview__actions">
                <button type="button" onClick={() => setPreviewRevision((value) => value + 1)}>
                  刷新
                </button>
                <button type="button" onClick={() => setLargePreviewOpen(false)}>
                  关闭大预览
                </button>
              </div>
            </header>
            <iframe
              key={previewRevision}
              className="artifact-runner-large-preview__frame"
              sandbox="allow-scripts"
              title={`${title} 大预览`}
              srcDoc={previewDocument}
            />
          </div>
        </div>
      ) : null}
    </section>
  );
}

function CodeBlock({ title, language, content }: { title: string; language: string; content: string }) {
  return (
    <section className="artifact-runner-card">
      <header className="artifact-runner-card__header">
        <strong>{title}</strong>
        <span>{language}</span>
      </header>
      <pre className="artifact-runner-code"><code>{content}</code></pre>
      <p className="artifact-runner-note">这种产物当前只提供源码查看。</p>
    </section>
  );
}

function resultText(value: unknown) {
  return typeof value === "string" ? value : "";
}

function resultNumber(value: unknown) {
  return typeof value === "number" && Number.isFinite(value) ? value : undefined;
}

function PythonRunBlock({
  title,
  content,
  record,
  helpers,
}: {
  title: string;
  content: string;
  record: ExtensionConversationRecord;
  helpers: ExtensionUiRenderHelpers;
}) {
  const payload = asRecordPayload<ArtifactPayload>(record.payload);
  const [status, setStatus] = useState<"idle" | "running" | "succeeded" | "failed">("idle");
  const [result, setResult] = useState<PythonRunResult | null>(null);
  const [error, setError] = useState("");
  const exitCode = resultNumber(result?.exit_code);
  const duration = resultNumber(result?.duration_ms);
  const stdout = resultText(result?.stdout);
  const stderr = resultText(result?.stderr);

  async function runPython() {
    setStatus("running");
    setError("");
    setResult(null);
    try {
      const response = await fetch(`${helpers.apiBaseUrl}/api/extensions/artifact-runner/rpc/artifact.run_python`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({
          params: {
            code: content,
            record_id: record.id,
            conversation_id: record.scope_id,
            agent_id: textValue(payload.agent_id),
          },
          context: {
            source: "artifact-runner.ui",
            record_id: record.id,
          },
        }),
      });
      const envelope = await response.json();
      if (!response.ok || envelope?.ok === false) {
        const message = envelope?.error?.message || `运行失败：HTTP ${response.status}`;
        throw new Error(message);
      }
      const data = (envelope?.data ?? {}) as PythonRunResult;
      setResult(data);
      setStatus(data.ok === false ? "failed" : "succeeded");
    } catch (runError) {
      setStatus("failed");
      setError(String(runError));
    }
  }

  return (
    <section className="artifact-runner-card">
      <header className="artifact-runner-card__header">
        <strong>{title}</strong>
        <span>Python</span>
      </header>
      <pre className="artifact-runner-code"><code>{content}</code></pre>
      <div className="artifact-runner-runbar">
        <button type="button" onClick={() => void runPython()} disabled={status === "running"}>
          {status === "running" ? "运行中" : "运行"}
        </button>
        <span>
          {status === "idle"
            ? "等待运行"
            : status === "running"
              ? "正在执行"
              : status === "succeeded"
                ? "运行完成"
                : "运行失败"}
        </span>
        {typeof exitCode === "number" ? <span>退出码 {exitCode}</span> : null}
        {typeof duration === "number" ? <span>{duration} ms</span> : null}
      </div>
      {result || error ? (
        <div className="artifact-runner-output">
          {error ? (
            <div className="artifact-runner-output__error">{error}</div>
          ) : null}
          <section>
            <h3>stdout</h3>
            <pre>{stdout || "无输出"}</pre>
          </section>
          <section>
            <h3>stderr</h3>
            <pre>{stderr || "无输出"}</pre>
          </section>
        </div>
      ) : null}
    </section>
  );
}

function ArtifactCard({ record, helpers }: { record: ExtensionConversationRecord; helpers: ExtensionUiRenderHelpers }) {
  const payload = asRecordPayload<ArtifactPayload>(record.payload);
  const type = textValue(payload.type);
  const title = textValue(payload.title) || record.title || "产物";
  const content = textValue(payload.content);
  if (!content.trim()) {
    return null;
  }

  if (type === "html-preview") {
    return <HtmlArtifactBlock title={title} content={content} initialMode="preview" />;
  }

  if (type === "html-source") {
    return <HtmlArtifactBlock title={title} content={content} initialMode="source" />;
  }

  if (type === "python-run") {
    return <PythonRunBlock title={title} content={content} record={record} helpers={helpers} />;
  }

  return <CodeBlock title={title} language={type || "text"} content={content} />;
}

function ArtifactRunnerPanel({ helpers }: { helpers: ExtensionUiRenderHelpers }) {
  const [htmlPreview, setHtmlPreview] = useState(false);
  const [pythonRun, setPythonRun] = useState(false);
  const [status, setStatus] = useState("");

  useEffect(() => {
    const params = new URLSearchParams({
      extension_id: "artifact-runner",
      namespace: "artifact-runner/config",
      scope_type: CONFIG_SCOPE_TYPE,
      scope_id: CONFIG_SCOPE_ID,
      key: CONFIG_KEY,
    });
    fetch(`${helpers.apiBaseUrl}/api/extensions/state/item?${params.toString()}`)
      .then((response) => {
        if (response.status === 404) {
          return null;
        }
        if (!response.ok) {
          throw new Error("load failed");
        }
        return response.json();
      })
      .then((entry) => {
        const value = entry?.value;
        if (!value || typeof value !== "object") {
          return;
        }
        const config = value as {
          html_artifact_enabled?: unknown;
          python_artifact_enabled?: unknown;
          artifacts?: unknown;
        };
        if (Array.isArray(config.artifacts)) {
          setHtmlPreview(config.artifacts.includes("html-artifact"));
          setPythonRun(config.artifacts.includes("python-artifact"));
          return;
        }
        setHtmlPreview(Boolean(config.html_artifact_enabled));
        setPythonRun(Boolean(config.python_artifact_enabled));
      })
      .catch(() => setStatus("读取配置失败。"));
  }, [helpers.apiBaseUrl]);

  async function saveConfig() {
    const artifacts = [
      htmlPreview ? "html-artifact" : "",
      pythonRun ? "python-artifact" : "",
    ].filter(Boolean);
    const response = await fetch(`${helpers.apiBaseUrl}/api/extensions/state`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        extension_id: "artifact-runner",
        namespace: "artifact-runner/config",
        scope_type: CONFIG_SCOPE_TYPE,
        scope_id: CONFIG_SCOPE_ID,
        key: CONFIG_KEY,
        value: {
          enabled: artifacts.length > 0,
          html_artifact_enabled: htmlPreview,
          python_artifact_enabled: pythonRun,
          artifacts,
        },
      }),
    });
    setStatus(response.ok ? "已保存。" : "保存失败。");
  }

  return (
    <div className="artifact-runner-panel">
      <h2>{helpers.t("ext.artifact_runner.panel", "产物运行")}</h2>
      <p>控制是否允许 HTML 产物预览和 Python 代码产物运行。</p>
      <div className="artifact-runner-form">
        <label className="artifact-runner-check">
          <input type="checkbox" checked={htmlPreview} onChange={(event) => setHtmlPreview(event.target.checked)} />
          <span>允许 HTML 预览产物</span>
        </label>
        <label className="artifact-runner-check">
          <input type="checkbox" checked={pythonRun} onChange={(event) => setPythonRun(event.target.checked)} />
          <span>允许 Python 代码产物运行</span>
        </label>
        <button type="button" onClick={() => void saveConfig()}>保存配置</button>
        {status ? <small>{status}</small> : null}
      </div>
    </div>
  );
}

const extensionUi: ExtensionUiModule = {
  panels: {
    "artifact-runner.panel": (container, context) =>
      renderIntoContainer(container, <ArtifactRunnerPanel helpers={context.helpers} />),
  },
  conversationRecords: {
    "artifact-runner.artifact": (container, context) =>
      renderIntoContainer(container, <ArtifactCard record={context.record} helpers={context.helpers} />),
  },
};

export default extensionUi;
