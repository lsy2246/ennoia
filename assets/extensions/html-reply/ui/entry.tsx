import React, { useEffect, useState, type CSSProperties } from "react";
import { createRoot, type Root } from "react-dom/client";
import type {
  ExtensionConversationRecord,
  ExtensionUiModule,
  ExtensionUiRenderHelpers,
} from "@ennoia/ui-sdk";

import "./html-reply.css";

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

type HtmlReplyPayload = {
  html?: string;
  fallback?: string;
};

const MIN_MESSAGE_FRAME_HEIGHT = 64;
const MAX_MESSAGE_FRAME_HEIGHT = 420;
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
  return `<!doctype html><html><head>
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
  </head><body>${content}</body></html>`;
}

function HtmlReplyCard({ record }: { record: ExtensionConversationRecord }) {
  const payload = asRecordPayload<HtmlReplyPayload>(record.payload);
  const html = textValue(payload.html);
  const [frameHeight, setFrameHeight] = useState(MIN_MESSAGE_FRAME_HEIGHT);
  useEffect(() => {
    setFrameHeight(MIN_MESSAGE_FRAME_HEIGHT);
  }, [html]);

  if (!html.trim()) {
    return null;
  }

  function handleFrameLoad(event: React.SyntheticEvent<HTMLIFrameElement>) {
    const documentElement = event.currentTarget.contentDocument?.documentElement;
    const body = event.currentTarget.contentDocument?.body;
    const bodyHeight = Math.max(
      body?.scrollHeight ?? 0,
      body?.offsetHeight ?? 0,
      body?.getBoundingClientRect().height ?? 0,
    );
    const contentHeight = Math.max(
      bodyHeight,
      documentElement?.offsetHeight ?? 0,
      MIN_MESSAGE_FRAME_HEIGHT,
    );
    setFrameHeight(Math.min(contentHeight, MAX_MESSAGE_FRAME_HEIGHT));
  }

  return (
    <section className="html-reply-card html-reply-card--message">
      <iframe
        className="html-reply-frame html-reply-frame--message html-reply-frame--auto"
        sandbox="allow-same-origin"
        onLoad={handleFrameLoad}
        style={{ "--html-reply-frame-height": `${frameHeight}px` } as CSSProperties}
        title={record.title || "HTML 排版回复"}
        srcDoc={buildSandboxHtml(html)}
      />
    </section>
  );
}

function HtmlReplyPanel({ helpers }: { helpers: ExtensionUiRenderHelpers }) {
  const [enabled, setEnabled] = useState(false);
  const [status, setStatus] = useState("");

  useEffect(() => {
    const params = new URLSearchParams({
      extension_id: "html-reply",
      namespace: "html-reply/config",
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
        if (entry?.value && typeof entry.value === "object") {
          setEnabled(Boolean((entry.value as { enabled?: unknown }).enabled));
        }
      })
      .catch(() => setStatus("读取配置失败。"));
  }, [helpers.apiBaseUrl]);

  async function saveConfig() {
    const response = await fetch(`${helpers.apiBaseUrl}/api/extensions/state`, {
      method: "POST",
      headers: { "content-type": "application/json" },
      body: JSON.stringify({
        extension_id: "html-reply",
        namespace: "html-reply/config",
        scope_type: CONFIG_SCOPE_TYPE,
        scope_id: CONFIG_SCOPE_ID,
        key: CONFIG_KEY,
        value: {
          enabled,
          reply_mode: enabled ? "html-message" : null,
        },
      }),
    });
    setStatus(response.ok ? "已保存。" : "保存失败。");
  }

  return (
    <div className="html-reply-panel">
      <h2>{helpers.t("ext.html_reply.panel", "HTML 回复")}</h2>
      <p>控制普通回复是否允许使用 HTML 富排版。</p>
      <div className="html-reply-form">
        <label className="html-reply-check">
          <input type="checkbox" checked={enabled} onChange={(event) => setEnabled(event.target.checked)} />
          <span>允许 HTML 富排版回复</span>
        </label>
        <button type="button" onClick={() => void saveConfig()}>保存配置</button>
        {status ? <small>{status}</small> : null}
      </div>
    </div>
  );
}

const extensionUi: ExtensionUiModule = {
  panels: {
    "html-reply.panel": (container, context) =>
      renderIntoContainer(container, <HtmlReplyPanel helpers={context.helpers} />),
  },
  conversationRecords: {
    "html-reply.message": (container, context) =>
      renderIntoContainer(container, <HtmlReplyCard record={context.record} />),
  },
};

export default extensionUi;
