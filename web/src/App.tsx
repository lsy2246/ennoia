import { useCallback, useMemo, useState, useEffect, useRef } from "react";
import {
  DockviewDefaultTab,
  DockviewReact,
  positionToDirection,
  type IDockviewPanel,
  type DockviewDidDropEvent,
  type DockviewDndOverlayEvent,
  type IDockviewPanelProps,
  type IWatermarkPanelProps,
} from "dockview";

import { getApiBaseUrl } from "@ennoia/api-client";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { AgentEditorView } from "@/views/agents/Editor";
import { ApiChannelEditorView } from "@/views/providers/Editor";
import { SessionView } from "@/views/conversations/Session";
import { useWorkbenchStore, type WorkbenchViewDescriptor } from "@/stores/workbench";
import { useSessionCommandsStore } from "@/stores/sessionCommands";
import { CommandPalette, type CommandPaletteAction } from "@/components/layout/CommandPalette";
import {
  OmniDock,
  ENNOIA_ROUTE_DRAG_MIME,
  getActiveDraggedNavItem,
  type DockPosition,
  type NavItem,
} from "@/components/layout/OmniDock";
import { Agents } from "@/pages/agents";
import { Conversations } from "@/pages/conversations";
import { Extensions } from "@/pages/extensions";
import { Observability } from "@/pages/observability";
import { Schedules } from "@/pages/schedules";
import { Settings } from "@/pages/settings";
import { Skills } from "@/pages/skills";
import { ExtensionPageView } from "@/views/extensions/Page";

type ResourcePanelParams = {
  panelKind?: "resource";
  descriptor: WorkbenchViewDescriptor;
};

type RoutePanelParams = {
  panelKind?: "route";
  routeId: string;
  href: string;
  label: string;
  source: NavItem["source"];
};

type LocalWorkbenchPreferences = {
  dockPosition: DockPosition;
  dockExpanded: boolean;
  layout?: unknown;
  mobileActiveNavId?: string;
};

const LOCAL_WORKBENCH_PREFERENCES_KEY = "ennoia.web.workbench.v2";
const WORKBENCH_EMPTY_PRIMARY_IDS = ["conversations", "agents", "skills", "settings"] as const;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isRoutePanelParams(value: unknown): value is RoutePanelParams {
  return isRecord(value)
    && typeof value.routeId === "string"
    && typeof value.href === "string"
    && (value.source === "builtin" || value.source === "extension");
}

function isResourcePanelParams(value: unknown): value is ResourcePanelParams {
  return isRecord(value)
    && isRecord(value.descriptor)
    && typeof value.descriptor.kind === "string"
    && typeof value.descriptor.entityId === "string"
    && typeof value.descriptor.title === "string";
}

function readWorkbenchPreferences(): LocalWorkbenchPreferences {
  if (typeof window === "undefined") {
    return {
      dockPosition: "left",
      dockExpanded: false,
    };
  }

  try {
    const parsed = JSON.parse(localStorage.getItem(LOCAL_WORKBENCH_PREFERENCES_KEY) ?? "{}") as Partial<LocalWorkbenchPreferences>;
    return {
      dockPosition: parsed.dockPosition ?? "left",
      dockExpanded: parsed.dockExpanded ?? false,
      layout: parsed.layout,
      mobileActiveNavId: parsed.mobileActiveNavId,
    };
  } catch {
    return {
      dockPosition: "left",
      dockExpanded: false,
    };
  }
}

function writeWorkbenchPreferences(preferences: LocalWorkbenchPreferences) {
  if (typeof window === "undefined") {
    return;
  }

  localStorage.setItem(LOCAL_WORKBENCH_PREFERENCES_KEY, JSON.stringify(preferences));
}

function useIsMobileViewport() {
  const [isMobile, setIsMobile] = useState(() =>
    typeof window !== "undefined" ? window.innerWidth <= 1100 : false,
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }

    const media = window.matchMedia("(max-width: 1100px)");
    const sync = () => setIsMobile(media.matches);
    sync();
    if (typeof media.addEventListener === "function") {
      media.addEventListener("change", sync);
      return () => media.removeEventListener("change", sync);
    }
    media.addListener(sync);
    return () => media.removeListener(sync);
  }, []);

  return isMobile;
}

const BUILTIN_NAV = [
  { id: "conversations", href: "/conversations", icon: "⌘", labelKey: "web.nav.conversations", fallback: "会话", hintKey: "web.nav.conversations_hint", hint: "统一发起 direct 和 group conversation" },
  { id: "agents", href: "/agents", icon: "A", labelKey: "web.nav.agents", fallback: "Agents", hintKey: "web.nav.agents_hint", hint: "多开 Agent 编辑视图" },
  { id: "skills", href: "/skills", icon: "S", labelKey: "web.nav.skills", fallback: "技能", hintKey: "web.nav.skills_hint", hint: "发现并分配技能给 Agent" },
  { id: "schedules", href: "/schedules", icon: "T", labelKey: "web.nav.schedules", fallback: "定时器", hintKey: "web.nav.schedules_hint", hint: "管理定时触发的命令、Agent 与会话投递" },
  { id: "extensions", href: "/extensions", icon: "E", labelKey: "web.nav.extensions", fallback: "扩展", hintKey: "web.nav.extensions_hint", hint: "系统插件与贡献能力" },
  { id: "observability", href: "/observability", icon: "L", labelKey: "web.nav.observability", fallback: "观测", hintKey: "web.nav.observability_hint", hint: "统一日志与 trace 观测台" },
  { id: "settings", href: "/settings", icon: "⚙", labelKey: "web.nav.settings", fallback: "设置", hintKey: "web.nav.settings_hint", hint: "运行时配置表单" },
] as const;

function RuntimeInspectorPanel() {
  const profile = useRuntimeStore((state) => state.profile);
  const locale = useUiStore((state) => state.locale);
  const themeId = useUiStore((state) => state.themeId);
  const { runtime, t } = useUiHelpers();

  return (
    <div className="dock-panel-content">
      <div className="panel-title">{t("web.panel.runtime", "运行态")}</div>
      <div className="kv-list">
        <span>{t("web.panel.operator", "操作者")}</span>
        <strong>{profile?.display_name ?? "Operator"}</strong>
        <span>{t("web.panel.api", "API")}</span>
        <strong>{getApiBaseUrl()}</strong>
        <span>{t("web.panel.locale", "语言")}</span>
        <strong>{locale}</strong>
        <span>{t("web.panel.theme", "主题")}</span>
        <strong>{themeId}</strong>
        <span>{t("web.panel.extension_pages", "扩展页面")}</span>
        <strong>{runtime?.registry.pages.length ?? 0}</strong>
        <span>{t("web.panel.extension_panels", "扩展面板")}</span>
        <strong>{runtime?.registry.panels.length ?? 0}</strong>
      </div>
    </div>
  );
}

function ExtensionPanelDeck() {
  const { runtime, resolveText, t } = useUiHelpers();
  const panels = runtime?.registry.panels ?? [];

  return (
    <div className="dock-panel-content dock-panel-content--horizontal">
      {panels.length === 0 ? (
        <div className="empty-card">{t("web.panel.no_extensions", "暂无扩展面板贡献。")}</div>
      ) : (
        panels.map((panel) => {
          return (
            <article key={`${panel.extension_id}:${panel.panel.id}`} className="mini-card">
              <strong>{resolveText(panel.panel.title)}</strong>
              <span><span className="badge badge--muted">{panel.panel.slot}</span> {panel.extension_id}</span>
              <span className="badge badge--muted">{panel.panel.mount}</span>
            </article>
          );
        })
      )}
    </div>
  );
}

function ResourceViewPanel(props: IDockviewPanelProps<ResourcePanelParams>) {
  const descriptor = props.params.descriptor;

  switch (descriptor.kind) {
    case "agent":
      return (
        <div className="resource-panel">
          <AgentEditorView agentId={descriptor.entityId} />
        </div>
      );
    case "api-channel":
      return (
        <div className="resource-panel">
          <ApiChannelEditorView channelId={descriptor.entityId} panelId={descriptor.panelId} />
        </div>
      );
    case "session":
      return (
        <div className="resource-panel">
          <SessionView sessionId={descriptor.entityId} panelId={descriptor.panelId} />
        </div>
      );
    default:
      return <div className="empty-card">{descriptor.title}</div>;
  }
}

function RoutedWorkbenchPanel(props: IDockviewPanelProps<RoutePanelParams>) {
  const routeId = props.params.routeId;
  const href = props.params.href;

  const content = (() => {
    switch (routeId) {
      case "conversations":
        return <Conversations />;
      case "agents":
        return <Agents />;
      case "skills":
        return <Skills />;
      case "schedules":
        return <Schedules />;
      case "extensions":
        return <Extensions />;
      case "observability":
        return <Observability />;
      case "settings":
        return <Settings />;
      default:
        return href.startsWith("/extension-pages/")
          ? <ExtensionPageView pageId={decodeURIComponent(href.replace("/extension-pages/", ""))} />
          : <div className="empty-card">{props.params.label}</div>;
    }
  })();

  return (
    <div className="route-panel-scroll">
      <div className="route-panel-transition">{content}</div>
    </div>
  );
}

type WorkbenchEmptyStateProps = {
  actions?: NavItem[];
  isMobile?: boolean;
  onOpenItem?: (item: NavItem) => void;
};

function WorkbenchEmptyState({ actions = [], isMobile = false, onOpenItem }: WorkbenchEmptyStateProps) {
  const { t } = useUiHelpers();

  return (
    <section className="workbench-empty-state">
      <div className="workbench-empty-hero">
        <span>{t("web.brand.subtitle", "Local multi-agent Web workbench")}</span>
        <h1>
          {isMobile
            ? t("web.workbench.empty_mobile_title", "从底部导航打开第一个视图。")
            : t("web.workbench.empty_title", "从导航栏开启你的第一个工作区。")}
        </h1>
        <p>
          {isMobile
            ? t("web.workbench.empty_mobile_description", "移动端保持单页面浏览，你可以先打开会话、Agents、技能或偏好设置。")
            : t("web.workbench.empty_description", "点击导航会进入当前活跃视图，拖拽到画面的上、下、左、右边缘可以创建新的分区。")}
        </p>
      </div>

      {actions.length > 0 ? (
        <div className="workbench-empty-actions" role="list" aria-label={t("web.workbench.quick_actions", "快速入口")}>
          {actions.map((item) => (
            <button
              key={`${item.source}:${item.id}`}
              type="button"
              className="workbench-empty-action"
              onClick={() => onOpenItem?.(item)}
            >
              <strong>{item.label}</strong>
              <span>{item.hint}</span>
            </button>
          ))}
        </div>
      ) : null}

      <div className="workbench-empty-tips">
        <article className="workbench-empty-tip">
          <strong>{t("web.workbench.tip_focus", "轻量切换")}</strong>
          <span>{t("web.workbench.tip_focus_desc", "直接点击导航时，会替换当前活跃视图，不会不断堆叠新窗口。")}</span>
        </article>
        <article className="workbench-empty-tip">
          <strong>{t("web.workbench.tip_split", "四向分区")}</strong>
          <span>{t("web.workbench.tip_split_desc", "把导航项拖到上下左右预览区后放开，就会生成新的工作分区。")}</span>
        </article>
        <article className="workbench-empty-tip">
          <strong>{t("web.workbench.tip_preferences", "偏好保持")}</strong>
          <span>{t("web.workbench.tip_preferences_desc", "导航位置、展开状态和工作台布局都会保存在本地，下次会继续沿用。")}</span>
        </article>
      </div>
    </section>
  );
}

function WorkbenchTab(props: any) {
  const markClosed = useWorkbenchStore((state) => state.markClosed);
  const panelId = props.api?.id as string | undefined;
  const isFixedPanel = panelId === "inspector" || panelId === "extensionPanels";

  return (
    <DockviewDefaultTab
      {...props}
      hideClose={props.hideClose || isFixedPanel}
      closeActionOverride={
        isFixedPanel
          ? undefined
          : () => {
              if (panelId) {
                markClosed(panelId);
              }
              props.api?.close?.();
            }
      }
    />
  );
}

export function App() {
  const { describeAppliedTheme, resolveText, runtime, t } = useUiHelpers();
  const uiState = useUiStore();
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const sessionCommandItems = useSessionCommandsStore((state) => state.items);
  const registerApi = useWorkbenchStore((state) => state.registerApi);
  const updateViewDescriptor = useWorkbenchStore((state) => state.updateViewDescriptor);
  const initialWorkbenchPreferences = useMemo(() => readWorkbenchPreferences(), []);
  const isMobileViewport = useIsMobileViewport();
  const dockThemeClass = describeAppliedTheme(uiState.themeId).appearance === "light"
    ? "dockview-theme-light"
    : "dockview-theme-dark";
  const [dockPosition, setDockPosition] = useState<DockPosition>(initialWorkbenchPreferences.dockPosition);
  const [dockExpanded, setDockExpanded] = useState(initialWorkbenchPreferences.dockExpanded);
  const [activeNavId, setActiveNavId] = useState<string | undefined>(initialWorkbenchPreferences.mobileActiveNavId);
  const [activePanelId, setActivePanelId] = useState<string | undefined>();
  const [hasDesktopViews, setHasDesktopViews] = useState(false);
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const dockPositionRef = useRef(dockPosition);
  const dockExpandedRef = useRef(dockExpanded);
  const pendingDropOpenRef = useRef<number | null>(null);
  const pendingLayoutPersistRef = useRef<number | null>(null);
  const restoringLayoutRef = useRef(false);

  const navItems = useMemo<NavItem[]>(() => {
    const builtins = BUILTIN_NAV.map((item) => ({
      id: item.id,
      href: item.href,
      icon: item.icon,
      label: t(item.labelKey, item.fallback),
      hint: t(item.hintKey, item.hint),
      source: "builtin" as const,
    }));
    const extensionItems =
      runtime?.registry.pages
        .filter((page) => page.page.nav?.default_pinned)
        .sort((left, right) => (left.page.nav?.order ?? 1000) - (right.page.nav?.order ?? 1000))
        .map((page) => ({
          id: page.page.id,
          href: `/extension-pages/${encodeURIComponent(page.page.id)}`,
          icon: page.page.icon?.slice(0, 1).toUpperCase() ?? "↗",
          label: resolveText(page.page.title),
          hint: t("web.nav.extension_hint", "扩展贡献视图"),
          source: "extension" as const,
        })) ?? [];
    return [...builtins, ...extensionItems];
  }, [resolveText, runtime?.registry.pages, t]);

  const activeNavItem = useMemo(
    () => navItems.find((item) => item.id === activeNavId),
    [activeNavId, navItems],
  );

  const emptyStateActions = useMemo(() => {
    const order = new Map<string, number>(WORKBENCH_EMPTY_PRIMARY_IDS.map((id, index) => [id, index]));
    return navItems
      .filter((item) => order.has(item.id))
      .sort((left, right) => (order.get(left.id) ?? 0) - (order.get(right.id) ?? 0));
  }, [navItems]);

  const resolveRouteLabel = useCallback((params: Pick<RoutePanelParams, "routeId" | "href" | "source" | "label">) => {
    return navItems.find((item) =>
      item.id === params.routeId
      && item.href === params.href
      && item.source === params.source,
    )?.label
      ?? navItems.find((item) => item.id === params.routeId && item.source === params.source)?.label
      ?? navItems.find((item) => item.href === params.href)?.label
      ?? params.label;
  }, [navItems]);

  const resolveDescriptorTitle = useCallback((descriptor: WorkbenchViewDescriptor) => {
    return descriptor.titleKey
      ? t(descriptor.titleKey, descriptor.titleFallback ?? descriptor.title)
      : descriptor.title;
  }, [t]);

  const resolveDescriptorSubtitle = useCallback((descriptor: WorkbenchViewDescriptor) => {
    if (!descriptor.subtitleKey) {
      return descriptor.subtitle;
    }
    return t(descriptor.subtitleKey, descriptor.subtitleFallback ?? descriptor.subtitle ?? "");
  }, [t]);

  const activeSessionCommands = activePanelId ? sessionCommandItems[activePanelId] : undefined;

  const normalizeDescriptorLocalization = useCallback((descriptor: WorkbenchViewDescriptor) => {
    if (descriptor.titleKey) {
      return descriptor;
    }

    if (descriptor.kind === "agent" && descriptor.entityId.startsWith("new-")) {
      return {
        ...descriptor,
        titleKey: "web.agents.new",
        titleFallback: "新建 Agent",
        subtitleKey: descriptor.subtitle ? "web.agents.edit" : descriptor.subtitleKey,
        subtitleFallback: descriptor.subtitle ? "编辑 Agent" : descriptor.subtitleFallback,
      };
    }

    if (descriptor.kind === "api-channel" && descriptor.entityId.startsWith("new-")) {
      return {
        ...descriptor,
        titleKey: "web.channels.new",
        titleFallback: "新建渠道",
        subtitleKey: descriptor.subtitle ? "web.channels.edit" : descriptor.subtitleKey,
        subtitleFallback: descriptor.subtitle ? "编辑 API 上游渠道" : descriptor.subtitleFallback,
      };
    }

    return descriptor;
  }, []);

  const dockComponents = useMemo(
    () => ({
      inspector: RuntimeInspectorPanel,
      extensionPanels: ExtensionPanelDeck,
      resource: ResourceViewPanel,
      route: RoutedWorkbenchPanel,
    }),
    [],
  );

  const resolveNavDragPayload = (dataTransfer: DataTransfer | null): NavItem | null => {
    const raw = dataTransfer?.getData(ENNOIA_ROUTE_DRAG_MIME);
    if (!raw) {
      return getActiveDraggedNavItem();
    }
    try {
      return JSON.parse(raw) as NavItem;
    } catch {
      return getActiveDraggedNavItem();
    }
  };

  const buildPanelPosition = (event: DockviewDidDropEvent) => {
    if (event.group) {
      return {
        referenceGroup: event.group.id,
        direction: positionToDirection(event.position),
      } as const;
    }

    return {
      direction: positionToDirection(event.position),
    } as const;
  };

  const openRoutePanel = useCallback((payload: NavItem, options?: { panelPosition?: ReturnType<typeof buildPanelPosition> }) => {
    if (isMobileViewport) {
      setActiveNavId(payload.id);
      return;
    }

    if (!workbenchApi) {
      return;
    }

    if (!options?.panelPosition) {
      const activePanel = workbenchApi.activePanel as IDockviewPanel | undefined;
      if (activePanel && isRoutePanelParams(activePanel.params)) {
        activePanel.api.setTitle(payload.label);
        activePanel.update({
          params: {
            panelKind: "route",
            routeId: payload.id,
            href: payload.href,
            label: payload.label,
            source: payload.source,
          } satisfies RoutePanelParams,
        });
        setActiveNavId(payload.id);
        return;
      }
    }

    workbenchApi.addPanel({
      id: `route:${payload.source}:${payload.id}:${Date.now().toString(36)}`,
      title: payload.label,
      component: "route",
      params: {
        panelKind: "route",
        routeId: payload.id,
        href: payload.href,
        label: payload.label,
        source: payload.source,
      } satisfies RoutePanelParams,
      position: options?.panelPosition,
    });
    setHasDesktopViews(true);
    setActiveNavId(payload.id);
  }, [isMobileViewport, workbenchApi]);

  const scheduleOpenRoutePanel = useCallback((payload: NavItem, options?: { panelPosition?: ReturnType<typeof buildPanelPosition> }) => {
    if (typeof window === "undefined") {
      openRoutePanel(payload, options);
      return;
    }

    if (pendingDropOpenRef.current !== null) {
      window.clearTimeout(pendingDropOpenRef.current);
    }

    pendingDropOpenRef.current = window.setTimeout(() => {
      pendingDropOpenRef.current = null;
      openRoutePanel(payload, options);
    }, 0);
  }, [openRoutePanel]);

  const commandPaletteActions = useMemo<CommandPaletteAction[]>(() => {
    const navActions = navItems.map<CommandPaletteAction>((item) => ({
      id: `nav:${item.id}`,
      title: item.label,
      hint: item.hint,
      keywords: [item.id, item.href, item.source],
      run: () => openRoutePanel(item),
    }));

    const systemActions: CommandPaletteAction[] = [
      {
        id: "system:new-conversation",
        title: t("web.conversations.create_direct", "创建私聊"),
        hint: t("web.command_palette.new_conversation_hint", "打开会话页并创建新的会话"),
        keywords: ["conversation", "new", "create", "session", "chat"],
        run: () => {
          const target = navItems.find((item) => item.id === "conversations");
          if (target) {
            openRoutePanel(target);
          }
        },
      },
    ];

    const sessionActions = activeSessionCommands
      ? [
          {
            id: `session:${activeSessionCommands.sessionId}:reset`,
            title: t("web.conversations.reset_context", "清空上下文"),
            hint: activeSessionCommands.title,
            keywords: ["session", "conversation", "reset", "branch"],
            run: () => activeSessionCommands.actions.resetContext(),
          },
          {
            id: `session:${activeSessionCommands.sessionId}:checkpoint`,
            title: t("web.conversations.create_checkpoint", "创建检查点"),
            hint: activeSessionCommands.title,
            keywords: ["session", "conversation", "checkpoint"],
            run: () => activeSessionCommands.actions.createCheckpoint(),
          },
          ...activeSessionCommands.branches.map<CommandPaletteAction>((branch) => ({
            id: `session:${activeSessionCommands.sessionId}:branch:${branch.id}`,
            title: `${t("web.command_palette.switch_branch", "切换到分支")} · ${branch.name}`,
            hint: branch.kind,
            keywords: ["branch", "switch", branch.name, branch.kind],
            run: () => activeSessionCommands.actions.switchBranch(branch.id),
          })),
          ...activeSessionCommands.checkpoints.map<CommandPaletteAction>((checkpoint) => ({
            id: `session:${activeSessionCommands.sessionId}:checkpoint:${checkpoint.id}`,
            title: `${t("web.command_palette.branch_from_checkpoint", "从检查点创建分支")} · ${checkpoint.label}`,
            hint: checkpoint.kind,
            keywords: ["checkpoint", "branch", checkpoint.label, checkpoint.kind],
            run: () => activeSessionCommands.actions.branchFromCheckpoint(checkpoint.id),
          })),
        ]
      : [];

    return [...systemActions, ...navActions, ...sessionActions];
  }, [activeSessionCommands, navItems, openRoutePanel, t]);

  const openDraggedRoutePanel = (event: DockviewDidDropEvent) => {
    const payload = resolveNavDragPayload(event.nativeEvent.dataTransfer);
    if (!payload) {
      return;
    }

    scheduleOpenRoutePanel(payload, { panelPosition: buildPanelPosition(event) });
  };

  const persistWorkbenchPreferences = useCallback((layout?: unknown) => {
    writeWorkbenchPreferences({
      dockPosition: dockPositionRef.current,
      dockExpanded: dockExpandedRef.current,
      layout: layout === undefined ? readWorkbenchPreferences().layout : layout,
      mobileActiveNavId: isMobileViewport ? activeNavId : readWorkbenchPreferences().mobileActiveNavId,
    });
  }, [activeNavId, isMobileViewport]);

  const scheduleLayoutPersistence = useCallback((api: { toJSON?: () => unknown }) => {
    if (typeof window === "undefined") {
      try {
        persistWorkbenchPreferences(api.toJSON?.());
      } catch {
        persistWorkbenchPreferences();
      }
      return;
    }

    if (pendingLayoutPersistRef.current !== null) {
      window.clearTimeout(pendingLayoutPersistRef.current);
    }

    pendingLayoutPersistRef.current = window.setTimeout(() => {
      pendingLayoutPersistRef.current = null;
      try {
        persistWorkbenchPreferences(api.toJSON?.());
      } catch {
        persistWorkbenchPreferences();
      }
    }, 48);
  }, [persistWorkbenchPreferences]);

  const syncOpenPanelTitles = useCallback((api = workbenchApi) => {
    if (!api?.panels || isMobileViewport) {
      return;
    }

    let hasChanges = false;

    for (const rawPanel of api.panels as IDockviewPanel[]) {
      if (isRoutePanelParams(rawPanel.params)) {
        const nextLabel = resolveRouteLabel(rawPanel.params);
        rawPanel.api.setTitle(nextLabel);
        if (rawPanel.params.label !== nextLabel || rawPanel.params.panelKind !== "route") {
          rawPanel.update({
            params: {
              ...rawPanel.params,
              panelKind: "route",
              label: nextLabel,
            } satisfies RoutePanelParams,
          });
          hasChanges = true;
        }
        continue;
      }

      if (!isResourcePanelParams(rawPanel.params)) {
        continue;
      }

      const descriptor = normalizeDescriptorLocalization(rawPanel.params.descriptor);
      const nextTitle = resolveDescriptorTitle(descriptor);
      const nextSubtitle = resolveDescriptorSubtitle(descriptor);
      rawPanel.api.setTitle(nextTitle);
      if (
        rawPanel.params.panelKind !== "resource"
        || rawPanel.params.descriptor.titleKey !== descriptor.titleKey
        || rawPanel.params.descriptor.titleFallback !== descriptor.titleFallback
        || rawPanel.params.descriptor.subtitleKey !== descriptor.subtitleKey
        || rawPanel.params.descriptor.subtitleFallback !== descriptor.subtitleFallback
        || descriptor.title !== nextTitle
        || descriptor.subtitle !== nextSubtitle
      ) {
        const nextDescriptor = {
          ...descriptor,
          title: nextTitle,
          subtitle: nextSubtitle,
        };
        rawPanel.update({
          params: {
            panelKind: "resource",
            descriptor: nextDescriptor,
          } satisfies ResourcePanelParams,
        });
        updateViewDescriptor(rawPanel.id, {
          title: nextTitle,
          subtitle: nextSubtitle,
        });
        hasChanges = true;
      }
    }

    if (hasChanges) {
      scheduleLayoutPersistence(api);
    }
  }, [
    isMobileViewport,
    normalizeDescriptorLocalization,
    resolveDescriptorSubtitle,
    resolveDescriptorTitle,
    resolveRouteLabel,
    scheduleLayoutPersistence,
    updateViewDescriptor,
    workbenchApi,
  ]);

  const watermarkComponent = useMemo(
    () =>
      function WorkbenchWatermark(_props: IWatermarkPanelProps) {
        return <WorkbenchEmptyState actions={emptyStateActions} onOpenItem={openRoutePanel} />;
      },
    [emptyStateActions, openRoutePanel],
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
      if (
        uiState.runtime?.ui_config.show_command_palette !== false
        && (e.metaKey || e.ctrlKey)
        && !e.shiftKey
        && e.key.toLowerCase() === "k"
      ) {
        e.preventDefault();
        setCommandPaletteOpen((current) => !current);
        return;
      }
      if (e.metaKey && e.shiftKey && e.key === "d") {
        setDockPosition(p => {
          if (p === "left") return "bottom";
          if (p === "bottom") return "right";
          if (p === "right") return "top";
          return "left";
        });
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [uiState.runtime?.ui_config.show_command_palette]);

  useEffect(() => {
    dockPositionRef.current = dockPosition;
    dockExpandedRef.current = dockExpanded;
    persistWorkbenchPreferences();
  }, [activeNavId, dockExpanded, dockPosition, isMobileViewport, persistWorkbenchPreferences]);

  useEffect(() => {
    if (!isMobileViewport) {
      return;
    }

    const storedMobileActiveNavId = readWorkbenchPreferences().mobileActiveNavId;
    if (!storedMobileActiveNavId) {
      return;
    }

    const hasStoredItem = navItems.some((item) => item.id === storedMobileActiveNavId);
    if (hasStoredItem) {
      setActiveNavId((current) => current ?? storedMobileActiveNavId);
    }
  }, [isMobileViewport, navItems]);

  useEffect(() => {
    syncOpenPanelTitles();
  }, [syncOpenPanelTitles]);

  useEffect(() => () => {
    if (typeof window === "undefined") {
      return;
    }

    if (pendingDropOpenRef.current !== null) {
      window.clearTimeout(pendingDropOpenRef.current);
    }

    if (pendingLayoutPersistRef.current !== null) {
      window.clearTimeout(pendingLayoutPersistRef.current);
    }
  }, []);

  const editorPadding = useMemo(() => {
    if (isMobileViewport) {
      return {
        paddingLeft: "12px",
        paddingRight: "12px",
        paddingBottom: "calc(var(--mobile-dock-height, 72px) + 12px)",
        paddingTop: "12px",
      };
    }

    const sideCollapsed = 60;
    const sideExpanded = 220;
    const edgeCollapsed = 52;
    const edgeExpanded = 88;
    return {
      paddingLeft: dockPosition === "left" ? `${dockExpanded ? sideExpanded : sideCollapsed}px` : "16px",
      paddingRight: dockPosition === "right" ? `${dockExpanded ? sideExpanded : sideCollapsed}px` : "16px",
      paddingBottom: dockPosition === "bottom" ? `${dockExpanded ? edgeExpanded : edgeCollapsed}px` : "16px",
      paddingTop: dockPosition === "top" ? `${dockExpanded ? edgeExpanded : edgeCollapsed}px` : "16px",
    };
  }, [dockExpanded, dockPosition, isMobileViewport]);

  return (
    <div className="ide-web">
      <main
        className={`editor-area ${dockThemeClass}`}
        style={{
          ...editorPadding,
          transition: "padding 0.4s cubic-bezier(0.16, 1, 0.3, 1)"
        }}
        onDragOver={(event) => {
          if (resolveNavDragPayload(event.dataTransfer)) {
            event.preventDefault();
            event.dataTransfer.dropEffect = "copy";
          }
        }}
        onDrop={(event) => {
          if (resolveNavDragPayload(event.dataTransfer)) {
            event.preventDefault();
          }
        }}
      >
        {isMobileViewport ? (
          activeNavItem ? (
            <section className={`mobile-workbench ${dockThemeClass}`}>
              <RoutedWorkbenchPanel
                api={null as never}
                containerApi={null as never}
                params={{
                  panelKind: "route",
                  routeId: activeNavItem.id,
                  href: activeNavItem.href,
                  label: activeNavItem.label,
                  source: activeNavItem.source,
                }}
              />
            </section>
          ) : (
            <WorkbenchEmptyState actions={emptyStateActions} isMobile />
          )
        ) : (
          <DockviewReact
            components={dockComponents}
            watermarkComponent={watermarkComponent}
            defaultTabComponent={WorkbenchTab}
            tabAnimation="smooth"
            onDidDrop={openDraggedRoutePanel}
            onWillDrop={(dropEvent) => {
              if (resolveNavDragPayload(dropEvent.nativeEvent.dataTransfer)) {
                dropEvent.nativeEvent.preventDefault();
              }
            }}
            onReady={(event) => {
              const api = event.api as any;
              registerApi(api);
              const storedLayout = initialWorkbenchPreferences.layout;
              const syncDesktopViewState = () => {
                setHasDesktopViews(Boolean((api.panels as IDockviewPanel[] | undefined)?.length));
              };

              api.onDidLayoutChange?.(() => {
                syncDesktopViewState();
                if (restoringLayoutRef.current) {
                  return;
                }
                scheduleLayoutPersistence(api);
              });
              api.onDidActivePanelChange?.((panel: IDockviewPanel | undefined) => {
                setActiveNavId(isRoutePanelParams(panel?.params) ? panel.params.routeId : undefined);
                setActivePanelId(panel?.id);
              });
              api.onUnhandledDragOverEvent?.((dragEvent: DockviewDndOverlayEvent) => {
                if (resolveNavDragPayload(dragEvent.nativeEvent.dataTransfer)) {
                  dragEvent.accept();
                }
              });
              if (storedLayout) {
                try {
                  restoringLayoutRef.current = true;
                  api.fromJSON(storedLayout, { reuseExistingPanels: false });
                  window.setTimeout(() => {
                    restoringLayoutRef.current = false;
                    syncOpenPanelTitles(api);
                    syncDesktopViewState();
                    scheduleLayoutPersistence(api);
                  }, 0);
                } catch {
                  restoringLayoutRef.current = false;
                  writeWorkbenchPreferences({
                    dockPosition: initialWorkbenchPreferences.dockPosition,
                    dockExpanded: initialWorkbenchPreferences.dockExpanded,
                  });
                }
              } else {
                syncOpenPanelTitles(api);
                syncDesktopViewState();
              }
            }}
          />
        )}
      </main>

      <OmniDock
        navItems={navItems as NavItem[]}
        activeId={isMobileViewport ? activeNavId : hasDesktopViews ? activeNavId : undefined}
        position={isMobileViewport ? "bottom" : dockPosition}
        onPositionChange={isMobileViewport ? () => undefined : setDockPosition}
        expanded={isMobileViewport ? false : dockExpanded}
        onExpandedChange={isMobileViewport ? () => undefined : setDockExpanded}
        onOpenItem={openRoutePanel}
      />

      {uiState.runtime?.ui_config.show_command_palette !== false ? (
        <CommandPalette
          open={commandPaletteOpen}
          actions={commandPaletteActions}
          onClose={() => setCommandPaletteOpen(false)}
          t={t}
        />
      ) : null}
    </div>
  );
}
