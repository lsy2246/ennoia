import { useEffect, useState } from "react";

import {
  getConfigHistory,
  getConfigSnapshot,
  putConfig,
  type ConfigChangeRecord,
  type SystemConfig,
} from "@ennoia/api-client";

const TABS = [
  { key: "auth", label: "Auth" },
  { key: "rate_limit", label: "Rate Limit" },
  { key: "cors", label: "CORS" },
  { key: "timeout", label: "Timeout" },
  { key: "logging", label: "Logging" },
  { key: "body_limit", label: "Body Limit" },
] as const;

type TabKey = (typeof TABS)[number]["key"];

export function SettingsPage() {
  const [snapshot, setSnapshot] = useState<SystemConfig | null>(null);
  const [tab, setTab] = useState<TabKey>("auth");
  const [editorText, setEditorText] = useState("");
  const [history, setHistory] = useState<ConfigChangeRecord[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    try {
      const s = await getConfigSnapshot();
      setSnapshot(s);
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  useEffect(() => {
    if (!snapshot) return;
    const payload = (snapshot as unknown as Record<string, unknown>)[tab];
    setEditorText(JSON.stringify(payload, null, 2));
    getConfigHistory(tab).then(setHistory).catch(() => setHistory([]));
  }, [tab, snapshot]);

  async function save() {
    setError(null);
    setMessage(null);
    try {
      const parsed = JSON.parse(editorText);
      await putConfig(tab, parsed, "ui");
      setMessage(`Saved ${tab} (applied live).`);
      await refresh();
      const h = await getConfigHistory(tab);
      setHistory(h);
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <h1>Settings</h1>
      <div className="tabs">
        {TABS.map((t) => (
          <button
            key={t.key}
            className={`tab ${t.key === tab ? "tab--active" : ""}`}
            onClick={() => setTab(t.key)}
          >
            {t.label}
          </button>
        ))}
      </div>

      <div className="settings-grid">
        <section>
          <h3>{TABS.find((t) => t.key === tab)?.label} payload</h3>
          <textarea
            className="json-editor"
            value={editorText}
            onChange={(e) => setEditorText(e.target.value)}
            spellCheck={false}
            rows={20}
          />
          <div className="actions">
            <button onClick={save}>Save & apply</button>
            <button onClick={refresh} className="secondary">
              Reload
            </button>
          </div>
          {error && <div className="error">{error}</div>}
          {message && <div className="success">{message}</div>}
        </section>

        <section>
          <h3>Recent changes</h3>
          {history.length === 0 ? (
            <p className="muted">(no history)</p>
          ) : (
            <ul className="history-list">
              {history.map((h) => (
                <li key={h.id}>
                  <time>{new Date(h.changed_at).toLocaleString()}</time>
                  <span>by {h.changed_by ?? "unknown"}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>
    </div>
  );
}
