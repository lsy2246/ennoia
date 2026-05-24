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
    return content;
  }
  return `<!doctype html><html><head>${baseStyle}</head><body>${content}</body></html>`;
}

function HtmlPreviewBlock({ title, content }: { title: string; content: string }) {
  return (
    <section className="artifact-runner-card">
      <header className="artifact-runner-card__header">
        <strong>{title}</strong>
        <span>HTML</span>
      </header>
      <iframe
        className="artifact-runner-frame artifact-runner-frame--preview"
        sandbox="allow-scripts"
        title={title}
        srcDoc={buildSandboxHtml(content)}
      />
      <details className="artifact-runner-source">
        <summary>源码</summary>
        <pre>{content}</pre>
      </details>
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
      <p className="artifact-runner-note">当前版本只展示代码，不自动执行。</p>
    </section>
  );
}

function ArtifactCard({ record }: { record: ExtensionConversationRecord }) {
  const payload = asRecordPayload<ArtifactPayload>(record.payload);
  const type = textValue(payload.type);
  const title = textValue(payload.title) || record.title || "产物";
  const content = textValue(payload.content);
  if (!content.trim()) {
    return null;
  }

  if (type === "html-preview") {
    return <HtmlPreviewBlock title={title} content={content} />;
  }

  if (type === "python-run") {
    return <CodeBlock title={title} language="Python" content={content} />;
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
      <p>控制是否允许 HTML 产物预览和 Python 代码产物展示。</p>
      <div className="artifact-runner-form">
        <label className="artifact-runner-check">
          <input type="checkbox" checked={htmlPreview} onChange={(event) => setHtmlPreview(event.target.checked)} />
          <span>允许 HTML 预览产物</span>
        </label>
        <label className="artifact-runner-check">
          <input type="checkbox" checked={pythonRun} onChange={(event) => setPythonRun(event.target.checked)} />
          <span>允许 Python 代码产物展示</span>
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
      renderIntoContainer(container, <ArtifactCard record={context.record} />),
  },
};

export default extensionUi;
