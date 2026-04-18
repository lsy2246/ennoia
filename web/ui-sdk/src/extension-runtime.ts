export type ExtensionPageContribution = {
  extension_id: string;
  extension_kind: string;
  extension_version: string;
  install_dir: string;
  page: {
    id: string;
    title: string;
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
    title: string;
    mount: string;
    slot: string;
    icon?: string | null;
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

export function sortExtensionPages(pages: ExtensionPageContribution[]) {
  return [...pages].sort((left, right) =>
    left.page.title.localeCompare(right.page.title, "zh-CN"),
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
