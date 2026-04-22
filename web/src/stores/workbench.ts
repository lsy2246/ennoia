import { create } from "zustand";

export type WorkbenchViewKind = "agent" | "api-channel" | "session";

export type WorkbenchViewDescriptor = {
  panelId: string;
  kind: WorkbenchViewKind;
  entityId: string;
  title: string;
  subtitle?: string;
  openedAt: number;
};

type DockviewApiLike = {
  addPanel: (options: Record<string, unknown>) => unknown;
  removePanel: (panel: unknown) => void;
  getPanel?: (id: string) => any;
  setActivePanel?: (panel: unknown) => void;
  activePanel?: { id: string } | null;
};

type OpenViewOptions = {
  placement?: "right" | "below";
  reuseOpenInstance?: boolean;
};

type WorkbenchState = {
  api: DockviewApiLike | null;
  openViews: WorkbenchViewDescriptor[];
  recentViews: WorkbenchViewDescriptor[];
  registerApi: (api: DockviewApiLike) => void;
  openView: (
    descriptor: Omit<WorkbenchViewDescriptor, "panelId" | "openedAt">,
    options?: OpenViewOptions,
  ) => void;
  focusView: (panelId: string) => void;
  closeView: (panelId: string) => void;
  restoreView: (panelId: string) => void;
  markClosed: (panelId: string) => void;
  resetLayout: () => void;
};

function buildPanelId(kind: WorkbenchViewKind, entityId: string) {
  const suffix = Date.now().toString(36);
  return `${kind}:${entityId}:${suffix}`;
}

function sameResource(
  left: Pick<WorkbenchViewDescriptor, "kind" | "entityId">,
  right: Pick<WorkbenchViewDescriptor, "kind" | "entityId">,
) {
  return left.kind === right.kind && left.entityId === right.entityId;
}

export const useWorkbenchStore = create<WorkbenchState>((set, get) => ({
  api: null,
  openViews: [],
  recentViews: [],

  registerApi(api) {
    set({ api });
  },

  openView(descriptor, options) {
    const state = get();
    if (!state.api) {
      return;
    }

    if (options?.reuseOpenInstance) {
      const existing = state.openViews.find((item) => sameResource(item, descriptor));
      const existingPanel = existing ? state.api.getPanel?.(existing.panelId) : null;
      if (existingPanel) {
        state.api.setActivePanel?.(existingPanel);
        return;
      }
    }

    const panelId = buildPanelId(descriptor.kind, descriptor.entityId);
    const view: WorkbenchViewDescriptor = {
      ...descriptor,
      panelId,
      openedAt: Date.now(),
    };

    state.api.addPanel({
      id: panelId,
      title: descriptor.title,
      component: "resource",
      params: {
        descriptor: view,
      },
      position: {
        referencePanel: state.api.activePanel?.id ?? "main",
        direction: options?.placement ?? "right",
      },
    });

    set({
      openViews: [...state.openViews, view],
      recentViews: state.recentViews.filter((item) => !sameResource(item, view)),
    });
  },

  focusView(panelId) {
    const panel = get().api?.getPanel?.(panelId);
    if (panel) {
      get().api?.setActivePanel?.(panel);
    }
  },

  closeView(panelId) {
    const panel = get().api?.getPanel?.(panelId);
    if (panel) {
      get().api?.removePanel(panel);
    }
    get().markClosed(panelId);
  },

  restoreView(panelId) {
    const state = get();
    const view = state.recentViews.find((item) => item.panelId === panelId);
    if (!view) {
      return;
    }
    state.openView({
      kind: view.kind,
      entityId: view.entityId,
      title: view.title,
      subtitle: view.subtitle,
    });
  },

  markClosed(panelId) {
    const state = get();
    const removed = state.openViews.find((item) => item.panelId === panelId);
    set({
      openViews: state.openViews.filter((item) => item.panelId !== panelId),
      recentViews: removed
        ? [removed, ...state.recentViews.filter((item) => item.panelId !== panelId)].slice(0, 16)
        : state.recentViews,
    });
  },

  resetLayout() {
    const state = get();
    for (const view of state.openViews) {
      const panel = state.api?.getPanel?.(view.panelId);
      if (panel) {
        state.api?.removePanel(panel);
      }
    }
    set({
      openViews: [],
      recentViews: [...state.openViews, ...state.recentViews].slice(0, 16),
    });
  },
}));
