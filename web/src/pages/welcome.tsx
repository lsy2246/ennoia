import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState, type FormEvent } from "react";

import { bootstrapSetup } from "@ennoia/api-client";
import { applyTheme, readUiBootstrapCache, writeUiBootstrapCache } from "@ennoia/theme-runtime";
import { buildTimeZoneOptionGroups, getBrowserTimeZone } from "@/lib/timeZones";
import { normalizeLocaleSelection } from "@/lib/uiCapabilities";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

export function Welcome() {
  const navigate = useNavigate();
  const bootstrap = useRuntimeStore((state) => state.bootstrap);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const hydrateUi = useUiStore((state) => state.hydrate);
  const previewLocale = useUiStore((state) => state.previewLocale);
  const { availableLocales, availableThemes, t } = useUiHelpers();

  const [displayName, setDisplayName] = useState(t("settings.profile.default_name", "Operator"));
  const timeZoneGroups = useMemo(() => buildTimeZoneOptionGroups(t, false), [t]);
  const [locale, setLocale] = useState(
    normalizeLocaleSelection(
      typeof navigator !== "undefined" ? navigator.language : "zh-CN",
      availableLocales,
      "zh-CN",
    ),
  );
  const [timeZone, setTimeZone] = useState(getBrowserTimeZone);
  const [themeId, setThemeId] = useState("system");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (bootstrap?.is_initialized) {
      navigate({ to: "/conversations" });
    }
  }, [bootstrap, navigate]);

  useEffect(() => {
    setLocale((current) => normalizeLocaleSelection(current, availableLocales, "zh-CN"));
  }, [availableLocales]);

  function previewTheme(nextThemeId: string) {
    setThemeId(nextThemeId);
    applyTheme(nextThemeId);
    writeUiBootstrapCache({
      ...readUiBootstrapCache(),
      theme_id: nextThemeId,
      updated_at: new Date().toISOString(),
    });
  }

  async function handleLocalePreview(nextLocale: string) {
    setLocale(nextLocale);
    await previewLocale(nextLocale);
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setError(null);
    setBusy(true);
    try {
      await bootstrapSetup({
        display_name: displayName,
        locale,
        time_zone: timeZone,
        theme_id: themeId,
      });
      await Promise.all([hydrateRuntime(), hydrateUi()]);
      navigate({ to: "/conversations" });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page page--centered onboarding-page">
      <section className="onboarding-hero">
        <span className="onboarding-hero__eyebrow">
          {t("settings.bootstrap.hero_eyebrow", "首次启动引导")}
        </span>
        <h1>{t("settings.bootstrap.hero_title", "先确定这一套工作台的基础偏好，再开始协作。")}</h1>
        <p className="onboarding-hero__lead">
          {t(
            "settings.bootstrap.hero_lead",
            "Ennoia 现在是单操作者、多 Agent 的本地实例。这里不需要账号，初始化完成后会直接进入会话。",
          )}
        </p>
      </section>

      <form onSubmit={handleSubmit} className="setup-card setup-card--wide onboarding-card">
        <div className="onboarding-card__header">
          <h2>{t("settings.bootstrap.title", "工作台初始化")}</h2>
          <p>{t("settings.bootstrap.description", "这些信息会写入当前实例，并作为浏览器缓存与服务端偏好的初始值。")}</p>
        </div>

        <div className="form-stack">
          <div className="form-row">
            <label>
              {t("settings.bootstrap.operator_name", "操作者名称")}
              <input value={displayName} onChange={(event) => setDisplayName(event.target.value)} />
            </label>
            <label>
              {t("settings.bootstrap.language", "语言")}
              <select value={locale} onChange={(event) => void handleLocalePreview(event.target.value)}>
                {availableLocales.map((option) => (
                  <option key={option} value={option}>
                    {option}
                  </option>
                ))}
              </select>
            </label>
          </div>

          <div className="form-row">
            <label>
              {t("settings.bootstrap.time_zone", "时区")}
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
            <label>
              {t("settings.bootstrap.theme", "主题")}
              <select value={themeId} onChange={(event) => previewTheme(event.target.value)}>
                {availableThemes.map((theme) => (
                  <option key={theme.id} value={theme.id}>
                    {theme.label}
                  </option>
                ))}
              </select>
            </label>
          </div>
        </div>

        {error ? <div className="setup-card__error">{error}</div> : null}

        <div className="onboarding-actions">
          <button type="submit" disabled={busy}>
            {busy
              ? t("settings.bootstrap.submitting", "正在初始化…")
              : t("settings.bootstrap.submit", "完成初始化并进入会话")}
          </button>
        </div>
      </form>
    </div>
  );
}

