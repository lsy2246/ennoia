import { useEffect, useMemo, useState } from "react";

import {
  getConfigHistory,
  getConfigSnapshot,
  putConfig,
  type ConfigChangeRecord,
  type SystemConfig,
} from "@ennoia/api-client";
import { useAuthStore } from "@/stores/auth";
import { useUiHelpers, useUiStore } from "@/stores/ui";

const TABS = [
  { key: "auth", label: "Auth" },
  { key: "rate_limit", label: "Rate Limit" },
  { key: "cors", label: "CORS" },
  { key: "timeout", label: "Timeout" },
  { key: "logging", label: "Logging" },
  { key: "body_limit", label: "Body Limit" },
] as const;

const TIME_ZONES = [
  { value: "", label: "Browser default" },
  { value: "UTC", label: "UTC" },
  { value: "Asia/Shanghai", label: "Asia/Shanghai" },
  { value: "America/New_York", label: "America/New_York" },
  { value: "Europe/Berlin", label: "Europe/Berlin" },
] as const;

type TabKey = (typeof TABS)[number]["key"];

export function SettingsPage() {
  const user = useAuthStore((s) => s.user);
  const runtime = useUiStore((s) => s.runtime);
  const locale = useUiStore((s) => s.locale);
  const themeId = useUiStore((s) => s.themeId);
  const timeZone = useUiStore((s) => s.timeZone);
  const dateStyle = useUiStore((s) => s.dateStyle);
  const savePreferences = useUiStore((s) => s.savePreferences);
  const { t, formatDateTime, availableThemes } = useUiHelpers();

  const [snapshot, setSnapshot] = useState<SystemConfig | null>(null);
  const [tab, setTab] = useState<TabKey>("auth");
  const [editorText, setEditorText] = useState("");
  const [history, setHistory] = useState<ConfigChangeRecord[]>([]);
  const [message, setMessage] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [prefLocale, setPrefLocale] = useState(locale);
  const [prefTheme, setPrefTheme] = useState(themeId);
  const [prefTimeZone, setPrefTimeZone] = useState(timeZone ?? "");
  const [prefDateStyle, setPrefDateStyle] = useState(dateStyle ?? "locale");

  const isAdmin = user?.role === "admin" || user?.role === "anonymous";
  const localeOptions = runtime?.ui_config.available_locales ?? ["zh-CN", "en-US"];
  const themeOptions = useMemo(
    () =>
      availableThemes.map((item) => ({
        value: item.id,
        label:
          item.id === "system"
            ? t("theme.system", "System")
            : item.id === "ennoia.midnight"
              ? t("theme.midnight", "Midnight")
              : item.id === "ennoia.paper"
                ? t("theme.paper", "Paper")
                : item.id === "observatory.daybreak"
                  ? t("theme.daybreak", "Daybreak")
                  : item.label,
      })),
    [availableThemes, t],
  );

  async function refresh() {
    if (!isAdmin) {
      return;
    }
    try {
      const s = await getConfigSnapshot();
      setSnapshot(s);
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    setPrefLocale(locale);
    setPrefTheme(themeId);
    setPrefTimeZone(timeZone ?? "");
    setPrefDateStyle(dateStyle ?? "locale");
  }, [dateStyle, locale, themeId, timeZone]);

  useEffect(() => {
    refresh();
  }, [isAdmin]);

  useEffect(() => {
    if (!snapshot) return;
    const payload = (snapshot as unknown as Record<string, unknown>)[tab];
    setEditorText(JSON.stringify(payload, null, 2));
    getConfigHistory(tab).then(setHistory).catch(() => setHistory([]));
  }, [tab, snapshot]);

  async function saveSystemConfig() {
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

  async function saveUiPreference() {
    setError(null);
    setMessage(null);
    try {
      await savePreferences({
        locale: prefLocale,
        theme_id: prefTheme,
        time_zone: prefTimeZone || null,
        date_style: prefDateStyle || null,
      });
      setMessage(t("settings.personal.saved", "Preferences saved."));
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <h1>{t("settings.title", "Settings")}</h1>

      <section className="settings-personal">
        <div className="settings-personal__intro">
          <h2>{t("settings.personal.title", "Personal UI preferences")}</h2>
          <p>{t("settings.personal.subtitle", "These choices are cached in the browser and synchronized to the current account in the background.")}</p>
        </div>
        <div className="form-row">
          <label>
            {t("settings.personal.locale", "Language")}
            <select value={prefLocale} onChange={(e) => setPrefLocale(e.target.value)}>
              {localeOptions.map((option) => (
                <option key={option} value={option}>
                  {option}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("settings.personal.theme", "Theme")}
            <select value={prefTheme} onChange={(e) => setPrefTheme(e.target.value)}>
              {themeOptions.map((option) => (
                <option key={option.value} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
        </div>
        <div className="form-row">
          <label>
            {t("settings.personal.time_zone", "Time zone")}
            <select value={prefTimeZone} onChange={(e) => setPrefTimeZone(e.target.value)}>
              {TIME_ZONES.map((option) => (
                <option key={option.value || "browser"} value={option.value}>
                  {option.label}
                </option>
              ))}
            </select>
          </label>
          <label>
            {t("settings.personal.date_style", "Date format")}
            <select value={prefDateStyle} onChange={(e) => setPrefDateStyle(e.target.value)}>
              <option value="locale">{t("date_style.locale", "Locale default")}</option>
              <option value="iso">{t("date_style.iso", "ISO 8601")}</option>
            </select>
          </label>
        </div>
        <div className="actions">
          <button onClick={saveUiPreference}>
            {t("settings.personal.save", "Save preferences")}
          </button>
        </div>
      </section>

      {isAdmin && (
        <>
          <section className="settings-personal settings-personal--admin">
            <div className="settings-personal__intro">
              <h2>{t("settings.system.title", "System runtime config")}</h2>
              <p>{t("settings.system.subtitle", "Administrators can edit middleware and auth configuration live.")}</p>
            </div>
          </section>

          <div className="tabs">
            {TABS.map((tItem) => (
              <button
                key={tItem.key}
                className={`tab ${tItem.key === tab ? "tab--active" : ""}`}
                onClick={() => setTab(tItem.key)}
              >
                {tItem.label}
              </button>
            ))}
          </div>

          <div className="settings-grid">
            <section>
              <h3>{TABS.find((item) => item.key === tab)?.label} payload</h3>
              <textarea
                className="json-editor"
                value={editorText}
                onChange={(e) => setEditorText(e.target.value)}
                spellCheck={false}
                rows={20}
              />
              <div className="actions">
                <button onClick={saveSystemConfig}>{t("settings.save_apply", "Save & apply")}</button>
                <button onClick={refresh} className="secondary">
                  {t("settings.reload", "Reload")}
                </button>
              </div>
            </section>

            <section>
              <h3>{t("settings.recent_changes", "Recent changes")}</h3>
              {history.length === 0 ? (
                <p className="muted">(no history)</p>
              ) : (
                <ul className="history-list">
                  {history.map((h) => (
                    <li key={h.id}>
                      <time>{formatDateTime(h.changed_at)}</time>
                      <span>by {h.changed_by ?? "unknown"}</span>
                    </li>
                  ))}
                </ul>
              )}
            </section>
          </div>
        </>
      )}

      {error && <div className="error">{error}</div>}
      {message && <div className="success">{message}</div>}

      <section className="settings-preview">
        <h3>Preview</h3>
        <p>
          Locale: <code>{prefLocale}</code> · Theme: <code>{prefTheme}</code>
        </p>
        <p>{formatDateTime(new Date())}</p>
      </section>
    </div>
  );
}
