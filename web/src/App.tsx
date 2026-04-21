import { Outlet, useNavigate, useRouterState } from "@tanstack/react-router";
import { useEffect, useMemo, useState } from "react";
import { DockviewDefaultTab, DockviewReact, type IDockviewPanelProps } from "dockview";

import { getApiBaseUrl } from "@ennoia/api-client";
import { builtinExtensionPanels } from "@ennoia/builtins";
import { WORKBENCH_PALETTES, applyWorkbenchPalette, readWorkbenchPalette } from "@/lib/palette";
import { Memory } from "@/pages/memory";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { AgentEditorView } from "@/views/agents/Editor";
import { ApiChannelEditorView } from "@/views/providers/Editor";
import { SessionView } from "@/views/workspace/Session";
import { useWorkbenchStore, type WorkbenchViewDescriptor } from "@/stores/workbench";

type NavItem = {
  id: string;
  href: string;
  icon: string;
  label: string;
  hint: string;
  source: "builtin" | "extension";
};

type ResourcePanelParams = {
  descriptor: WorkbenchViewDescriptor;
};

const BUILTIN_NAV = [
  { id: "workspace", href: "/workspace", icon: "⌘", labelKey: "web.nav.workspace", fallback: "会话", hintKey: "web.nav.workspace_hint", hint: "统一发起 direct 和 group session" },
  { id: "agents", href: "/agents", icon: "A", labelKey: "web.nav.agents", fallback: "Agents", hintKey: "web.nav.agents_hint", hint: "多开 Agent 编辑视图" },
  { id: "skills", href: "/skills", icon: "S", labelKey: "web.nav.skills", fallback: "技能", hintKey: "web.nav.skills_hint", hint: "发现并分配技能给 Agent" },
  { id: "channels", href: "/upstreams", icon: "C", labelKey: "web.nav.channels", fallback: "API 上游渠道", hintKey: "web.nav.channels_hint", hint: "管理渠道实例与接口类型" },
  { id: "extensions", href: "/extensions", icon: "E", labelKey: "web.nav.extensions", fallback: "扩展", hintKey: "web.nav.extensions_hint", hint: "系统插件与贡献能力" },
  { id: "logs", href: "/logs", icon: "L", labelKey: "web.nav.logs", fallback: "日志", hintKey: "web.nav.logs_hint", hint: "统一观测台" },
  { id: "memory", href: "/memory", icon: "M", labelKey: "web.nav.memory", fallback: "记忆", hintKey: "web.nav.memory_hint", hint: "记忆可视化与审核" },
  { id: "tasks", href: "/tasks", icon: "T", labelKey: "web.nav.tasks", fallback: "任务", hintKey: "web.nav.tasks_hint", hint: "AI Prompt 与命令任务" },
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

function MainRoutePanel() {
  return (
    <div className="route-panel-scroll">
      <div className="route-panel-transition">
        <Outlet />
      </div>
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
    case "memory":
      return (
        <div className="resource-panel">
          <Memory />
        </div>
      );
    default:
      return <div className="empty-card">{descriptor.title}</div>;
  }
}

function WorkbenchTab(props: any) {
  const markClosed = useWorkbenchStore((state) => state.markClosed);
  const panelId = props.api?.id as string | undefined;
  const isFixedPanel = panelId === "main" || panelId === "inspector" || panelId === "extensionPanels";

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
  const navigate = useNavigate();
  const { resolveText, runtime, t } = useUiHelpers();
  const [palette, setPalette] = useState(readWorkbenchPalette);
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const registerApi = useWorkbenchStore((state) => state.registerApi);
  const resetLayout = useWorkbenchStore((state) => state.resetLayout);
  const openViews = useWorkbenchStore((state) => state.openViews);
  const recentViews = useWorkbenchStore((state) => state.recentViews);
  const closeView = useWorkbenchStore((state) => state.closeView);
  const focusView = useWorkbenchStore((state) => state.focusView);
  const openView = useWorkbenchStore((state) => state.openView);
  const restoreView = useWorkbenchStore((state) => state.restoreView);
  const dockThemeClass = palette === "paper" ? "dockview-theme-light" : "dockview-theme-dark";

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

  const active = useMemo(
    () => navItems.find((item) => pathname === item.href || pathname.startsWith(`${item.href}/`)) ?? navItems[0],
    [navItems, pathname],
  );

  const dockComponents = useMemo(
    () => ({
      main: MainRoutePanel,
      inspector: RuntimeInspectorPanel,
      extensionPanels: ExtensionPanelDeck,
      resource: ResourceViewPanel,
    }),
    [],
  );

  useEffect(() => {
    setPalette(applyWorkbenchPalette(palette, { clearRuntimeTheme: false }));
  }, [palette]);

  const visibleViews = openViews;

  return (
    <div className="ide-shell">
      <aside className="activity-bar" aria-label={t("web.nav.aria", "主导航")}>
        <button
          type="button"
          className="activity-brand"
          title="Ennoia Web"
          onClick={() => void navigate({ to: "/workspace" })}
        >
          E
        </button>
        <nav className="activity-list">
          {navItems.map((item) => (
            <button
              type="button"
              key={`${item.source}:${item.id}`}
              className={active?.id === item.id ? "activity-item activity-item--active" : "activity-item"}
              title={item.label}
              onClick={() => void navigate({ to: item.href as never })}
            >
              {item.icon}
            </button>
          ))}
        </nav>
      </aside>

      <aside className="primary-sidebar">
        <div className="sidebar-title">
          <span>{runtime ? resolveText(runtime.ui_config.web_title) : t("web.title", "Ennoia")}</span>
          <small>{active?.hint ?? t("web.brand.subtitle", "本地多 Agent Web 工作台")}</small>
        </div>

        <div className="stack">
          <article className="sidebar-card sidebar-card--fixed">
            <strong>{active?.label}</strong>
            <p className="helper-text">{active?.hint}</p>
          </article>

          <article className="sidebar-card sidebar-card--fixed">
            <strong>{t("web.workbench.open_views", "打开中的窗口")}</strong>
            <div className="stack">
              {visibleViews.length === 0 ? (
                <div className="empty-card">{t("web.workbench.open_views_empty", "可以从列表页多开 Agent、会话、API 上游渠道，也可以拖拽 Dockview 标签重排。")}</div>
              ) : (
                visibleViews.map((view) => (
                  <div key={view.panelId} className="session-card">
                    <button type="button" className="plain-card-button" onClick={() => focusView(view.panelId)}>
                      <strong>{view.title}</strong>
                      <span>{view.kind} · {view.entityId}</span>
                    </button>
                    <button type="button" className="icon-button" onClick={() => closeView(view.panelId)}>
                      ×
                    </button>
                  </div>
                ))
              )}
            </div>
          </article>

          <article className="sidebar-card sidebar-card--fixed">
            <strong>{t("web.workbench.view_panel", "视图面板")}</strong>
            <p className="helper-text">{t("web.workbench.view_panel_help", "此面板不可删除；它负责打开新窗口、恢复关闭窗口和切换布局样式。")}</p>
            <div className="button-row button-row--wrap">
              <button
                type="button"
                className="secondary"
                onClick={() =>
                  openView({
                    kind: "memory",
                    entityId: "memory",
                    title: t("web.nav.memory", "记忆"),
                    subtitle: t("web.memory.title", "记忆系统可视化"),
                  })}
              >
                {t("web.workbench.open_memory_window", "打开记忆窗口")}
              </button>
              <button type="button" className="secondary" onClick={() => resetLayout()}>
                {t("web.workbench.close_all", "关闭资源窗口")}
              </button>
            </div>
            <div className="stack">
              {recentViews.length === 0 ? (
                <div className="empty-card">{t("web.workbench.recent_empty", "暂无已关闭窗口。")}</div>
              ) : (
                recentViews.slice(0, 6).map((view) => (
                  <button
                    key={view.panelId}
                    type="button"
                    className="explorer-item"
                    onClick={() => restoreView(view.panelId)}
                  >
                    <strong>{view.title}</strong>
                    <small>{t("web.workbench.restore", "恢复")} · {view.kind}</small>
                  </button>
                ))
              )}
            </div>
          </article>

          <article className="sidebar-card sidebar-card--fixed">
            <strong>{t("web.palette.title", "工作台配色")}</strong>
            <label>
              {t("web.palette.label", "配色方案")}
              <select value={palette} onChange={(event) => setPalette(event.target.value)}>
                {WORKBENCH_PALETTES.map((item) => (
                  <option key={item.id} value={item.id}>
                    {t(`web.palette.${item.id}.label`, item.label)}
                  </option>
                ))}
              </select>
            </label>
          </article>
        </div>
      </aside>

      <main className={`editor-area ${dockThemeClass}`}>
        <DockviewReact
          components={dockComponents}
          defaultTabComponent={WorkbenchTab}
          onReady={(event) => {
            const api = event.api as any;
            registerApi(api);
            api.addPanel({
              id: "main",
              title: t("web.workbench.main_panel", "Web 工作台"),
              component: "main",
              disableClose: true,
            });
            api.addPanel({
              id: "inspector",
              title: t("web.panel.inspector", "Inspector"),
              component: "inspector",
              position: { referencePanel: "main", direction: "right" },
            });
            api.addPanel({
              id: "extensionPanels",
              title: t("web.panel.extensions", "扩展面板"),
              component: "extensionPanels",
              position: { referencePanel: "main", direction: "below" },
            });
          }}
        />
      </main>

      <footer className="status-bar">
        <span>Ennoia Web</span>
        <span>{active?.label}</span>
        <span>{t("web.status.palette", "配色")}: {palette}</span>
      </footer>
    </div>
  );
}

