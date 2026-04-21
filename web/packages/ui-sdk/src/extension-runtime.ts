export type LocalizedText = {
  key: string;
  fallback: string;
};

export type ExtensionPageContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  page: {
    id: string;
    title: LocalizedText;
    route: string;
    mount: string;
    icon?: string | null;
  };
};

export type ExtensionPanelContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  panel: {
    id: string;
    title: LocalizedText;
    mount: string;
    slot: string;
    icon?: string | null;
  };
};

export type ThemeAppearance = "light" | "dark" | "system" | "high-contrast";

export type ExtensionThemeContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  theme: {
    id: string;
    label: LocalizedText;
    appearance: ThemeAppearance;
    tokens_entry: string;
    preview_color?: string | null;
    extends?: string | null;
    category?: string | null;
  };
};

export type ExtensionLocaleContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  locale: {
    locale: string;
    namespace: string;
    entry: string;
    version: string;
  };
};

export type PanelSlot = "left" | "right" | "bottom" | "main";

export type ExtensionPageDescriptor = {
  mount: string;
  eyebrow: string;
  summary: string;
  highlights: string[];
};

export type ExtensionPanelDescriptor = {
  mount: string;
  summary: string;
  slot: PanelSlot;
  metricLabel: string;
};

export type ExtensionCommandContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  command: {
    id: string;
    title: LocalizedText;
    action: string;
    shortcut?: string | null;
  };
};

export type ExtensionProviderContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  provider: {
    id: string;
    kind: string;
    entry?: string | null;
    extension_id?: string | null;
    interfaces: string[];
    model_discovery: boolean;
    recommended_model?: string | null;
    manual_model: boolean;
  };
};

export type ExtensionHookContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  source_mode: "workspace" | "package";
  install_dir: string;
  hook: {
    event: string;
    handler?: string | null;
  };
};

export type ResolvedFrontendEntry = {
  kind: string;
  entry: string;
  hmr: boolean;
};

export type ResolvedBackendEntry = {
  kind: string;
  runtime: string;
  entry: string;
  command?: string | null;
  healthcheck?: string | null;
  status: string;
  pid?: number | null;
};

export type ExtensionDiagnostic = {
  level: string;
  summary: string;
  detail?: string | null;
  at: string;
};

export type ExtensionRuntimeExtension = {
  id: string;
  name: string;
  kind: string;
  version: string;
  source_mode: "workspace" | "package";
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  frontend?: ResolvedFrontendEntry | null;
  backend?: ResolvedBackendEntry | null;
  capabilities: {
    pages: boolean;
    panels: boolean;
    themes: boolean;
    locales: boolean;
    commands: boolean;
    providers: boolean;
    hooks: boolean;
  };
  pages: ExtensionPageContribution["page"][];
  panels: ExtensionPanelContribution["panel"][];
  themes: ExtensionThemeContribution["theme"][];
  locales: ExtensionLocaleContribution["locale"][];
  commands: ExtensionCommandContribution["command"][];
  providers: ExtensionProviderContribution["provider"][];
  hooks: ExtensionHookContribution["hook"][];
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionRuntimeSnapshot = {
  generation: number;
  updated_at: string;
  extensions: ExtensionRuntimeExtension[];
  pages: ExtensionPageContribution[];
  panels: ExtensionPanelContribution[];
  themes: ExtensionThemeContribution[];
  locales: ExtensionLocaleContribution[];
  commands: ExtensionCommandContribution[];
  providers: ExtensionProviderContribution[];
  hooks: ExtensionHookContribution[];
};

export function sortExtensionPages(
  pages: ExtensionPageContribution[],
  locale: string,
  resolveTitle: (value: LocalizedText) => string,
) {
  return [...pages].sort((left, right) =>
    resolveTitle(left.page.title).localeCompare(resolveTitle(right.page.title), locale),
  );
}

export function groupPanelsBySlot(panels: ExtensionPanelContribution[]) {
  const grouped: Record<PanelSlot, ExtensionPanelContribution[]> = {
    left: [],
    right: [],
    bottom: [],
    main: [],
  };

  for (const panel of panels) {
    const slot = normalizePanelSlot(panel.panel.slot);
    grouped[slot].push(panel);
  }

  return grouped;
}

export function normalizePanelSlot(slot: string): PanelSlot {
  if (slot === "left" || slot === "right" || slot === "bottom") {
    return slot;
  }

  return "main";
}
