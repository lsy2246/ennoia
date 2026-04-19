export type ThemeAppearance = "light" | "dark" | "system" | "high-contrast";

export type ThemeDefinition = {
  id: string;
  label: string;
  appearance: ThemeAppearance;
  previewColor: string;
  variables: Record<string, string>;
};

export type UiBootstrapCache = {
  locale?: string;
  theme_id?: string;
  time_zone?: string;
  date_style?: string;
  version?: number;
  updated_at?: string;
};

export const UI_BOOTSTRAP_CACHE_KEY = "ennoia.ui.bootstrap";

export const BUILTIN_THEMES: ThemeDefinition[] = [
  {
    id: "system",
    label: "System",
    appearance: "system",
    previewColor: "#5b8def",
    variables: {},
  },
  {
    id: "ennoia.midnight",
    label: "Midnight",
    appearance: "dark",
    previewColor: "#5b8def",
    variables: {
      "--color-bg": "#0b1221",
      "--color-surface": "#141a2a",
      "--color-surface-2": "#1b2236",
      "--color-border": "#23304a",
      "--color-text": "#e6ebf5",
      "--color-text-muted": "#7b8aa8",
      "--color-primary": "#5b8def",
      "--color-primary-hover": "#7aa3ff",
    },
  },
  {
    id: "ennoia.paper",
    label: "Paper",
    appearance: "light",
    previewColor: "#2f6fed",
    variables: {
      "--color-bg": "#f5f1e8",
      "--color-surface": "#fffaf1",
      "--color-surface-2": "#ece3d3",
      "--color-border": "#d3c5ad",
      "--color-text": "#1f2532",
      "--color-text-muted": "#6b7280",
      "--color-primary": "#2f6fed",
      "--color-primary-hover": "#3f7cff",
    },
  },
  {
    id: "observatory.daybreak",
    label: "Daybreak",
    appearance: "light",
    previewColor: "#f4a261",
    variables: {
      "--color-bg": "#fcf4ea",
      "--color-surface": "#fffaf3",
      "--color-surface-2": "#f3e4d0",
      "--color-border": "#e5c9a2",
      "--color-text": "#33231c",
      "--color-text-muted": "#8e6f5b",
      "--color-primary": "#d97706",
      "--color-primary-hover": "#ea8c1d",
    },
  },
];

export function readUiBootstrapCache(): UiBootstrapCache {
  if (typeof window === "undefined") {
    return {};
  }
  try {
    return JSON.parse(localStorage.getItem(UI_BOOTSTRAP_CACHE_KEY) ?? "{}");
  } catch {
    return {};
  }
}

export function writeUiBootstrapCache(cache: UiBootstrapCache) {
  if (typeof window === "undefined") {
    return;
  }
  localStorage.setItem(UI_BOOTSTRAP_CACHE_KEY, JSON.stringify(cache));
}

export function resolveThemeDefinition(themeId?: string | null) {
  return BUILTIN_THEMES.find((item) => item.id === themeId) ?? BUILTIN_THEMES[0];
}

export function applyTheme(themeId?: string | null) {
  if (typeof document === "undefined") {
    return;
  }
  const root = document.documentElement;
  const theme = resolveThemeDefinition(themeId);
  root.dataset.theme = theme.id;
  const resolved =
    theme.appearance === "system"
      ? window.matchMedia("(prefers-color-scheme: dark)").matches
        ? resolveThemeDefinition("ennoia.midnight")
        : resolveThemeDefinition("ennoia.paper")
      : theme;
  root.style.colorScheme = resolved.appearance === "dark" ? "dark" : "light";

  for (const [key, value] of Object.entries(resolved.variables)) {
    root.style.setProperty(key, value);
  }
}

export function bootstrapTheme() {
  const cache = readUiBootstrapCache();
  applyTheme(cache.theme_id ?? "system");
  if (typeof document !== "undefined" && cache.locale) {
    document.documentElement.lang = cache.locale;
  }
}
