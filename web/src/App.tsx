import { useMemo, useState, useEffect, useRef } from "react";
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
import { builtinExtensionPanels } from "@ennoia/builtins";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { AgentEditorView } from "@/views/agents/Editor";
import { ApiChannelEditorView } from "@/views/providers/Editor";
import { SessionView } from "@/views/conversations/Session";
import { useWorkbenchStore, type WorkbenchViewDescriptor } from "@/stores/workbench";
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
import { Logs } from "@/pages/logs";
import { Providers } from "@/pages/providers";
import { Settings } from "@/pages/settings";
import { Skills } from "@/pages/skills";
import { ExtensionPageView } from "@/views/extensions/Page";

type ResourcePanelParams = {
  descriptor: WorkbenchViewDescriptor;
};

type RoutePanelParams = {
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
  { id: "channels", href: "/upstreams", icon: "C", labelKey: "web.nav.channels", fallback: "API 上游渠道", hintKey: "web.nav.channels_hint", hint: "管理渠道实例与接口类型" },
  { id: "extensions", href: "/extensions", icon: "E", labelKey: "web.nav.extensions", fallback: "扩展", hintKey: "web.nav.extensions_hint", hint: "系统插件与贡献能力" },
  { id: "logs", href: "/logs", icon: "L", labelKey: "web.nav.logs", fallback: "日志", hintKey: "web.nav.logs_hint", hint: "统一观测台" },
  { id: "settings", href: "/settings", icon: "⚙", labelKey: "web.nav.settings", fallback: "设置", hintKey: "web.nav.settings_hint", hint: "运行时配置表单" },
] as const;

const HIDDEN_EXTENSION_IDS = new Set(["observatory", "ext.observatory"]);
const HIDDEN_EXTENSION_ROUTES = new Set(["/observatory"]);

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
  const panels =
    runtime?.registry.panels.filter(
      (panel) =>
        !HIDDEN_EXTENSION_IDS.has(panel.extension_id) &&
        !panel.panel.mount.startsWith("observatory."),
    ) ?? [];

  return (
    <div className="dock-panel-content dock-panel-content--horizontal">
      {panels.length === 0 ? (
        <div className="empty-card">{t("web.panel.no_extensions", "暂无扩展面板贡献。")}</div>
      ) : (
        panels.map((panel) => {
          const descriptor = builtinExtensionPanels[panel.panel.mount];
          return (
            <article key={`${panel.extension_id}:${panel.panel.id}`} className="mini-card">
              <strong>{resolveText(panel.panel.title)}</strong>
              <span>{panel.panel.slot} · {panel.extension_id}</span>
              <span>{descriptor?.summary ?? panel.panel.mount}</span>
            </article>
          );
        })
      )}
    </div>
  );
}

function ResourceViewPanel(props: IDockviewPanelProps<ResourcePanelParams>) {
  const descriptor = props.params.descriptor;
  const openView = useWorkbenchStore((state) => state.openView);

  switch (descriptor.kind) {
    case "agent":
      return (
        <div className="resource-panel">
          <AgentEditorView
            agentId={descriptor.entityId}
            onOpenApiChannel={(channelId) =>
              openView({
                kind: "api-channel",
                entityId: channelId,
                title: channelId,
              })}
          />
        </div>
      );
    case "api-channel":
      return (
        <div className="resource-panel">
          <ApiChannelEditorView channelId={descriptor.entityId} />
        </div>
      );
    case "session":
      return (
        <div className="resource-panel">
          <SessionView sessionId={descriptor.entityId} />
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
      case "channels":
        return <Providers />;
      case "extensions":
        return <Extensions />;
      case "logs":
        return <Logs />;
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
  const registerApi = useWorkbenchStore((state) => state.registerApi);
  const initialWorkbenchPreferences = useMemo(() => readWorkbenchPreferences(), []);
  const isMobileViewport = useIsMobileViewport();
  const dockThemeClass = describeAppliedTheme(uiState.themeId).appearance === "light"
    ? "dockview-theme-light"
    : "dockview-theme-dark";
  const [dockPosition, setDockPosition] = useState<DockPosition>(initialWorkbenchPreferences.dockPosition);
  const [dockExpanded, setDockExpanded] = useState(initialWorkbenchPreferences.dockExpanded);
  const [activeNavId, setActiveNavId] = useState<string | undefined>(initialWorkbenchPreferences.mobileActiveNavId);
  const [hasDesktopViews, setHasDesktopViews] = useState(false);
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
        .filter(
          (page) =>
            !HIDDEN_EXTENSION_IDS.has(page.extension_id) &&
            !HIDDEN_EXTENSION_ROUTES.has(page.page.route) &&
            !page.page.mount.startsWith("observatory."),
        )
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

  const openRoutePanel = (payload: NavItem, options?: { panelPosition?: ReturnType<typeof buildPanelPosition> }) => {
    if (isMobileViewport) {
      setActiveNavId(payload.id);
      return;
    }

    const currentApi = useWorkbenchStore.getState().api;
    if (!currentApi) {
      return;
    }

    if (!options?.panelPosition) {
      const activePanel = currentApi.activePanel as IDockviewPanel | undefined;
      const activePanelParams = activePanel?.params as RoutePanelParams | undefined;
      if (activePanel && activePanelParams) {
        activePanel.api.setTitle(payload.label);
        activePanel.update({
          params: {
            routeId: payload.id,
            href: payload.href,
            label: payload.label,
            source: payload.source,
          },
        });
        setActiveNavId(payload.id);
        return;
      }
    }

    currentApi.addPanel({
      id: `route:${payload.source}:${payload.id}:${Date.now().toString(36)}`,
      title: payload.label,
      component: "route",
      params: {
        routeId: payload.id,
        href: payload.href,
        label: payload.label,
        source: payload.source,
      },
      position: options?.panelPosition,
    });
    setHasDesktopViews(true);
    setActiveNavId(payload.id);
  };

  const scheduleOpenRoutePanel = (payload: NavItem, options?: { panelPosition?: ReturnType<typeof buildPanelPosition> }) => {
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
  };

  const openDraggedRoutePanel = (event: DockviewDidDropEvent) => {
    const payload = resolveNavDragPayload(event.nativeEvent.dataTransfer);
    if (!payload) {
      return;
    }

    scheduleOpenRoutePanel(payload, { panelPosition: buildPanelPosition(event) });
  };

  const persistWorkbenchPreferences = (layout?: unknown) => {
    writeWorkbenchPreferences({
      dockPosition: dockPositionRef.current,
      dockExpanded: dockExpandedRef.current,
      layout: layout === undefined ? readWorkbenchPreferences().layout : layout,
      mobileActiveNavId: isMobileViewport ? activeNavId : readWorkbenchPreferences().mobileActiveNavId,
    });
  };

  const scheduleLayoutPersistence = (api: { toJSON?: () => unknown }) => {
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
  };

  const watermarkComponent = useMemo(
    () =>
      function WorkbenchWatermark(_props: IWatermarkPanelProps) {
        return <WorkbenchEmptyState actions={emptyStateActions} onOpenItem={openRoutePanel} />;
      },
    [emptyStateActions, openRoutePanel],
  );

  useEffect(() => {
    const handleKeyDown = (e: KeyboardEvent) => {
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
  }, []);

  useEffect(() => {
    dockPositionRef.current = dockPosition;
    dockExpandedRef.current = dockExpanded;
    persistWorkbenchPreferences();
  }, [activeNavId, dockExpanded, dockPosition, isMobileViewport]);

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
        paddingBottom: "92px",
        paddingTop: "12px",
      };
    }

    const collapsed = 88;
    const sideExpanded = 248;
    const edgeExpanded = 140;
    return {
      paddingLeft: dockPosition === "left" ? `${dockExpanded ? sideExpanded : collapsed}px` : "16px",
      paddingRight: dockPosition === "right" ? `${dockExpanded ? sideExpanded : collapsed}px` : "16px",
      paddingBottom: dockPosition === "bottom" ? `${dockExpanded ? edgeExpanded : collapsed}px` : "16px",
      paddingTop: dockPosition === "top" ? `${dockExpanded ? edgeExpanded : collapsed}px` : "16px",
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
                const params = panel?.params as RoutePanelParams | undefined;
                setActiveNavId(params?.routeId);
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
    </div>
  );
}
