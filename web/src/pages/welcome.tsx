import { useNavigate } from "@tanstack/react-router";
import { useEffect, useMemo, useState, type FormEvent } from "react";

import { bootstrapSetup } from "@ennoia/api-client";
import { applyTheme, readUiBootstrapCache, writeUiBootstrapCache } from "@ennoia/theme-runtime";
import { StatusNotice } from "@/components/StatusNotice";
import { buildTimeZoneOptionGroups, getBrowserTimeZone } from "@/lib/timeZones";
import { normalizeLocaleSelection } from "@/lib/uiCapabilities";
import {
  resolveDefaultDisplayName,
  resolveDefaultLocale,
  resolveDefaultTheme,
} from "@/lib/uiDefaults";
import { Select } from "@/components/Select";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

export function Welcome() {
  const navigate = useNavigate();
  const bootstrap = useRuntimeStore((state) => state.bootstrap);
  const hydrateRuntime = useRuntimeStore((state) => state.hydrate);
  const hydrateUi = useUiStore((state) => state.hydrate);
  const previewLocale = useUiStore((state) => state.previewLocale);
  const { availableLocales, availableThemes, runtime, t } = useUiHelpers();
  const defaultDisplayName = resolveDefaultDisplayName(runtime);
  const defaultLocale = resolveDefaultLocale(runtime);
  const defaultTheme = resolveDefaultTheme(runtime);

  const [displayName, setDisplayName] = useState(defaultDisplayName);
  const timeZoneGroups = useMemo(() => buildTimeZoneOptionGroups(t, false), [t]);
  const [locale, setLocale] = useState(
    normalizeLocaleSelection(
      typeof navigator !== "undefined" ? navigator.language : defaultLocale,
      availableLocales,
      defaultLocale,
    ),
  );
  const [timeZone, setTimeZone] = useState(getBrowserTimeZone);
  const [themeId, setThemeId] = useState(defaultTheme);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (bootstrap?.is_initialized) {
      navigate({ to: "/" });
    }
  }, [bootstrap, navigate]);

  useEffect(() => {
    setLocale((current) => normalizeLocaleSelection(current, availableLocales, defaultLocale));
  }, [availableLocales, defaultLocale]);

  useEffect(() => {
    setDisplayName((current) => current || defaultDisplayName);
  }, [defaultDisplayName]);

  useEffect(() => {
    setThemeId((current) => current || defaultTheme);
  }, [defaultTheme]);

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
      navigate({ to: "/" });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="page page--centered onboarding-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
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
              <Select
                value={locale}
                onChange={(value) => void handleLocalePreview(value)}
                options={availableLocales.map((option) => ({ value: option, label: option }))}
              />
            </label>
          </div>

          <div className="form-row">
            <label>
              {t("settings.bootstrap.time_zone", "时区")}
              <Select
                value={timeZone}
                onChange={setTimeZone}
                options={timeZoneGroups.flatMap((group) =>
                  group.options.map((option) => ({ value: option.value, label: option.label, group: group.label }))
                )}
              />
            </label>
            <label>
              {t("settings.bootstrap.theme", "主题")}
              <Select
                value={themeId}
                onChange={previewTheme}
                options={availableThemes.map((theme) => ({ value: theme.id, label: theme.label }))}
              />
            </label>
          </div>
        </div>

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

