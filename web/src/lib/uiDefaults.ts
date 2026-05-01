import type { UiRuntime } from "@ennoia/api-client";

export const FRONTEND_UI_DEFAULTS = {
  defaultTheme: "system",
  defaultLocale: "zh-CN",
  fallbackLocale: "en-US",
  availableLocales: ["zh-CN", "en-US"] as string[],
  defaultDisplayName: "Operator",
  defaultTimeZone: "Asia/Shanghai",
} as const;

export function resolveDefaultTheme(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.default_theme ?? FRONTEND_UI_DEFAULTS.defaultTheme;
}

export function resolveDefaultLocale(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.default_locale ?? FRONTEND_UI_DEFAULTS.defaultLocale;
}

export function resolveFallbackLocale(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.fallback_locale ?? FRONTEND_UI_DEFAULTS.fallbackLocale;
}

export function resolveAvailableLocales(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.available_locales ?? [...FRONTEND_UI_DEFAULTS.availableLocales];
}

export function resolveDefaultDisplayName(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.default_display_name ?? FRONTEND_UI_DEFAULTS.defaultDisplayName;
}

export function resolveDefaultTimeZone(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.default_time_zone ?? FRONTEND_UI_DEFAULTS.defaultTimeZone;
}
