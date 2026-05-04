import type { UiRuntime } from "@ennoia/api-client";
import { readUiBootstrapCache } from "@ennoia/theme-runtime";

export const FRONTEND_UI_DEFAULTS = {
  defaultTheme: "system",
  defaultLocale: "zh-CN",
  fallbackLocale: "en-US",
  availableLocales: ["zh-CN", "en-US"] as string[],
  defaultDisplayName: "Operator",
  defaultTimeZone: "Asia/Shanghai",
  pauseNotificationsOnHover: true,
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

function readBootstrapNumber(value: unknown) {
  const parsed = Number(value);
  if (!Number.isFinite(parsed) || parsed <= 0) {
    return undefined;
  }
  return Math.trunc(parsed);
}

export function resolveDefaultRequestTimeoutMs(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.api.default_request_timeout_ms ?? undefined;
}

export function resolveSuccessAutoDismissMs(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.notifications.success_auto_dismiss_ms
    ?? readBootstrapNumber(readUiBootstrapCache().success_auto_dismiss_ms);
}

export function resolveErrorAutoDismissMs(runtime: UiRuntime | null | undefined) {
  return runtime?.ui_config.notifications.error_auto_dismiss_ms
    ?? readBootstrapNumber(readUiBootstrapCache().error_auto_dismiss_ms);
}

export function resolvePauseNotificationsOnHover(runtime: UiRuntime | null | undefined) {
  const cached = readUiBootstrapCache().pause_notifications_on_hover;
  return runtime?.ui_config.notifications.pause_on_hover
    ?? (typeof cached === "boolean" ? cached : FRONTEND_UI_DEFAULTS.pauseNotificationsOnHover);
}
