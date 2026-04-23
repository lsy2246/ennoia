export type ThemeAppearance = "light" | "dark" | "system" | "high-contrast";

export type ThemeSource = "builtin" | "extension";

export const THEME_CONTRACT = "ennoia.theme";

export const REQUIRED_THEME_TOKENS = [
  "--color-bg",
  "--color-surface",
  "--color-surface-2",
  "--color-border",
  "--color-text",
  "--color-text-muted",
  "--color-primary",
  "--color-primary-hover",
] as const;

export const DOCKVIEW_THEME_TOKENS = [
  "--dockview-header-surface",
  "--dockview-header-border",
  "--dockview-tab-surface",
  "--dockview-tab-hover",
  "--dockview-tab-border",
  "--dockview-tab-accent",
  "--dockview-drop-surface",
  "--dockview-tab-shadow",
  "--dockview-divider-shadow",
  "--dockview-splitter-line",
  "--dockview-splitter-track",
  "--dockview-splitter-hover",
  "--dockview-empty-surface",
  "--dockview-empty-border",
  "--dockview-empty-accent",
  "--dockview-empty-card",
  "--dockview-empty-card-hover",
] as const;

export const WEB_ALIAS_THEME_TOKENS = [
  "--bg",
  "--bg-elevated",
  "--bg-soft",
  "--bg-panel",
  "--line",
  "--line-strong",
  "--text",
  "--text-muted",
  "--accent",
  "--accent-soft",
] as const;

export type ThemeDefinition = {
  id: string;
  label: string;
  appearance: ThemeAppearance;
  previewColor: string;
  variables?: Record<string, string>;
  source: ThemeSource;
  contract?: string | null;
  cssUrl?: string | null;
  extends?: string | null;
  category?: string | null;
};

export type ThemeValidationResult = {
  valid: boolean;
  diagnostics: string[];
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
export const WEB_THEME_VARIABLE_BRIDGE: Record<string, string> = {
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
  "--dockview-header-surface": "--color-surface",
  "--dockview-header-border": "--color-border",
  "--dockview-tab-surface": "--color-surface",
  "--dockview-tab-hover": "--color-surface-2",
  "--dockview-tab-border": "--color-border",
  "--dockview-tab-accent": "--color-primary",
  "--dockview-drop-surface": "--color-surface-2",
  "--dockview-splitter-line": "--color-border",
  "--dockview-splitter-track": "--color-border",
  "--dockview-splitter-hover": "--color-primary",
  "--dockview-empty-surface": "--color-surface",
  "--dockview-empty-border": "--color-border",
  "--dockview-empty-accent": "--color-primary",
  "--dockview-empty-card": "--color-surface",
  "--dockview-empty-card-hover": "--color-surface-2",
};

export function isSupportedThemeContract(contract?: string | null) {
  return !contract || contract === THEME_CONTRACT;
}

export function validateThemeDefinition(theme: ThemeDefinition): ThemeValidationResult {
  const diagnostics: string[] = [];
  if (!isSupportedThemeContract(theme.contract)) {
    diagnostics.push(`unsupported theme contract '${theme.contract}'`);
  }
  if (theme.source === "extension" && !theme.cssUrl) {
    diagnostics.push("extension theme must provide cssUrl");
  }
  if (theme.variables) {
    for (const token of REQUIRED_THEME_TOKENS) {
      const value = theme.variables[token];
      if (!value || !value.trim()) {
        diagnostics.push(`theme variables missing required token '${token}'`);
      }
    }
  }
  return {
    valid: diagnostics.length === 0,
    diagnostics,
  };
}

export function listThemeContractTokens() {
  return {
    contract: THEME_CONTRACT,
    required: [...REQUIRED_THEME_TOKENS],
    dockview: [...DOCKVIEW_THEME_TOKENS],
    aliases: [...WEB_ALIAS_THEME_TOKENS],
  };
}

const LIGHT_DOCKVIEW_TOKENS = {
  "--dockview-header-surface": "rgba(255, 255, 255, 0.84)",
  "--dockview-header-border": "rgba(0, 0, 0, 0.08)",
  "--dockview-tab-surface": "rgba(255, 255, 255, 0.96)",
  "--dockview-tab-hover": "rgba(0, 0, 0, 0.04)",
  "--dockview-tab-border": "rgba(0, 0, 0, 0.08)",
  "--dockview-tab-accent": "var(--color-primary)",
  "--dockview-drop-surface": "rgba(0, 122, 255, 0.12)",
  "--dockview-tab-shadow": "inset 0 1px 0 rgba(255, 255, 255, 0.58)",
  "--dockview-divider-shadow": "rgba(255, 255, 255, 0.55)",
  "--dockview-splitter-line": "rgba(0, 0, 0, 0.12)",
  "--dockview-splitter-track": "rgba(0, 0, 0, 0.08)",
  "--dockview-splitter-hover": "var(--color-primary)",
  "--dockview-empty-surface": "linear-gradient(180deg, rgba(255, 255, 255, 0.94), rgba(255, 255, 255, 0.82))",
  "--dockview-empty-border": "rgba(0, 0, 0, 0.08)",
  "--dockview-empty-accent": "rgba(0, 122, 255, 0.14)",
  "--dockview-empty-card": "rgba(255, 255, 255, 0.8)",
  "--dockview-empty-card-hover": "rgba(255, 255, 255, 0.96)",
};

const DARK_DOCKVIEW_TOKENS = {
  "--dockview-header-surface": "rgba(20, 24, 32, 0.82)",
  "--dockview-header-border": "rgba(255, 255, 255, 0.08)",
  "--dockview-tab-surface": "rgba(255, 255, 255, 0.04)",
  "--dockview-tab-hover": "rgba(255, 255, 255, 0.05)",
  "--dockview-tab-border": "rgba(255, 255, 255, 0.08)",
  "--dockview-tab-accent": "var(--color-primary)",
  "--dockview-drop-surface": "rgba(10, 132, 255, 0.16)",
  "--dockview-tab-shadow": "inset 0 1px 0 rgba(255, 255, 255, 0.05)",
  "--dockview-divider-shadow": "rgba(0, 0, 0, 0.2)",
  "--dockview-splitter-line": "rgba(255, 255, 255, 0.12)",
  "--dockview-splitter-track": "rgba(255, 255, 255, 0.08)",
  "--dockview-splitter-hover": "var(--color-primary)",
  "--dockview-empty-surface": "linear-gradient(180deg, rgba(20, 24, 32, 0.94), rgba(20, 24, 32, 0.82))",
  "--dockview-empty-border": "rgba(255, 255, 255, 0.08)",
  "--dockview-empty-accent": "rgba(10, 132, 255, 0.18)",
  "--dockview-empty-card": "rgba(255, 255, 255, 0.04)",
  "--dockview-empty-card-hover": "rgba(255, 255, 255, 0.06)",
};

export const BUILTIN_THEMES: ThemeDefinition[] = [
  {
    id: "system",
    label: "System",
    appearance: "system",
    previewColor: "#007aff",
    variables: {},
    source: "builtin",
    contract: THEME_CONTRACT,
  },
  {
    id: "apple.light",
    label: "macOS Light",
    appearance: "light",
    previewColor: "#007aff",
    variables: {
      "--color-bg": "#f5f5f7",
      "--color-surface": "#ffffff",
      "--color-surface-2": "#ececec",
      "--color-border": "rgba(0, 0, 0, 0.1)",
      "--color-text": "#1d1d1f",
      "--color-text-muted": "#86868b",
      "--color-primary": "#007aff",
      "--color-primary-hover": "#005bb5",
      ...LIGHT_DOCKVIEW_TOKENS,
    },
    source: "builtin",
    contract: THEME_CONTRACT,
  },
  {
    id: "apple.dark",
    label: "macOS Dark",
    appearance: "dark",
    previewColor: "#0a84ff",
    variables: {
      "--color-bg": "#1e1e1e",
      "--color-surface": "#282828",
      "--color-surface-2": "#323232",
      "--color-border": "rgba(255, 255, 255, 0.1)",
      "--color-text": "#f5f5f7",
      "--color-text-muted": "#98989d",
      "--color-primary": "#0a84ff",
      "--color-primary-hover": "#409cff",
      ...DARK_DOCKVIEW_TOKENS,
    },
    source: "builtin",
    contract: THEME_CONTRACT,
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
      "--dockview-header-surface": "rgba(20, 26, 42, 0.88)",
      "--dockview-header-border": "#23304a",
      "--dockview-tab-surface": "rgba(255, 255, 255, 0.05)",
      "--dockview-tab-hover": "rgba(255, 255, 255, 0.04)",
      "--dockview-tab-border": "#23304a",
      "--dockview-tab-accent": "#7aa3ff",
      "--dockview-drop-surface": "rgba(122, 163, 255, 0.14)",
      "--dockview-tab-shadow": "inset 0 1px 0 rgba(255, 255, 255, 0.05)",
      "--dockview-divider-shadow": "rgba(0, 0, 0, 0.2)",
      "--dockview-splitter-line": "#2b3a58",
      "--dockview-splitter-track": "rgba(43, 58, 88, 0.86)",
      "--dockview-splitter-hover": "#7aa3ff",
      "--dockview-empty-surface": "linear-gradient(180deg, rgba(20, 26, 42, 0.96), rgba(20, 26, 42, 0.84))",
      "--dockview-empty-border": "#23304a",
      "--dockview-empty-accent": "rgba(122, 163, 255, 0.14)",
      "--dockview-empty-card": "rgba(255, 255, 255, 0.04)",
      "--dockview-empty-card-hover": "rgba(255, 255, 255, 0.06)",
    },
    source: "builtin",
    contract: THEME_CONTRACT,
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
      "--dockview-header-surface": "rgba(255, 250, 241, 0.88)",
      "--dockview-header-border": "#d3c5ad",
      "--dockview-tab-surface": "rgba(255, 255, 255, 0.9)",
      "--dockview-tab-hover": "rgba(211, 197, 173, 0.16)",
      "--dockview-tab-border": "#d3c5ad",
      "--dockview-tab-accent": "#2f6fed",
      "--dockview-drop-surface": "rgba(47, 111, 237, 0.12)",
      "--dockview-tab-shadow": "inset 0 1px 0 rgba(255, 255, 255, 0.64)",
      "--dockview-divider-shadow": "rgba(255, 255, 255, 0.5)",
      "--dockview-splitter-line": "#d3c5ad",
      "--dockview-splitter-track": "rgba(211, 197, 173, 0.9)",
      "--dockview-splitter-hover": "#2f6fed",
      "--dockview-empty-surface": "linear-gradient(180deg, rgba(255, 250, 241, 0.96), rgba(255, 250, 241, 0.86))",
      "--dockview-empty-border": "#d3c5ad",
      "--dockview-empty-accent": "rgba(47, 111, 237, 0.12)",
      "--dockview-empty-card": "rgba(255, 255, 255, 0.76)",
      "--dockview-empty-card-hover": "rgba(255, 255, 255, 0.94)",
    },
    source: "builtin",
    contract: THEME_CONTRACT,
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
    const validation = validateThemeDefinition(theme);
    if (!validation.valid) {
      console.warn(`[theme-runtime] skipped theme '${theme.id}': ${validation.diagnostics.join("; ")}`);
      continue;
    }
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
    ? BUILTIN_THEME_MAP.get("apple.dark")!
    : BUILTIN_THEME_MAP.get("apple.light")!;
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
      ? BUILTIN_THEME_MAP.get("apple.dark")!
      : BUILTIN_THEME_MAP.get("apple.light")!;
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
  root.dataset.themeContract = theme.contract ?? THEME_CONTRACT;
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
