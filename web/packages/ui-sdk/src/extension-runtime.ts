export type LocalizedText = {
  key: string;
  fallback: string;
};

export type ExtensionPageContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
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
