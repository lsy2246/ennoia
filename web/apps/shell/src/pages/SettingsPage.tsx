import { useEffect, useMemo, useState } from "react";

import { getConfig, getConfigHistory, listConfig, putConfig, saveRuntimeProfile } from "@ennoia/api-client";
import { applyTheme, readUiBootstrapCache, writeUiBootstrapCache } from "@ennoia/theme-runtime";
import { buildTimeZoneOptionGroups } from "@/lib/timeZones";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

const CONFIG_TABS = ["rate_limit", "cors", "timeout", "logging", "body_limit", "bootstrap"] as const;
type ConfigTab = (typeof CONFIG_TABS)[number];

export function SettingsPage() {
  const runtimeProfile = useRuntimeStore((state) => state.profile);
  const refreshRuntime = useRuntimeStore((state) => state.hydrate);
  const savePreferences = useUiStore((state) => state.savePreferences);
  const previewLocale = useUiStore((state) => state.previewLocale);
  const currentThemeId = useUiStore((state) => state.themeId);
  const { availableLocales, availableThemes, t } = useUiHelpers();
  const [displayName, setDisplayName] = useState(runtimeProfile?.display_name ?? "");
  const [locale, setLocale] = useState(runtimeProfile?.locale ?? availableLocales[0] ?? "zh-CN");
  const [themeId, setThemeId] = useState(currentThemeId);
  const [timeZone, setTimeZone] = useState(runtimeProfile?.time_zone ?? "Asia/Shanghai");
  const [activeTab, setActiveTab] = useState<ConfigTab>("rate_limit");
  const [configPayload, setConfigPayload] = useState("{}");
  const [history, setHistory] = useState<Array<{ changed_at: string; changed_by?: string | null }>>([]);
  const [message, setMessage] = useState<string | null>(null);

  const timeZoneGroups = useMemo(() => buildTimeZoneOptionGroups(t, true), [t]);

  useEffect(() => {
    if (runtimeProfile) {
      setDisplayName(runtimeProfile.display_name);
      setLocale(runtimeProfile.locale);
      setTimeZone(runtimeProfile.time_zone);
    }
  }, [runtimeProfile]);

  useEffect(() => {
    setThemeId((current) => {
      if (availableThemes.length === 0) {
        return current;
      }
      return availableThemes.some((item) => item.id === current) ? current : availableThemes[0].id;
    });
  }, [availableThemes]);

  useEffect(() => {
    void (async () => {
      const entries = await listConfig();
      const current = entries.find((item) => item.key === activeTab);
      if (current) {
        setConfigPayload(current.payload_json || "{}");
      }
      const nextHistory = await getConfigHistory(activeTab);
      setHistory(nextHistory.map((item) => ({ changed_at: item.changed_at, changed_by: item.changed_by })));
    })();
  }, [activeTab]);

  async function handleSaveProfile() {
    await saveRuntimeProfile({
      display_name: displayName,
      locale,
      time_zone: timeZone,
    });
    await savePreferences({
      locale,
      theme_id: themeId,
      time_zone: timeZone,
    });
    await refreshRuntime();
    setMessage(t("settings.personal.saved", "偏好已保存。"));
  }

  async function handleLocaleChange(nextLocale: string) {
    setLocale(nextLocale);
    await previewLocale(nextLocale);
  }

  function handleThemeChange(nextThemeId: string) {
    setThemeId(nextThemeId);
    applyTheme(nextThemeId);
    writeUiBootstrapCache({
      ...readUiBootstrapCache(),
      theme_id: nextThemeId,
      updated_at: new Date().toISOString(),
    });
  }

  async function handleSaveConfig() {
    const parsed = JSON.parse(configPayload);
    await putConfig(activeTab, parsed, "operator");
    const current = await getConfig(activeTab);
    setConfigPayload(current.payload_json || "{}");
    const nextHistory = await getConfigHistory(activeTab);
    setHistory(nextHistory.map((item) => ({ changed_at: item.changed_at, changed_by: item.changed_by })));
    setMessage(t("settings.runtime.saved", "{tab} 已保存并已实时应用。").replace("{tab}", activeTab));
  }

  return (
    <div className="page">
      <div className="settings-layout">
        <section className="surface-panel">
          <h1>{t("shell.nav.settings", "设置")}</h1>
          {message ? <div className="success">{message}</div> : null}
          <div className="form-stack">
            <label>
              {t("settings.profile.display_name", "显示名称")}
              <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} />
            </label>
            <label>
              {t("settings.profile.language", "语言")}
              <select value={locale} onChange={(event) => void handleLocaleChange(event.target.value)}>
                {availableLocales.map((item) => (
                  <option key={item} value={item}>
                    {item}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("settings.profile.theme", "主题")}
              <select value={themeId} onChange={(event) => handleThemeChange(event.target.value)}>
                {availableThemes.map((item) => (
                  <option key={item.id} value={item.id}>
                    {item.label}
                  </option>
                ))}
              </select>
            </label>
            <label>
              {t("settings.profile.time_zone", "时区")}
              <select value={timeZone} onChange={(event) => setTimeZone(event.target.value)}>
                {timeZoneGroups.map((group) => (
                  <optgroup key={group.label} label={group.label}>
                    {group.options.map((option) => (
                      <option key={`${group.label}:${option.value}`} value={option.value}>
                        {option.label}
                      </option>
                    ))}
                  </optgroup>
                ))}
              </select>
            </label>
            <p className="muted">
              {t(
                "settings.timezone.description",
                "只影响工作台里的时间显示，不影响调度、存储或后端时间。",
              )}
            </p>
            <button onClick={() => void handleSaveProfile()}>
              {t("settings.profile.save", "保存资料与偏好")}
            </button>
          </div>
        </section>

        <section className="surface-panel">
          <h2>{t("settings.runtime.title", "运行时配置")}</h2>
          <div className="tabs">
            {CONFIG_TABS.map((tab) => (
              <button
                key={tab}
                className={tab === activeTab ? "tab tab--active" : "tab"}
                onClick={() => setActiveTab(tab)}
              >
                {tab}
              </button>
            ))}
          </div>

          <label>
            {t("settings.runtime.payload", "{tab} 配置载荷").replace("{tab}", activeTab)}
            <textarea rows={14} value={configPayload} onChange={(event) => setConfigPayload(event.target.value)} />
          </label>
          <button onClick={() => void handleSaveConfig()}>
            {t("settings.runtime.save_apply", "保存并应用")}
          </button>

          <div className="stack-list stack-list--compact">
            {history.map((item, index) => (
              <div key={`${item.changed_at}:${index}`} className="execution-row">
                <strong>{item.changed_by ?? t("settings.runtime.unknown", "未知")}</strong>
                <span>{item.changed_at}</span>
              </div>
            ))}
          </div>
        </section>
      </div>
    </div>
  );
}
