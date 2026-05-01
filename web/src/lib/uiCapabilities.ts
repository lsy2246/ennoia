import type { UiRuntime } from "@ennoia/api-client";
import { getExtensionThemeStylesheetUrl } from "@ennoia/api-client";
import { BUILTIN_THEMES, type ThemeDefinition } from "@ennoia/theme-runtime";
import type { LocalizedText } from "@ennoia/ui-sdk";
import {
  FRONTEND_UI_DEFAULTS,
  resolveAvailableLocales,
  resolveDefaultTheme,
} from "@/lib/uiDefaults";

export type ThemeOption = {
  id: string;
  label: string;
  appearance: ThemeDefinition["appearance"];
  previewColor: string;
  source: ThemeDefinition["source"];
};

export function listSupportedLocales(runtime: UiRuntime | null | undefined) {
  return dedupe(resolveAvailableLocales(runtime));
}

export function normalizeLocaleSelection(
  candidate: string | null | undefined,
  supportedLocales: string[],
  fallbackLocale: string,
) {
  const options =
    supportedLocales.length > 0 ? supportedLocales : FRONTEND_UI_DEFAULTS.availableLocales;
  if (!candidate) {
    return options.includes(fallbackLocale) ? fallbackLocale : options[0];
  }
  const exact = options.find((item) => item.toLowerCase() === candidate.toLowerCase());
  if (exact) {
    return exact;
  }
  const language = candidate.toLowerCase().split("-")[0];
  const partial = options.find((item) => item.toLowerCase().split("-")[0] === language);
  return partial ?? (options.includes(fallbackLocale) ? fallbackLocale : options[0]);
}

export function buildRuntimeThemeDefinitions(runtime: UiRuntime | null | undefined): ThemeDefinition[] {
  return dedupeById([
    ...BUILTIN_THEMES,
    ...(runtime?.registry.themes ?? []).map((item) => ({
      id: item.theme.id,
      label: item.theme.label.fallback,
      appearance: normalizeAppearance(item.theme.appearance),
      previewColor: item.theme.preview_color ?? "#5b8def",
      source: "extension" as const,
      contract: item.theme.contract,
      cssUrl: getExtensionThemeStylesheetUrl(item.extension_id, item.theme.id),
      extends: item.theme.extends,
      category: item.theme.category,
    })),
  ]);
}

function normalizeAppearance(value: string): ThemeDefinition["appearance"] {
  const normalized = value.toLowerCase();
  if (
    normalized === "light" ||
    normalized === "dark" ||
    normalized === "system" ||
    normalized === "high-contrast"
  ) {
    return normalized;
  }
  return "system";
}

export function normalizeThemeSelection(themeId: string | null | undefined, runtime: UiRuntime | null | undefined) {
  const themes = buildRuntimeThemeDefinitions(runtime);
  if (themeId && themes.some((item) => item.id === themeId)) {
    return themeId;
  }
  const fallback = resolveDefaultTheme(runtime);
  return themes.some((item) => item.id === fallback)
    ? fallback
    : FRONTEND_UI_DEFAULTS.defaultTheme;
}

export function buildThemeOptions(
  runtime: UiRuntime | null | undefined,
  resolveText: (text: LocalizedText) => string,
): ThemeOption[] {
  const extensionThemeLabels = new Map(
    (runtime?.registry.themes ?? []).map((item) => [item.theme.id, resolveText(item.theme.label)]),
  );

  return buildRuntimeThemeDefinitions(runtime).map((item) => ({
    id: item.id,
    label: extensionThemeLabels.get(item.id) ?? item.label,
    appearance: item.appearance,
    previewColor: item.previewColor,
    source: item.source,
  }));
}

function dedupe(values: string[]) {
  return [...new Set(values)];
}

function dedupeById(values: ThemeDefinition[]) {
  const map = new Map<string, ThemeDefinition>();
  for (const value of values) {
    if (!map.has(value.id)) {
      map.set(value.id, value);
    }
  }
  return [...map.values()];
}
