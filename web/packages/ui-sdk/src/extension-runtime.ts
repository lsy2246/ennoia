export type LocalizedText = {
  key: string;
  fallback: string;
};

export type RegisteredContributionBase = {
  extension_id: string;
  extension_kind: string;
  source_mode: "dev" | "package";
  install_dir: string;
};

export type ExtensionResourceType = {
  id: string;
  title?: LocalizedText | null;
  content_kind: string;
  metadata_schema?: string | null;
  content_schema?: string | null;
  operations: string[];
  tags: string[];
};

export type ExtensionCapability = {
  id: string;
  contract: string;
  kind: string;
  title?: LocalizedText | null;
  runtime?: string | null;
  entry?: string | null;
  input_schema?: string | null;
  output_schema?: string | null;
  consumes: string[];
  produces: string[];
  requires: string[];
  emits: string[];
  metadata: unknown;
};

export type ExtensionSurface = {
  id: string;
  kind: string;
  mount: string;
  title?: LocalizedText | null;
  route?: string | null;
  slot?: string | null;
  icon?: string | null;
  nav?: {
    default_pinned?: boolean;
    order?: number | null;
  } | null;
  match_resource_types: string[];
  match_capability_contracts: string[];
  priority?: number | null;
};

export type ExtensionSubscription = {
  event: string;
  capability: string;
  match_resource_types: string[];
  match_capability_ids: string[];
  match_capability_contracts: string[];
};

export type ExtensionResourceTypeContribution = RegisteredContributionBase & {
  resource_type: ExtensionResourceType;
};

export type ExtensionCapabilityContribution = RegisteredContributionBase & {
  capability: ExtensionCapability;
};

export type ExtensionSurfaceContribution = RegisteredContributionBase & {
  surface: ExtensionSurface;
};

export type ExtensionSubscriptionContribution = RegisteredContributionBase & {
  subscription: ExtensionSubscription;
};

export type ExtensionPageContribution = RegisteredContributionBase & {
  page: {
    id: string;
    title: LocalizedText;
    route: string;
    mount: string;
    icon?: string | null;
    nav?: {
      default_pinned?: boolean;
      order?: number | null;
    } | null;
  };
};

export type ExtensionPanelContribution = RegisteredContributionBase & {
  panel: {
    id: string;
    title: LocalizedText;
    mount: string;
    slot: string;
    icon?: string | null;
  };
};

export type ThemeAppearance = "light" | "dark" | "system" | "high-contrast";

export type ExtensionThemeContribution = RegisteredContributionBase & {
  theme: {
    id: string;
    label: LocalizedText;
    appearance: ThemeAppearance;
    tokens_entry: string;
    contract?: string | null;
    preview_color?: string | null;
    extends?: string | null;
    category?: string | null;
  };
};

export type ExtensionLocaleContribution = RegisteredContributionBase & {
  locale: {
    locale: string;
    namespace: string;
    entry: string;
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

export type ExtensionUiRenderHelpers = {
  locale: string;
  themeId: string;
  apiBaseUrl: string;
  t: (key: string, fallback: string) => string;
  formatDateTime: (value: string | number | Date) => string;
  formatDate: (value: string | number | Date) => string;
  formatTime: (value: string | number | Date) => string;
};

export type ExtensionViewMountContext = {
  extensionId: string;
  mount: string;
  helpers: ExtensionUiRenderHelpers;
};

export type ExtensionPageMountContext = ExtensionViewMountContext & {
  kind: "page";
  page: ExtensionPageContribution;
};

export type ExtensionPanelMountContext = ExtensionViewMountContext & {
  kind: "panel";
  panel: ExtensionPanelContribution;
};

export type ExtensionViewHandle = {
  unmount?: () => void | Promise<void>;
};

export type ExtensionPageMount = (
  container: HTMLElement,
  context: ExtensionPageMountContext,
) => void | ExtensionViewHandle | Promise<void | ExtensionViewHandle>;

export type ExtensionPanelMount = (
  container: HTMLElement,
  context: ExtensionPanelMountContext,
) => void | ExtensionViewHandle | Promise<void | ExtensionViewHandle>;

export type ExtensionUiModule = {
  pages?: Record<string, ExtensionPageMount>;
  panels?: Record<string, ExtensionPanelMount>;
};

export type ExtensionCommandContribution = RegisteredContributionBase & {
  command: {
    id: string;
    title: LocalizedText;
    action: string;
    shortcut?: string | null;
  };
};

export type ExtensionProviderContribution = RegisteredContributionBase & {
  provider: {
    id: string;
    kind: string;
    entry?: string | null;
    extension_id?: string | null;
    interfaces: string[];
    model_discovery: boolean;
    recommended_model?: string | null;
    manual_model: boolean;
    generation_options: {
      id: string;
      label: LocalizedText;
      value_type: string;
      required: boolean;
      default_value?: string | null;
      allowed_values: string[];
    }[];
  };
};

export type ExtensionHookContribution = RegisteredContributionBase & {
  hook: {
    event: string;
    handler?: string | null;
  };
};

export type ExtensionBehaviorContribution = RegisteredContributionBase & {
  behavior: {
    id: string;
    extension_id?: string | null;
    interfaces: string[];
    entry?: string | null;
  };
};

export type ExtensionMemoryContribution = RegisteredContributionBase & {
  memory: {
    id: string;
    extension_id?: string | null;
    interfaces: string[];
    entry?: string | null;
  };
};

export type ExtensionInterfaceContribution = RegisteredContributionBase & {
  interface: {
    key: string;
    method: string;
    schema?: string | null;
  };
};

export type ExtensionScheduleActionContribution = RegisteredContributionBase & {
  schedule_action: {
    id: string;
    method: string;
    title?: LocalizedText | null;
    schema?: string | null;
  };
};

export type ResolvedUiEntry = {
  kind: string;
  entry: string;
  hmr: boolean;
  version: string;
};

export type ResolvedWorkerEntry = {
  kind: string;
  entry: string;
  abi: string;
  protocol?: string | null;
  status: string;
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
  description: string;
  docs?: string | null;
  links: {
    label: string;
    target: string;
  }[];
  examples: {
    title: string;
    summary?: string | null;
    input_hint?: string | null;
  }[];
  kind: string;
  source_mode: "dev" | "package";
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  ui?: ResolvedUiEntry | null;
  worker?: ResolvedWorkerEntry | null;
  permissions: {
    storage?: string | null;
    sqlite: boolean;
    network: string[];
    events: string[];
    fs: string[];
    env: string[];
  };
  runtime: {
    startup: string;
    timeout_ms: number;
    memory_limit_mb: number;
  };
  capabilities: {
    resource_types: boolean;
    capabilities: boolean;
    surfaces: boolean;
    locales: boolean;
    themes: boolean;
    commands: boolean;
    subscriptions: boolean;
  };
  resource_types: ExtensionResourceType[];
  capability_rows: ExtensionCapability[];
  surfaces: ExtensionSurface[];
  pages: ExtensionPageContribution["page"][];
  panels: ExtensionPanelContribution["panel"][];
  themes: ExtensionThemeContribution["theme"][];
  locales: ExtensionLocaleContribution["locale"][];
  commands: ExtensionCommandContribution["command"][];
  providers: ExtensionProviderContribution["provider"][];
  behaviors: ExtensionBehaviorContribution["behavior"][];
  memories: ExtensionMemoryContribution["memory"][];
  hooks: ExtensionHookContribution["hook"][];
  interfaces: ExtensionInterfaceContribution["interface"][];
  schedule_actions: ExtensionScheduleActionContribution["schedule_action"][];
  subscriptions: ExtensionSubscription[];
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionRuntimeSnapshot = {
  generation: number;
  updated_at: string;
  extensions: ExtensionRuntimeExtension[];
  resource_types: ExtensionResourceTypeContribution[];
  capabilities: ExtensionCapabilityContribution[];
  surfaces: ExtensionSurfaceContribution[];
  subscriptions: ExtensionSubscriptionContribution[];
  pages: ExtensionPageContribution[];
  panels: ExtensionPanelContribution[];
  themes: ExtensionThemeContribution[];
  locales: ExtensionLocaleContribution[];
  commands: ExtensionCommandContribution[];
  providers: ExtensionProviderContribution[];
  behaviors: ExtensionBehaviorContribution[];
  memories: ExtensionMemoryContribution[];
  hooks: ExtensionHookContribution[];
  interfaces: ExtensionInterfaceContribution[];
  schedule_actions: ExtensionScheduleActionContribution[];
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
