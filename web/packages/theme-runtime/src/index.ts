export type ThemeAppearance = "light" | "dark" | "system" | "high-contrast";

export type ThemeSource = "builtin" | "extension";

export type ThemeDefinition = {
  id: string;
  label: string;
  appearance: ThemeAppearance;
  previewColor: string;
  variables?: Record<string, string>;
  source: ThemeSource;
  cssUrl?: string | null;
  extends?: string | null;
  category?: string | null;
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
const ACTIVE_THEME_LINK_ID = "ennoia-runtime-theme-link";
const WEB_THEME_VARIABLE_BRIDGE: Record<string, string> = {
  "--bg": "--color-bg",
  "--bg-elevated": "--color-surface",
  "--bg-soft": "--color-surface-2",
  "--bg-panel": "--color-surface",
  "--line": "--color-border",
  "--line-strong": "--color-border",
  "--text": "--color-text",
  "--text-muted": "--color-text-muted",
  "--accent": "--color-primary",
  "--accent-soft": "--color-primary",
};

export const BUILTIN_THEMES: ThemeDefinition[] = [
  {
    id: "system",
    label: "System",
    appearance: "system",
    previewColor: "#5b8def",
    variables: {},
    source: "builtin",
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
    source: "builtin",
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
    source: "builtin",
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
    source: "builtin",
  },
];

const BUILTIN_THEME_MAP = new Map(BUILTIN_THEMES.map((theme) => [theme.id, theme]));
let runtimeThemeMap = new Map(BUILTIN_THEMES.map((theme) => [theme.id, theme]));
let systemThemeCleanup: (() => void) | null = null;

function activeThemeDefinitions() {
  return [...runtimeThemeMap.values()];
}

function themeVariableKeys() {
  return Array.from(
    new Set([
      ...activeThemeDefinitions().flatMap((theme) => Object.keys(theme.variables ?? {})),
      ...Object.keys(WEB_THEME_VARIABLE_BRIDGE),
    ]),
  );
}

function ensureSystemThemeObserver() {
  if (typeof window === "undefined" || systemThemeCleanup) {
    return;
  }
  const media = window.matchMedia("(prefers-color-scheme: dark)");
  const handler = () => {
    const activeThemeId = readUiBootstrapCache().theme_id ?? document.documentElement.dataset.theme;
    applyTheme(activeThemeId ?? "system");
  };
  if (typeof media.addEventListener === "function") {
    media.addEventListener("change", handler);
    systemThemeCleanup = () => media.removeEventListener("change", handler);
    return;
  }
  media.addListener(handler);
  systemThemeCleanup = () => media.removeListener(handler);
}

function ensureThemeLink() {
  if (typeof document === "undefined") {
    return null;
  }
  let link = document.getElementById(ACTIVE_THEME_LINK_ID) as HTMLLinkElement | null;
  if (!link) {
    link = document.createElement("link");
    link.id = ACTIVE_THEME_LINK_ID;
    link.rel = "stylesheet";
    document.head.appendChild(link);
  }
  return link;
}

function removeThemeLink() {
  if (typeof document === "undefined") {
    return;
  }
  document.getElementById(ACTIVE_THEME_LINK_ID)?.remove();
}

export function registerRuntimeThemes(themes: ThemeDefinition[]) {
  runtimeThemeMap = new Map(BUILTIN_THEMES.map((theme) => [theme.id, theme]));
  for (const theme of themes) {
    runtimeThemeMap.set(theme.id, theme);
  }
}

export function listThemeDefinitions() {
  return activeThemeDefinitions();
}

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
  return runtimeThemeMap.get(themeId ?? "") ?? BUILTIN_THEME_MAP.get("system")!;
}

function resolveSystemBaseTheme() {
  return window.matchMedia("(prefers-color-scheme: dark)").matches
    ? BUILTIN_THEME_MAP.get("ennoia.midnight")!
    : BUILTIN_THEME_MAP.get("ennoia.paper")!;
}

function resolveBaseTheme(theme: ThemeDefinition): ThemeDefinition {
  if (theme.appearance === "system") {
    return resolveSystemBaseTheme();
  }
  if (!theme.extends) {
    return theme.source === "builtin" ? theme : resolveSystemBaseTheme();
  }
  if (theme.extends === "system") {
    return theme.appearance === "dark"
      ? BUILTIN_THEME_MAP.get("ennoia.midnight")!
      : BUILTIN_THEME_MAP.get("ennoia.paper")!;
  }
  return resolveThemeDefinition(theme.extends);
}

export function resolveAppliedThemeDefinition(themeId?: string | null) {
  return resolveBaseTheme(resolveThemeDefinition(themeId));
}

export function applyTheme(themeId?: string | null) {
  if (typeof document === "undefined" || typeof window === "undefined") {
    return;
  }
  ensureSystemThemeObserver();

  const root = document.documentElement;
  const theme = resolveThemeDefinition(themeId);
  const baseTheme = resolveAppliedThemeDefinition(themeId);

  root.dataset.theme = theme.id;
  for (const key of themeVariableKeys()) {
    root.style.removeProperty(key);
  }
  removeThemeLink();

  root.style.colorScheme = baseTheme.appearance === "dark" ? "dark" : "light";
  for (const [key, value] of Object.entries(baseTheme.variables ?? {})) {
    root.style.setProperty(key, value);
  }
  for (const [target, source] of Object.entries(WEB_THEME_VARIABLE_BRIDGE)) {
    const value = baseTheme.variables?.[source];
    if (value) {
      root.style.setProperty(target, value);
    }
  }

  if (theme.source === "extension" && theme.cssUrl) {
    const link = ensureThemeLink();
    if (link) {
      link.href = theme.cssUrl;
    }
  }
}

export function bootstrapTheme() {
  const cache = readUiBootstrapCache();
  applyTheme(cache.theme_id ?? "system");
  if (typeof document !== "undefined" && cache.locale) {
    document.documentElement.lang = cache.locale;
  }
}
