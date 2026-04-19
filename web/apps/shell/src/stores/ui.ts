import { create } from "zustand";

import {
  fetchUiRuntime,
  saveMyUiPreferences,
  type UiRuntime,
} from "@ennoia/api-client";
import {
  formatDate,
  formatDateTime,
  formatTime,
  getCoreMessages,
  resolveLocalizedText,
  translate,
  type TranslationBundle,
} from "@ennoia/i18n";
import {
  applyTheme,
  bootstrapTheme,
  BUILTIN_THEMES,
  readUiBootstrapCache,
  resolveThemeDefinition,
  writeUiBootstrapCache,
} from "@ennoia/theme-runtime";
import type { LocalizedText } from "@ennoia/ui-sdk";

type UiStatus = "idle" | "checking" | "ready" | "error";

type UiState = {
  status: UiStatus;
  runtime: UiRuntime | null;
  locale: string;
  themeId: string;
  timeZone?: string;
  dateStyle?: string;
  error: string | null;
  hydrate: () => Promise<void>;
  savePreferences: (payload: {
    locale?: string | null;
    theme_id?: string | null;
    time_zone?: string | null;
    date_style?: string | null;
  }) => Promise<void>;
};

function pickEffectiveLocale(runtime: UiRuntime | null, cachedLocale?: string) {
  return (
    cachedLocale ??
    runtime?.user_preference?.preference.locale ??
    runtime?.ui_config.default_locale ??
    "en-US"
  );
}

function pickEffectiveTheme(runtime: UiRuntime | null, cachedTheme?: string) {
  return (
    cachedTheme ??
    runtime?.user_preference?.preference.theme_id ??
    runtime?.ui_config.default_theme ??
    "system"
  );
}

export const useUiStore = create<UiState>((set, get) => ({
  status: "idle",
  runtime: null,
  locale: readUiBootstrapCache().locale ?? "zh-CN",
  themeId: readUiBootstrapCache().theme_id ?? "system",
  timeZone: readUiBootstrapCache().time_zone,
  dateStyle: readUiBootstrapCache().date_style,
  error: null,

  async hydrate() {
    set({ status: "checking", error: null });
    try {
      bootstrapTheme();
      const cached = readUiBootstrapCache();
      const runtime = await fetchUiRuntime();
      const locale = pickEffectiveLocale(runtime, cached.locale);
      const themeId = pickEffectiveTheme(runtime, cached.theme_id);
      const timeZone =
        cached.time_zone ?? runtime.user_preference?.preference.time_zone ?? undefined;
      const dateStyle =
        cached.date_style ?? runtime.user_preference?.preference.date_style ?? undefined;
      applyTheme(themeId);
      document.documentElement.lang = locale;
      writeUiBootstrapCache({
        locale,
        theme_id: themeId,
        time_zone: timeZone,
        date_style: dateStyle,
        version: runtime.versions.preferences,
        updated_at: runtime.user_preference?.preference.updated_at ?? cached.updated_at,
      });
      set({
        status: "ready",
        runtime,
        locale,
        themeId,
        timeZone,
        dateStyle,
      });
    } catch (error) {
      set({ status: "error", error: String(error) });
    }
  },

  async savePreferences(payload) {
    const current = get();
    const nextLocale = payload.locale ?? current.locale;
    const nextTheme = payload.theme_id ?? current.themeId;
    const nextTimeZone = payload.time_zone ?? current.timeZone;
    const nextDateStyle = payload.date_style ?? current.dateStyle;

    applyTheme(nextTheme);
    document.documentElement.lang = nextLocale;
    writeUiBootstrapCache({
      locale: nextLocale,
      theme_id: nextTheme,
      time_zone: nextTimeZone,
      date_style: nextDateStyle,
      version: current.runtime?.versions.preferences ?? 0,
      updated_at: new Date().toISOString(),
    });
    set({
      locale: nextLocale,
      themeId: nextTheme,
      timeZone: nextTimeZone,
      dateStyle: nextDateStyle,
    });

    const saved = await saveMyUiPreferences(payload);
    set((state) => ({
      runtime: state.runtime
        ? {
            ...state.runtime,
            user_preference: saved,
            versions: {
              ...state.runtime.versions,
              preferences: saved.preference.version,
            },
          }
        : state.runtime,
    }));
  },
}));

export function useUiHelpers() {
  const locale = useUiStore((s) => s.locale);
  const timeZone = useUiStore((s) => s.timeZone);
  const runtime = useUiStore((s) => s.runtime);

  const bundles: TranslationBundle[] = [getCoreMessages(locale)];

  return {
    locale,
    runtime,
    availableThemes: BUILTIN_THEMES,
    resolveText: (text: LocalizedText) => resolveLocalizedText(text, locale, bundles),
    t: (key: string, fallback: string) => translate(locale, key, fallback, bundles),
    formatDateTime: (value: string | number | Date) => formatDateTime(value, locale, timeZone),
    formatDate: (value: string | number | Date) => formatDate(value, locale, timeZone),
    formatTime: (value: string | number | Date) => formatTime(value, locale, timeZone),
    describeTheme: (themeId: string) => resolveThemeDefinition(themeId),
  };
}
