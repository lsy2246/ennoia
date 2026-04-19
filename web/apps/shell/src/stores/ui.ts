import { create } from "zustand";

import {
  fetchUiMessages,
  fetchUiRuntime,
  saveInstanceUiPreferences,
  type UiRuntime,
} from "@ennoia/api-client";
import {
  builtinI18nRegistry,
  formatDate,
  formatDateTime,
  formatTime,
  getBuiltinNamespaces,
  resolveLocalizedText,
  translate,
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
  messagesVersion: number;
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
    runtime?.instance_preference?.preference.locale ??
    runtime?.ui_config.default_locale ??
    "en-US"
  );
}

function pickEffectiveTheme(runtime: UiRuntime | null, cachedTheme?: string) {
  return (
    cachedTheme ??
    runtime?.instance_preference?.preference.theme_id ??
    runtime?.ui_config.default_theme ??
    "system"
  );
}

function collectMessageNamespaces(runtime: UiRuntime | null) {
  const namespaces = new Set(getBuiltinNamespaces());
  for (const contribution of runtime?.registry.locales ?? []) {
    namespaces.add(contribution.locale.namespace);
  }
  return [...namespaces];
}

async function syncLocaleMessages(locale: string, runtime: UiRuntime | null) {
  const response = await fetchUiMessages(locale, collectMessageNamespaces(runtime));
  builtinI18nRegistry.clearRuntimeBundles();
  builtinI18nRegistry.registerBundles(response.bundles);
  return builtinI18nRegistry.getRevision();
}

export const useUiStore = create<UiState>((set, get) => ({
  status: "idle",
  runtime: null,
  messagesVersion: builtinI18nRegistry.getRevision(),
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
      let errorMessage: string | null = null;
      let messagesVersion = builtinI18nRegistry.getRevision();
      try {
        messagesVersion = await syncLocaleMessages(locale, runtime);
      } catch (error) {
        errorMessage = String(error);
      }
      const timeZone =
        cached.time_zone ?? runtime.instance_preference?.preference.time_zone ?? undefined;
      const dateStyle =
        cached.date_style ?? runtime.instance_preference?.preference.date_style ?? undefined;
      applyTheme(themeId);
      document.documentElement.lang = locale;
      writeUiBootstrapCache({
        locale,
        theme_id: themeId,
        time_zone: timeZone,
        date_style: dateStyle,
        version: runtime.versions.preferences,
        updated_at: runtime.instance_preference?.preference.updated_at ?? cached.updated_at,
      });
      set({
        status: "ready",
        runtime,
        messagesVersion,
        locale,
        themeId,
        timeZone,
        dateStyle,
        error: errorMessage,
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
    let errorMessage: string | null = null;
    let messagesVersion = current.messagesVersion;

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
      error: null,
    });

    if (nextLocale !== current.locale) {
      try {
        messagesVersion = await syncLocaleMessages(nextLocale, current.runtime);
      } catch (error) {
        errorMessage = String(error);
      }
    }

    const saved = await saveInstanceUiPreferences(payload);
    set((state) => ({
      runtime: state.runtime
        ? {
            ...state.runtime,
            instance_preference: saved,
            versions: {
              ...state.runtime.versions,
              preferences: saved.preference.version,
            },
          }
        : state.runtime,
      messagesVersion,
      error: errorMessage,
    }));
  },
}));

export function useUiHelpers() {
  const locale = useUiStore((s) => s.locale);
  const timeZone = useUiStore((s) => s.timeZone);
  const runtime = useUiStore((s) => s.runtime);
  const messagesVersion = useUiStore((s) => s.messagesVersion);
  void messagesVersion;

  return {
    locale,
    runtime,
    availableThemes: BUILTIN_THEMES,
    resolveText: (text: LocalizedText) => resolveLocalizedText(text, locale, builtinI18nRegistry),
    t: (key: string, fallback: string) => translate(locale, key, fallback, builtinI18nRegistry),
    formatDateTime: (value: string | number | Date) => formatDateTime(value, locale, timeZone),
    formatDate: (value: string | number | Date) => formatDate(value, locale, timeZone),
    formatTime: (value: string | number | Date) => formatTime(value, locale, timeZone),
    describeTheme: (themeId: string) => resolveThemeDefinition(themeId),
  };
}
