import { useEffect, useMemo, useState } from "react";

import {
  getConfigHistory,
  getConfigSnapshot,
  putConfig,
  saveRuntimeProfile,
  type ConfigChangeRecord,
  type SystemConfig,
} from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

const TABS = [
  { key: "rate_limit", label: "Rate Limit" },
  { key: "cors", label: "CORS" },
  { key: "timeout", label: "Timeout" },
  { key: "logging", label: "Logging" },
  { key: "body_limit", label: "Body Limit" },
  { key: "bootstrap", label: "Bootstrap" },
] as const;

type TabKey = (typeof TABS)[number]["key"];

const TIME_ZONES = [
  { value: "", label: "Browser default" },
  { value: "UTC", label: "UTC" },
  { value: "Asia/Shanghai", label: "Asia/Shanghai" },
  { value: "America/New_York", label: "America/New_York" },
] as const;

export function SettingsPage() {
  const profile = useRuntimeStore((state) => state.profile);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const runtime = useUiStore((state) => state.runtime);
  const locale = useUiStore((state) => state.locale);
  const themeId = useUiStore((state) => state.themeId);
  const timeZone = useUiStore((state) => state.timeZone);
  const dateStyle = useUiStore((state) => state.dateStyle);
  const savePreferences = useUiStore((state) => state.savePreferences);
  const { t, formatDateTime, availableThemes } = useUiHelpers();

  const [snapshot, setSnapshot] = useState<SystemConfig | null>(null);
  const [tab, setTab] = useState<TabKey>("rate_limit");
  const [editorText, setEditorText] = useState("");
  const [history, setHistory] = useState<ConfigChangeRecord[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [displayName, setDisplayName] = useState(profile?.display_name ?? "Operator");
  const [prefLocale, setPrefLocale] = useState(locale);
  const [prefTheme, setPrefTheme] = useState(themeId);
  const [prefTimeZone, setPrefTimeZone] = useState(timeZone ?? "");
  const [prefDateStyle, setPrefDateStyle] = useState(dateStyle ?? "locale");

  const localeOptions = runtime?.ui_config.available_locales ?? ["zh-CN", "en-US"];
  const themeOptions = useMemo(
    () =>
      availableThemes.map((item) => ({
        value: item.id,
        label: item.label,
      })),
    [availableThemes],
  );

  useEffect(() => {
    setDisplayName(profile?.display_name ?? "Operator");
  }, [profile]);

  useEffect(() => {
    setPrefLocale(locale);
    setPrefTheme(themeId);
    setPrefTimeZone(timeZone ?? "");
    setPrefDateStyle(dateStyle ?? "locale");
  }, [dateStyle, locale, themeId, timeZone]);

  async function refreshConfig() {
    try {
      const nextSnapshot = await getConfigSnapshot();
      setSnapshot(nextSnapshot);
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refreshConfig();
  }, []);

  useEffect(() => {
    if (!snapshot) return;
    const payload = (snapshot as unknown as Record<string, unknown>)[tab];
    setEditorText(JSON.stringify(payload, null, 2));
    getConfigHistory(tab).then(setHistory).catch(() => setHistory([]));
  }, [snapshot, tab]);

  async function saveSystemConfig() {
    setError(null);
    setMessage(null);
    try {
      const parsed = JSON.parse(editorText);
      await putConfig(tab, parsed, "shell");
      await refreshConfig();
      setHistory(await getConfigHistory(tab));
      setMessage(`Saved ${tab} and applied it live.`);
    } catch (err) {
      setError(String(err));
    }
  }

  async function saveProfileAndPreferences() {
    setError(null);
    setMessage(null);
    try {
      await Promise.all([
        saveRuntimeProfile({
          display_name: displayName,
          locale: prefLocale,
          time_zone: prefTimeZone || null,
        }),
        savePreferences({
          locale: prefLocale,
          theme_id: prefTheme,
          time_zone: prefTimeZone || null,
          date_style: prefDateStyle || null,
        }),
      ]);
      await hydrateRuntime();
      setMessage(t("settings.personal.saved", "Preferences saved."));
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.settings.title", "Settings")}
        description={t(
          "shell.page.settings.description",
          "Manage workspace preferences, theme, locale and runtime configuration.",
        )}
        meta={[
          profile?.display_name ?? "Operator",
          runtime?.registry.themes.length ? `${runtime.registry.themes.length} themes` : "builtin themes",
        ]}
      />

      <section className="settings-personal">
        <div className="settings-personal__intro">
          <h2>Workspace profile</h2>
          <p>浏览器会先读取本地缓存，再与当前实例的偏好同步，避免每次刷新都依赖远端状态。</p>
        </div>
        <div className="form-row">
          <label>
            Display name
            <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} />
          </label>
          <label>
            Language
            <select value={prefLocale} onChange={(event) => setPrefLocale(event.target.value)}>
              {localeOptions.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="form-row">
          <label>
            Theme
            <select value={prefTheme} onChange={(event) => setPrefTheme(event.target.value)}>
              {themeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            Time zone
            <select value={prefTimeZone} onChange={(event) => setPrefTimeZone(event.target.value)}>
              {TIME_ZONES.map((option) => (
                <option key={option.label} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="form-row">
          <label>
            Date format
            <select value={prefDateStyle} onChange={(event) => setPrefDateStyle(event.target.value)}>
              <option value="locale">Locale default</option>
              <option value="iso">ISO 8601</option>
            </select>
          </label>
        </div>
        <div className="actions">
          <button onClick={saveProfileAndPreferences}>Save profile & preferences</button>
        </div>
      </section>

      <section className="settings-personal settings-personal--runtime">
        <div className="settings-personal__intro">
          <h2>Runtime config</h2>
          <p>单用户实例依然保留热更新配置能力，方便我们在开发期快速调试限流、超时和日志。</p>
        </div>
      </section>

      <div className="tabs">
        {TABS.map((item) => (
          <button
            key={item.key}
            className={`tab ${item.key === tab ? "tab--active" : ""}`}
            onClick={() => setTab(item.key)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div className="settings-grid">
        <section>
          <h3>{TABS.find((item) => item.key === tab)?.label} payload</h3>
          <textarea
            className="json-editor"
            value={editorText}
            onChange={(event) => setEditorText(event.target.value)}
            spellCheck={false}
            rows={20}
          />
          <div className="actions">
            <button onClick={saveSystemConfig}>Save & apply</button>
            <button onClick={refreshConfig} className="secondary">
              Reload
            </button>
          </div>
        </section>

        <section>
          <h3>Recent changes</h3>
          {history.length === 0 ? (
            <p className="muted">(no history)</p>
          ) : (
            <ul className="history-list">
              {history.map((item) => (
                <li key={item.id}>
                  <time>{formatDateTime(item.changed_at)}</time>
                  <span>by {item.changed_by ?? "unknown"}</span>
                </li>
              ))}
            </ul>
          )}
        </section>
      </div>

      {error && <div className="error">{error}</div>}
      {message && <div className="success">{message}</div>}
    </div>
  );
}
