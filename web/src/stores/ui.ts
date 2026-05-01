import { create } from "zustand";

import {
  apiUrl,
  fetchUiMessages,
  fetchUiRuntime,
  saveInstanceUiPreferences,
  type UiRuntime,
} from "@ennoia/api-client";
import {
  buildRuntimeThemeDefinitions,
  buildThemeOptions,
  listSupportedLocales,
  normalizeLocaleSelection,
  normalizeThemeSelection,
} from "@/lib/uiCapabilities";
import {
  FRONTEND_UI_DEFAULTS,
  resolveDefaultLocale,
} from "@/lib/uiDefaults";
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
  registerRuntimeThemes,
  readUiBootstrapCache,
  resolveAppliedThemeDefinition,
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
  refreshRuntime: () => Promise<void>;
  connectExtensionEvents: () => () => void;
  previewLocale: (locale: string) => Promise<void>;
  savePreferences: (payload: {
    locale?: string | null;
    theme_id?: string | null;
    time_zone?: string | null;
    date_style?: string | null;
  }) => Promise<void>;
};

function pickEffectiveLocale(runtime: UiRuntime | null, cachedLocale?: string) {
  const supportedLocales = listSupportedLocales(runtime);
  return normalizeLocaleSelection(
    cachedLocale ?? runtime?.instance_preference?.preference.locale,
    supportedLocales,
    resolveDefaultLocale(runtime),
  );
}

function pickEffectiveTheme(runtime: UiRuntime | null, cachedTheme?: string) {
  return normalizeThemeSelection(
    cachedTheme ?? runtime?.instance_preference?.preference.theme_id,
    runtime,
  );
}

function syncThemeDefinitions(runtime: UiRuntime | null) {
  registerRuntimeThemes(buildRuntimeThemeDefinitions(runtime));
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
  locale: readUiBootstrapCache().locale ?? FRONTEND_UI_DEFAULTS.defaultLocale,
  themeId: readUiBootstrapCache().theme_id ?? FRONTEND_UI_DEFAULTS.defaultTheme,
  timeZone: readUiBootstrapCache().time_zone,
  dateStyle: readUiBootstrapCache().date_style,
  error: null,

  async hydrate() {
    set({ status: "checking", error: null });
    try {
      bootstrapTheme();
      const cached = readUiBootstrapCache();
      const runtime = await fetchUiRuntime();
      syncThemeDefinitions(runtime);
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

  async refreshRuntime() {
    const current = get();
    try {
      const runtime = await fetchUiRuntime();
      if (current.runtime && sameUiRuntime(current.runtime, runtime)) {
        return;
      }
      syncThemeDefinitions(runtime);
      const locale = pickEffectiveLocale(runtime, current.locale);
      const themeId = pickEffectiveTheme(runtime, current.themeId);
      const messagesVersion =
        locale !== current.locale || runtime.versions.registry !== current.runtime?.versions.registry
          ? await syncLocaleMessages(locale, runtime)
          : current.messagesVersion;

      applyTheme(themeId);
      document.documentElement.lang = locale;
      writeUiBootstrapCache({
        locale,
        theme_id: themeId,
        time_zone: current.timeZone,
        date_style: current.dateStyle,
        version: runtime.versions.preferences,
        updated_at: runtime.instance_preference?.preference.updated_at,
      });
      set({
        runtime,
        locale,
        themeId,
        messagesVersion,
      });
    } catch (error) {
      set({ error: String(error) });
    }
  },

  connectExtensionEvents() {
    if (typeof EventSource === "undefined") {
      return () => undefined;
    }
    const source = new EventSource(apiUrl("/api/extensions/events/stream"));
    const refresh = () => {
      void get().refreshRuntime();
    };
    source.addEventListener("extension.graph_swapped", refresh);
    source.onerror = () => undefined;
    return () => {
      source.removeEventListener("extension.graph_swapped", refresh);
      source.close();
    };
  },

  async previewLocale(locale) {
    const current = get();
    let messagesVersion = current.messagesVersion;
    let errorMessage: string | null = null;
    try {
      messagesVersion = await syncLocaleMessages(locale, current.runtime);
    } catch (error) {
      errorMessage = String(error);
    }

    document.documentElement.lang = locale;
    writeUiBootstrapCache({
      ...readUiBootstrapCache(),
      locale,
      updated_at: new Date().toISOString(),
    });
    set({
      locale,
      messagesVersion,
      error: errorMessage,
    });
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
    const savedLocale = saved.preference.locale ?? nextLocale;
    const savedTheme = saved.preference.theme_id ?? nextTheme;
    applyTheme(savedTheme);
    document.documentElement.lang = savedLocale;
    writeUiBootstrapCache({
      locale: savedLocale,
      theme_id: savedTheme,
      time_zone: saved.preference.time_zone ?? nextTimeZone,
      date_style: saved.preference.date_style ?? nextDateStyle,
      version: saved.preference.version,
      updated_at: saved.preference.updated_at,
    });
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
      locale: savedLocale,
      themeId: savedTheme,
      timeZone: saved.preference.time_zone ?? nextTimeZone,
      dateStyle: saved.preference.date_style ?? nextDateStyle,
      messagesVersion,
      error: errorMessage,
    }));
  },
}));

function sameUiRuntime(current: UiRuntime, next: UiRuntime) {
  return JSON.stringify({
    ui_config: current.ui_config,
    registry: current.registry,
    instance_preference: current.instance_preference,
    space_preferences: current.space_preferences,
    versions: {
      registry: current.versions.registry,
      preferences: current.versions.preferences,
    },
  }) === JSON.stringify({
    ui_config: next.ui_config,
    registry: next.registry,
    instance_preference: next.instance_preference,
    space_preferences: next.space_preferences,
    versions: {
      registry: next.versions.registry,
      preferences: next.versions.preferences,
    },
  });
}

export function useUiHelpers() {
  const locale = useUiStore((s) => s.locale);
  const timeZone = useUiStore((s) => s.timeZone);
  const runtime = useUiStore((s) => s.runtime);
  const messagesVersion = useUiStore((s) => s.messagesVersion);
  void messagesVersion;
  const resolveText = (text: LocalizedText) =>
    resolveLocalizedText(text, locale, builtinI18nRegistry);
  const localizeBuiltinThemeLabel = (themeId: string, fallback: string) => {
    const themeKey = `settings.theme.${themeId.replace(/[^a-zA-Z0-9]+/g, "_")}`;
    return translate(locale, themeKey, fallback, builtinI18nRegistry);
  };

  return {
    locale,
    runtime,
    availableLocales: listSupportedLocales(runtime),
    availableThemes: buildThemeOptions(runtime, resolveText).map((item) => ({
      ...item,
      label:
        item.source === "builtin" ? localizeBuiltinThemeLabel(item.id, item.label) : item.label,
    })),
    resolveText,
    t: (key: string, fallback: string) => translate(locale, key, fallback, builtinI18nRegistry),
    formatDateTime: (value: string | number | Date) => formatDateTime(value, locale, timeZone),
    formatDate: (value: string | number | Date) => formatDate(value, locale, timeZone),
    formatTime: (value: string | number | Date) => formatTime(value, locale, timeZone),
    describeTheme: (themeId: string) => resolveThemeDefinition(themeId),
    describeAppliedTheme: (themeId: string) => resolveAppliedThemeDefinition(themeId),
  };
}
