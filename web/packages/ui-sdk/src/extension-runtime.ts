export type LocalizedText = {
  key: string;
  fallback: string;
};

export type RegisteredContributionBase = {
  extension_id: string;
  source_mode: "dev" | "package";
  install_dir: string;
};

export type ExtensionSettingValue = string | number | boolean;

export type ExtensionSettingField = {
  key: string;
  label: LocalizedText;
  description?: LocalizedText | null;
  type: "text" | "textarea" | "boolean" | "select" | "number";
  placeholder?: string | null;
  required: boolean;
  options: Array<{
    label: LocalizedText;
    value: string;
  }>;
  default_value?: ExtensionSettingValue | null;
};

export type ExtensionView = {
  name: string;
  type: "page" | "panel" | string;
  title: LocalizedText;
  nav?: string | null;
  order?: number | null;
  slot?: string | null;
  icon?: string | null;
  route?: string | null;
  priority?: number | null;
};

export type ExtensionProviderSpec = {
  kind: string;
  interfaces: string[];
  model_discovery: boolean;
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

export type ExtensionOperation = {
  name: string;
  title?: LocalizedText | null;
  description?: string | null;
  agent: boolean;
  input?: string | null;
  output?: string | null;
  provider?: ExtensionProviderSpec | null;
  schedule: boolean;
};

export type ExtensionEvent = {
  on: string;
  operation: string;
};

export type ExtensionViewContribution = RegisteredContributionBase & {
  view: ExtensionView;
};

export type ExtensionOperationContribution = RegisteredContributionBase & {
  operation: ExtensionOperation;
};

export type ExtensionEventContribution = RegisteredContributionBase & {
  event: ExtensionEvent;
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

export type ExtensionMessageRendererContribution = RegisteredContributionBase & {
  renderer: {
    id: string;
    format: string;
    mount: string;
    priority: number;
  };
};

export type PanelSlot = "left" | "right" | "bottom" | "main";

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

export type ExtensionConversationRecord = {
  id: string;
  extension_id: string;
  namespace: string;
  scope_type: string;
  scope_id: string;
  kind: string;
  status?: string | null;
  title?: string | null;
  summary?: string | null;
  payload: unknown;
  related_message_id?: string | null;
  parent_id?: string | null;
  created_at: string;
  updated_at: string;
  closed_at?: string | null;
};

export type ExtensionConversationRecordMountContext = ExtensionViewMountContext & {
  kind: "conversation_record";
  conversationId: string;
  record: ExtensionConversationRecord;
};

export type ExtensionMessageRenderRequest = {
  body: string;
  format: string;
  role: "operator" | "agent" | "system" | "tool";
  agents: Array<{ id: string; display_name: string }>;
  skills: Array<{ id: string }>;
  mentionAgentIds: string[];
};

export type ExtensionMessageRendererMountContext = ExtensionViewMountContext & {
  kind: "message_renderer";
  renderer: ExtensionMessageRendererContribution;
  request: ExtensionMessageRenderRequest;
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

export type ExtensionConversationRecordMount = (
  container: HTMLElement,
  context: ExtensionConversationRecordMountContext,
) => void | ExtensionViewHandle | Promise<void | ExtensionViewHandle>;

export type ExtensionMessageRendererMount = (
  container: HTMLElement,
  context: ExtensionMessageRendererMountContext,
) => void | ExtensionViewHandle | Promise<void | ExtensionViewHandle>;

export type ExtensionUiModule = {
  pages?: Record<string, ExtensionPageMount>;
  panels?: Record<string, ExtensionPanelMount>;
  conversationRecords?: Record<string, ExtensionConversationRecordMount>;
  messageRenderers?: Record<string, ExtensionMessageRendererMount>;
};

export type ExtensionProviderContribution = RegisteredContributionBase & {
  provider: ExtensionProviderSpec & {
    id: string;
    entry?: string | null;
    extension_id?: string | null;
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

export type ExtensionActionContribution = RegisteredContributionBase & {
  action: {
    action: string;
    operation: string;
    method: string;
    phase: "before" | "execute" | "after_success" | "after_error";
    priority: number;
    enabled: boolean;
    result_mode: "void" | "first" | "last" | "collect" | "merge";
    when: unknown;
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

export type ExtensionDiagnostic = {
  level: string;
  summary: string;
  detail?: string | null;
  at: string;
};

export type ExtensionRuntimeExtension = {
  id: string;
  version?: string | null;
  name: string;
  description: string;
  docs?: string | null;
  compat: {
    ennoia?: string | null;
  };
  conversation: {
    visible: boolean;
    resources: string[];
    operations: string[];
  };
  source_mode: "dev" | "package";
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  views: ExtensionView[];
  operations: ExtensionOperation[];
  events: ExtensionEvent[];
  pages: ExtensionPageContribution["page"][];
  panels: ExtensionPanelContribution["panel"][];
  themes: ExtensionThemeContribution["theme"][];
  locales: ExtensionLocaleContribution["locale"][];
  message_renderers: ExtensionMessageRendererContribution["renderer"][];
  settings: ExtensionSettingField[];
  providers: ExtensionProviderContribution["provider"][];
  behaviors: ExtensionBehaviorContribution["behavior"][];
  memories: ExtensionMemoryContribution["memory"][];
  hooks: ExtensionHookContribution["hook"][];
  actions: ExtensionActionContribution["action"][];
  schedule_actions: ExtensionScheduleActionContribution["schedule_action"][];
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionRuntimeSnapshot = {
  generation: number;
  updated_at: string;
  extensions: ExtensionRuntimeExtension[];
  views: ExtensionViewContribution[];
  operations: ExtensionOperationContribution[];
  events: ExtensionEventContribution[];
  pages: ExtensionPageContribution[];
  panels: ExtensionPanelContribution[];
  themes: ExtensionThemeContribution[];
  locales: ExtensionLocaleContribution[];
  message_renderers: ExtensionMessageRendererContribution[];
  providers: ExtensionProviderContribution[];
  behaviors: ExtensionBehaviorContribution[];
  memories: ExtensionMemoryContribution[];
  hooks: ExtensionHookContribution[];
  actions: ExtensionActionContribution[];
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
