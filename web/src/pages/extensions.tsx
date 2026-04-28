import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  getExtension,
  getExtensionLogs,
  listExtensions,
  reloadExtension,
  restartExtension,
  setExtensionEnabled,
  type ExtensionDetail,
  type ExtensionRuntimeState,
} from "@ennoia/api-client";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

type ExtensionStatusFilter = "all" | "running" | "error" | "disabled";

type ExtensionLogsState = {
  status: "idle" | "loading" | "ready" | "error";
  extensionId: string | null;
  content: string;
};

type ExtensionPageListItem = {
  id: string;
  title: string;
  mount: string;
};

type ExtensionPanelListItem = {
  id: string;
  title: string;
  mount: string;
  slot: string;
};

function statusBadgeClass(status: string) {
  const normalized = status.toLowerCase();
  if (normalized.includes("run") || normalized.includes("ok") || normalized.includes("healthy")) {
    return "badge--success";
  }
  if (normalized.includes("error") || normalized.includes("fail")) {
    return "badge--danger";
  }
  if (normalized.includes("warn") || normalized.includes("degrad")) {
    return "badge--warn";
  }
  return "badge--muted";
}

function localizeExtensionStatus(status: string, t: (key: string, fallback: string) => string) {
  switch (status.toLowerCase()) {
    case "running":
      return t("web.extensions.status.running", "运行中");
    case "error":
      return t("web.extensions.status.error", "异常");
    case "warn":
    case "warning":
      return t("web.extensions.status.warn", "警告");
    case "info":
      return t("web.extensions.status.info", "信息");
    case "stopped":
      return t("web.extensions.status.stopped", "已停止");
    case "starting":
      return t("web.extensions.status.starting", "启动中");
    case "reloading":
      return t("web.extensions.status.reloading", "重载中");
    default:
      return status;
  }
}

function localizeSourceMode(sourceMode: string, t: (key: string, fallback: string) => string) {
  switch (sourceMode.toLowerCase()) {
    case "dev":
      return t("web.extensions.source_mode.dev", "开发源");
    case "package":
      return t("web.extensions.source_mode.package", "已打包");
    default:
      return sourceMode;
  }
}

function extensionSortWeight(extension: ExtensionRuntimeState) {
  if (extension.status.toLowerCase() === "error") {
    return 0;
  }
  if (!extension.enabled) {
    return 3;
  }
  if (extension.status.toLowerCase() === "running") {
    return 1;
  }
  return 2;
}

export function Extensions() {
  const { formatDateTime, resolveText, runtime, t } = useUiHelpers();
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const detailRequestRef = useRef(0);
  const [extensions, setExtensions] = useState<ExtensionRuntimeState[]>([]);
  const [selected, setSelected] = useState<ExtensionRuntimeState | null>(null);
  const [detail, setDetail] = useState<ExtensionDetail | null>(null);
  const [logsState, setLogsState] = useState<ExtensionLogsState>({
    status: "idle",
    extensionId: null,
    content: "",
  });
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<ExtensionStatusFilter>("all");
  const [busy, setBusy] = useState(false);
  const [loadingDetail, setLoadingDetail] = useState(false);
  const [actionBusy, setActionBusy] = useState<"enable" | "disable" | "reload" | "restart" | null>(null);
  const [error, setError] = useState<string | null>(null);

  const loadExtensionDetail = useCallback(async (extensionId: string) => {
    const requestId = ++detailRequestRef.current;
    setLoadingDetail(true);
    const nextDetail = await getExtension(extensionId).catch(() => null);
    if (requestId === detailRequestRef.current) {
      setDetail(nextDetail);
      setLoadingDetail(false);
    }
  }, []);

  const refresh = useCallback(async (selectedId?: string | null) => {
    setBusy(true);
    setError(null);
    try {
      const next = await listExtensions();
      setExtensions(next);
      const nextSelected = next.find((item) => item.id === selectedId) ?? next[0] ?? null;
      setSelected(nextSelected);
      if (nextSelected) {
        setLogsState((current) =>
          current.extensionId === nextSelected.id
            ? current
            : { status: "idle", extensionId: nextSelected.id, content: "" },
        );
        await loadExtensionDetail(nextSelected.id);
      } else {
        setDetail(null);
        setLoadingDetail(false);
        setLogsState({ status: "idle", extensionId: null, content: "" });
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }, [loadExtensionDetail]);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function selectExtension(extension: ExtensionRuntimeState) {
    setSelected(extension);
    setLogsState((current) =>
      current.extensionId === extension.id
        ? current
        : { status: "idle", extensionId: extension.id, content: "" },
    );
    await loadExtensionDetail(extension.id);
  }

  async function handleAction(action: "enable" | "disable" | "reload" | "restart") {
    if (!selected) {
      return;
    }
    setError(null);
    setActionBusy(action);
    try {
      if (action === "enable" || action === "disable") {
        await setExtensionEnabled(selected.id, action === "enable");
      }
      if (action === "reload") {
        await reloadExtension(selected.id);
      }
      if (action === "restart") {
        await restartExtension(selected.id);
      }
      setLogsState({ status: "idle", extensionId: selected.id, content: "" });
      await refresh(selected.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setActionBusy(null);
    }
  }

  async function loadLogs(extensionId: string) {
    setLogsState((current) => ({
      status: "loading",
      extensionId,
      content: current.extensionId === extensionId ? current.content : "",
    }));
    try {
      const logs = await getExtensionLogs(extensionId);
      setLogsState({ status: "ready", extensionId, content: logs });
    } catch (err) {
      setLogsState({ status: "error", extensionId, content: String(err) });
    }
  }

  const extensionCountsById = useMemo(() => {
    const counts = new Map<string, { pageCount: number; panelCount: number }>();
    for (const page of runtime?.registry.pages ?? []) {
      const current = counts.get(page.extension_id) ?? { pageCount: 0, panelCount: 0 };
      current.pageCount += 1;
      counts.set(page.extension_id, current);
    }
    for (const panel of runtime?.registry.panels ?? []) {
      const current = counts.get(panel.extension_id) ?? { pageCount: 0, panelCount: 0 };
      current.panelCount += 1;
      counts.set(panel.extension_id, current);
    }
    return counts;
  }, [runtime?.registry.pages, runtime?.registry.panels]);

  const selectedPages = useMemo<ExtensionPageListItem[]>(() => {
    if (!selected) {
      return [];
    }
    const runtimePages = runtime?.registry.pages
      .filter((page) => page.extension_id === selected.id)
      .map((page) => ({
        id: page.page.id,
        title: resolveText(page.page.title),
        mount: page.page.mount,
      })) ?? [];
    if (runtimePages.length > 0 || !detail) {
      return runtimePages;
    }
    return detail.pages.map((page) => ({
      id: page.id,
      title: resolveText(page.title),
      mount: page.mount,
    }));
  }, [detail, resolveText, runtime?.registry.pages, selected]);

  const selectedPanels = useMemo<ExtensionPanelListItem[]>(() => {
    if (!selected) {
      return [];
    }
    const runtimePanels = runtime?.registry.panels
      .filter((panel) => panel.extension_id === selected.id)
      .map((panel) => ({
        id: panel.panel.id,
        title: resolveText(panel.panel.title),
        mount: panel.panel.mount,
        slot: panel.panel.slot,
      })) ?? [];
    if (runtimePanels.length > 0 || !detail) {
      return runtimePanels;
    }
    return detail.panels.map((panel) => ({
      id: panel.id,
      title: resolveText(panel.title),
      mount: panel.mount,
      slot: panel.slot,
    }));
  }, [detail, resolveText, runtime?.registry.panels, selected]);

  const filteredExtensions = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return [...extensions]
      .filter((extension) => {
        if (statusFilter === "running" && extension.status.toLowerCase() !== "running") {
          return false;
        }
        if (statusFilter === "error" && extension.status.toLowerCase() !== "error") {
          return false;
        }
        if (statusFilter === "disabled" && extension.enabled) {
          return false;
        }
        if (!normalizedQuery) {
          return true;
        }
        const haystack = [
          extension.name,
          extension.id,
          extension.kind,
          extension.source_mode,
        ].join("\n").toLowerCase();
        return haystack.includes(normalizedQuery);
      })
      .sort((left, right) => {
        const weightDiff = extensionSortWeight(left) - extensionSortWeight(right);
        if (weightDiff !== 0) {
          return weightDiff;
        }
        return left.name.localeCompare(right.name);
      });
  }, [extensions, query, statusFilter]);

  const totalRunning = useMemo(
    () => extensions.filter((extension) => extension.status.toLowerCase() === "running").length,
    [extensions],
  );
  const totalError = useMemo(
    () => extensions.filter((extension) => extension.status.toLowerCase() === "error").length,
    [extensions],
  );
  const totalDisabled = useMemo(
    () => extensions.filter((extension) => !extension.enabled).length,
    [extensions],
  );
  const totalPages = runtime?.registry.pages.length ?? 0;
  const totalPanels = runtime?.registry.panels.length ?? 0;
  const selectedDiagnostics = detail?.diagnostics ?? selected?.diagnostics ?? [];
  const selectedHealth = detail?.health ?? selected?.status ?? t("web.common.unknown", "未知");
  const selectedPageCount = selectedPages.length;
  const selectedPanelCount = selectedPanels.length;
  const capabilityCount = detail?.capability_rows.length ?? 0;
  const surfaceCount = detail?.surfaces.length ?? 0;
  const resourceTypeCount = detail?.resource_types.length ?? 0;
  const subscriptionCount = detail?.subscriptions.length ?? 0;
  const logsButtonLabel = selected && logsState.extensionId === selected.id && logsState.status === "ready"
    ? t("web.action.refresh", "刷新")
    : t("web.extensions.view_logs", "查看日志");

  function openExtensionPage(pageId: string, label: string) {
    if (!workbenchApi) {
      setError(t("web.extensions.open_page_unavailable", "工作台尚未就绪，无法打开扩展视图。"));
      return;
    }
    workbenchApi.addPanel({
      id: `route:extension:${pageId}:${Date.now().toString(36)}`,
      title: label,
      component: "route",
      params: {
        routeId: pageId,
        href: `/extension-pages/${encodeURIComponent(pageId)}`,
        label,
        source: "extension",
      },
    });
  }

  return (
    <div className="extensions-page">
      <section className="work-panel extensions-toolbar">
        <div className="extensions-toolbar__row">
          <div className="page-heading">
            <span>{t("web.extensions.eyebrow", "Extensions")}</span>
            <h1>{t("web.extensions.title", "扩展负责系统能力，不和技能混用。")}</h1>
            <p>{t("web.extensions.description", "这里按扩展查看运行状态、能力说明、重载和日志。来源目录只显示相对实例路径。")}</p>
          </div>
          <div className="extensions-toolbar__actions">
            <button type="button" className="secondary" onClick={() => void refresh(selected?.id)} disabled={busy}>
              {busy ? t("web.common.loading", "加载中…") : t("web.action.rescan", "重新扫描")}
            </button>
          </div>
        </div>

        {error ? <div className="error">{error}</div> : null}

        <div className="extensions-overview-grid">
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_total", "扩展总数")}</span>
            <strong>{extensions.length}</strong>
            <small>{t("web.extensions.catalog", "扩展目录")}</small>
          </article>
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_running", "运行中")}</span>
            <strong>{totalRunning}</strong>
            <small>{t("web.extensions.runtime_overview", "运行概览")}</small>
          </article>
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_error", "异常")}</span>
            <strong>{totalError}</strong>
            <small>{t("web.extensions.diagnostics", "诊断")}</small>
          </article>
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_disabled", "停用")}</span>
            <strong>{totalDisabled}</strong>
            <small>{t("web.common.disabled", "停用")}</small>
          </article>
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_pages", "扩展视图")}</span>
            <strong>{totalPages}</strong>
            <small>{t("web.extensions.surfaces", "界面挂载")}</small>
          </article>
          <article className="metric-card extensions-metric-card">
            <span>{t("web.extensions.summary_panels", "扩展面板")}</span>
            <strong>{totalPanels}</strong>
            <small>{t("web.extensions.ui_contributions", "UI 贡献")}</small>
          </article>
        </div>
      </section>

      <div className="extensions-shell">
        <section className="work-panel extensions-catalog-panel">
          <div className="extensions-section__header">
            <div className="page-heading">
              <span>{t("web.extensions.catalog", "扩展目录")}</span>
              <h1>{t("web.extensions.catalog_title", "按状态定位扩展")}</h1>
              <p>{t("web.extensions.catalog_description", "先筛出异常或停用扩展，再进入右侧查看能力、诊断和日志。")}</p>
            </div>
            <span className="extensions-catalog-count">
              {`${filteredExtensions.length} ${t("web.extensions.catalog_count", "项")}`}
            </span>
          </div>

          <div className="extensions-catalog-toolbar">
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("web.extensions.search_placeholder", "搜索扩展名称、ID、类型或来源")}
            />
            <div className="extensions-filter-tabs">
              {[
                ["all", t("web.extensions.filter_all", "全部")],
                ["running", t("web.extensions.filter_running", "运行中")],
                ["error", t("web.extensions.filter_error", "异常")],
                ["disabled", t("web.extensions.filter_disabled", "停用")],
              ].map(([value, label]) => (
                <button
                  key={value}
                  type="button"
                  className={`chip extensions-filter-chip ${statusFilter === value ? "chip--active" : ""}`}
                  onClick={() => setStatusFilter(value as ExtensionStatusFilter)}
                >
                  {label}
                </button>
              ))}
            </div>
          </div>

          <div className="extensions-catalog-list">
            {filteredExtensions.length === 0 ? (
              <div className="empty-card">
                {t("web.extensions.empty_filtered", "当前筛选下没有匹配的扩展。")}
              </div>
            ) : (
              filteredExtensions.map((extension) => {
                const counts = extensionCountsById.get(extension.id) ?? { pageCount: 0, panelCount: 0 };
                return (
                  <article
                    key={extension.id}
                    className={`resource-card extensions-catalog-card ${selected?.id === extension.id ? "extensions-catalog-card--active" : ""}`}
                  >
                    <button type="button" className="plain-card-button" onClick={() => void selectExtension(extension)}>
                      <header className="extensions-catalog-card__header">
                        <div className="stack extensions-catalog-card__title">
                          <strong>{extension.name}</strong>
                          <small>{extension.id}</small>
                        </div>
                        <span className={`badge ${statusBadgeClass(extension.status)}`}>
                          {localizeExtensionStatus(extension.status, t)}
                        </span>
                      </header>
                      <div className="extensions-inline-meta">
                        <span className="badge badge--muted">{extension.kind}</span>
                        <span className="badge badge--muted">{localizeSourceMode(extension.source_mode, t)}</span>
                        <span className={extension.enabled ? "badge badge--success" : "badge badge--muted"}>
                          {extension.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
                        </span>
                      </div>
                      <div className="extensions-inline-meta">
                        <span>{`${t("web.extensions.summary_pages", "扩展视图")} ${counts.pageCount}`}</span>
                        <span>{`${t("web.extensions.summary_panels", "扩展面板")} ${counts.panelCount}`}</span>
                        <span>{`${t("web.extensions.diagnostics", "诊断")} ${extension.diagnostics.length}`}</span>
                      </div>
                    </button>
                  </article>
                );
              })
            )}
          </div>
        </section>

        <aside className="work-panel extensions-detail-panel">
          {selected ? (
            <div className="extensions-detail-scroll">
              <section className="extensions-hero">
                <div className="extensions-hero__copy">
                  <span>{t("web.extensions.details", "扩展详情")}</span>
                  <h1>{selected.name}</h1>
                  <p>{detail?.description || t("web.common.none", "无")}</p>
                  <div className="extensions-inline-meta">
                    <span className={`badge ${statusBadgeClass(selected.status)}`}>{localizeExtensionStatus(selected.status, t)}</span>
                    <span className={`badge ${statusBadgeClass(selectedHealth)}`}>{localizeExtensionStatus(selectedHealth, t)}</span>
                    <span className="badge badge--muted">{selected.kind}</span>
                    <span className="badge badge--muted">{localizeSourceMode(selected.source_mode, t)}</span>
                    <span className={selected.enabled ? "badge badge--success" : "badge badge--muted"}>
                      {selected.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
                    </span>
                  </div>
                </div>
                <div className="extensions-hero__actions">
                  <button
                    type="button"
                    onClick={() => void handleAction(selected.enabled ? "disable" : "enable")}
                    disabled={actionBusy !== null}
                  >
                    {actionBusy === (selected.enabled ? "disable" : "enable")
                      ? t("web.common.loading", "加载中…")
                      : selected.enabled
                        ? t("web.action.disable", "停用")
                        : t("web.action.enable", "启用")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void handleAction("reload")}
                    disabled={actionBusy !== null}
                  >
                    {actionBusy === "reload" ? t("web.common.loading", "加载中…") : t("web.action.reload", "重载")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void handleAction("restart")}
                    disabled={actionBusy !== null}
                  >
                    {actionBusy === "restart" ? t("web.common.loading", "加载中…") : t("web.action.restart", "重启")}
                  </button>
                </div>
              </section>

              <section className="extensions-section">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.runtime_overview", "运行概览")}</div>
                    {loadingDetail ? <p className="helper-text">{t("web.common.loading", "加载中…")}</p> : null}
                  </div>
                </div>
                <div className="kv-list extensions-kv-list">
                  <span>ID</span><strong>{selected.id}</strong>
                  <span>{t("web.common.status", "状态")}</span><strong>{localizeExtensionStatus(selected.status, t)}</strong>
                  <span>{t("web.extensions.health", "健康状态")}</span><strong>{localizeExtensionStatus(selectedHealth, t)}</strong>
                  <span>{t("web.extensions.generation", "版本代次")}</span><strong>{detail?.generation ?? t("web.common.none", "无")}</strong>
                  <span>{t("web.extensions.diagnostics", "诊断")}</span><strong>{selectedDiagnostics.length}</strong>
                  <span>{t("web.extensions.install_dir", "扩展目录")}</span><strong>{formatRelativePath(selected.install_dir)}</strong>
                  <span>{t("web.extensions.source_root", "来源目录")}</span><strong>{formatRelativePath(selected.source_root)}</strong>
                  <span>{t("web.extensions.docs", "文档入口")}</span><strong>{detail?.docs ? formatRelativePath(detail.docs) : t("web.common.none", "无")}</strong>
                </div>
                <div className="extensions-runtime-grid">
                  <article className="mini-card extensions-runtime-card">
                    <div className="extensions-runtime-card__header">
                      <strong>{t("web.extensions.ui_entry", "UI 入口")}</strong>
                      {detail?.ui ? <span className="badge badge--muted">{detail.ui.kind}</span> : null}
                    </div>
                    {detail?.ui ? (
                      <>
                        <p className="extensions-runtime-card__path">{detail.ui.entry}</p>
                        <div className="extensions-inline-meta extensions-runtime-card__meta">
                          <span>{`version ${detail.ui.version}`}</span>
                          <span>{detail.ui.hmr ? "HMR" : "static"}</span>
                        </div>
                      </>
                    ) : (
                      <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                    )}
                  </article>
                  <article className="mini-card extensions-runtime-card">
                    <div className="extensions-runtime-card__header">
                      <strong>{t("web.extensions.worker_entry", "Worker 入口")}</strong>
                      {detail?.worker ? <span className="badge badge--muted">{detail.worker.kind}</span> : null}
                    </div>
                    {detail?.worker ? (
                      <>
                        <p className="extensions-runtime-card__path">{detail.worker.entry}</p>
                        <div className="extensions-inline-meta extensions-runtime-card__meta">
                          <span>{`ABI ${detail.worker.abi}`}</span>
                          <span>{detail.worker.status}</span>
                        </div>
                      </>
                    ) : (
                      <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                    )}
                  </article>
                </div>
              </section>

              <section className="extensions-section">
                <div className="panel-title">{t("web.extensions.capability_summary", "能力与挂载")}</div>
                <div className="extensions-summary-grid">
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.capabilities", "能力声明")}</span>
                        <strong>{t("web.extensions.capabilities", "能力声明")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{capabilityCount}</span>
                    </div>
                    <p>{t("web.extensions.capabilities_help", "manifest 的 capabilities 是系统能力入口；Provider、Interface、Memory 等视图都从这里派生。")}</p>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.surfaces", "界面挂载")}</span>
                        <strong>{t("web.extensions.surfaces", "界面挂载")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{surfaceCount}</span>
                    </div>
                    <p>{t("web.extensions.surfaces_help", "surfaces 统一表达 page、panel 等 UI 挂载点。")}</p>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.subscriptions", "事件订阅")}</span>
                        <strong>{t("web.extensions.subscriptions", "事件订阅")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{subscriptionCount}</span>
                    </div>
                    <p>{t("web.extensions.subscriptions_help", "subscriptions 只声明监听关系，实际 Hook 处理入口由 capability.entry 决定。")}</p>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.resource_types", "资源类型")}</span>
                        <strong>{t("web.extensions.resource_types", "资源类型")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{resourceTypeCount}</span>
                    </div>
                    <p>{t("web.extensions.resource_types_help", "resource_types 用来声明扩展理解和产出的资源模型。")}</p>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.summary_pages", "扩展视图")}</span>
                        <strong>{t("web.extensions.summary_pages", "扩展视图")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{selectedPageCount}</span>
                    </div>
                    <p>{t("web.extensions.contributes_ui_help", "扩展可贡献页面、面板、主题和语言包。")}</p>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.summary_panels", "扩展面板")}</span>
                        <strong>{t("web.extensions.summary_panels", "扩展面板")}</strong>
                      </div>
                      <span className="badge badge--muted extensions-summary-card__count">{selectedPanelCount}</span>
                    </div>
                    <p>{t("web.extensions.contributes_ui", "贡献 UI")}</p>
                  </article>
                </div>
              </section>

              <section className="extensions-section">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.diagnostics", "诊断")}</div>
                    <p className="helper-text">{t("web.extensions.diagnostics_description", "先看诊断摘要，再决定是否重载、重启或查看日志。")}</p>
                  </div>
                  <span className={`badge ${selectedDiagnostics.length > 0 ? "badge--warn" : "badge--muted"}`}>
                    {selectedDiagnostics.length}
                  </span>
                </div>
                {selectedDiagnostics.length === 0 ? (
                  <div className="empty-card extensions-empty-state">
                    <strong>{t("web.extensions.diagnostics_empty_title", "当前状态正常")}</strong>
                    <p>{t("web.extensions.diagnostics_empty", "当前没有诊断。")}</p>
                  </div>
                ) : (
                  <div className="extensions-diagnostic-list">
                    {selectedDiagnostics.map((diagnostic, index) => (
                      <article key={`${diagnostic.at}:${diagnostic.summary}:${index}`} className={`mini-card extensions-diagnostic-card extensions-diagnostic-card--${statusBadgeClass(diagnostic.level).replace("badge--", "")}`}>
                        <header className="extensions-diagnostic-card__header">
                          <div className="stack extensions-diagnostic-card__title">
                            <strong>{diagnostic.summary}</strong>
                            <small>{formatDateTime(diagnostic.at)}</small>
                          </div>
                          <span className={`badge ${statusBadgeClass(diagnostic.level)}`}>
                            {localizeExtensionStatus(diagnostic.level, t)}
                          </span>
                        </header>
                        {diagnostic.detail ? <p className="extensions-diagnostic-card__detail">{diagnostic.detail}</p> : null}
                      </article>
                    ))}
                  </div>
                )}
              </section>

              <section className="extensions-section">
                <div className="panel-title">{t("web.extensions.ui_contributions", "UI 贡献")}</div>
                <div className="extensions-contributions-grid">
                  <div className="stack extensions-contribution-column">
                    <div className="extensions-subtitle">{t("web.extensions.pages", "扩展视图")}</div>
                    {selectedPages.length === 0 ? (
                      <div className="empty-card extensions-contribution-empty">
                        <strong>{t("web.extensions.ui_empty_title", "暂无贡献")}</strong>
                        <p>{t("web.extensions.pages_empty", "这个扩展没有声明视图。")}</p>
                      </div>
                    ) : (
                      selectedPages.map((page) => (
                        <article key={page.id} className="mini-card extensions-contribution-card">
                          <div className="extensions-contribution-card__header">
                            <strong>{page.title}</strong>
                            <span className="badge badge--muted">{t("web.extensions.page_kind", "页面")}</span>
                          </div>
                          <div className="extensions-contribution-card__body">
                            <span>{t("web.extensions.mount_point", "挂载点")}</span>
                            <p className="extensions-contribution-card__path">{page.mount}</p>
                          </div>
                          <div className="button-row extensions-contribution-card__footer">
                            <button type="button" className="secondary" onClick={() => openExtensionPage(page.id, page.title)}>
                              {t("web.extensions.open_page", "打开视图")}
                            </button>
                          </div>
                        </article>
                      ))
                    )}
                  </div>
                  <div className="stack extensions-contribution-column">
                    <div className="extensions-subtitle">{t("web.extensions.panels", "扩展面板")}</div>
                    {selectedPanels.length === 0 ? (
                      <div className="empty-card extensions-contribution-empty">
                        <strong>{t("web.extensions.ui_empty_title", "暂无贡献")}</strong>
                        <p>{t("web.extensions.panels_empty", "这个扩展没有声明面板。")}</p>
                      </div>
                    ) : (
                      selectedPanels.map((panel) => (
                        <article key={panel.id} className="mini-card extensions-contribution-card">
                          <div className="extensions-contribution-card__header">
                            <strong>{panel.title}</strong>
                            <span className="badge badge--muted">{panel.slot}</span>
                          </div>
                          <div className="extensions-contribution-card__body">
                            <span>{t("web.extensions.mount_point", "挂载点")}</span>
                            <p className="extensions-contribution-card__path">{panel.mount}</p>
                          </div>
                        </article>
                      ))
                    )}
                  </div>
                </div>
              </section>

              <section className="extensions-section">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.conversation", "会话装配")}</div>
                    <p className="helper-text">{t("web.extensions.conversation_description", "这里决定扩展是否进入会话目录，以及会向会话暴露哪些能力条件。")}</p>
                  </div>
                  <span className={`badge ${detail?.conversation.inject ? "badge--success" : "badge--muted"}`}>
                    {detail?.conversation.inject ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
                  </span>
                </div>
                {detail?.conversation.inject ? (
                  <div className="extensions-conversation-grid">
                    <article className="mini-card extensions-conversation-card extensions-conversation-card--status">
                      <div className="extensions-conversation-card__header">
                        <strong>{t("web.extensions.conversation_enabled", "进入会话目录")}</strong>
                        <span className="badge badge--success">{t("web.common.yes", "是")}</span>
                      </div>
                      <p>{t("web.extensions.conversation_enabled_help", "这个扩展会出现在会话能力目录里，可被会话按规则引用。")}</p>
                    </article>
                    <article className="mini-card extensions-conversation-card">
                      <div className="extensions-conversation-card__header">
                        <strong>{t("web.extensions.conversation_material", "会话注入内容")}</strong>
                      </div>
                      <p>{t("web.extensions.conversation_material_help", "只复用扩展说明和这里声明的能力目录，不自动注入文档正文。")}</p>
                    </article>
                    <article className="mini-card extensions-conversation-card">
                      <div className="extensions-conversation-card__header">
                        <strong>{t("web.extensions.conversation_resource_types", "资源类型条件")}</strong>
                      </div>
                      {detail.conversation.resource_types.length > 0 ? (
                        <div className="chip-grid">
                          {detail.conversation.resource_types.map((item) => (
                            <span key={item} className="chip chip--active">{item}</span>
                          ))}
                        </div>
                      ) : (
                        <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                      )}
                    </article>
                    <article className="mini-card extensions-conversation-card">
                      <div className="extensions-conversation-card__header">
                        <strong>{t("web.extensions.conversation_capabilities", "能力入口")}</strong>
                      </div>
                      {detail.conversation.capabilities.length > 0 ? (
                        <div className="chip-grid">
                          {detail.conversation.capabilities.map((item) => (
                            <span key={item} className="chip chip--active">{item}</span>
                          ))}
                        </div>
                      ) : (
                        <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                      )}
                    </article>
                  </div>
                ) : (
                  <div className="empty-card extensions-empty-state">
                    <strong>{t("web.extensions.conversation_empty_title", "未接入会话目录")}</strong>
                    <p>{t("web.extensions.conversation_empty", "这个扩展没有声明进入会话目录的规则。")}</p>
                  </div>
                )}
              </section>

              <section className="extensions-section">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.logs", "运行日志")}</div>
                    <p className="helper-text">{t("web.extensions.logs_description", "日志用于确认扩展启动、重载和运行期是否出现具体错误。")}</p>
                  </div>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void loadLogs(selected.id)}
                    disabled={logsState.status === "loading"}
                  >
                    {logsState.status === "loading" ? t("web.common.loading", "加载中…") : logsButtonLabel}
                  </button>
                </div>
                {logsState.status === "idle" ? (
                  <div className="empty-card extensions-empty-state extensions-logs-empty">
                    <strong>{t("web.extensions.log_empty_title", "日志尚未加载")}</strong>
                    <p>{t("web.extensions.log_empty", "选择“查看日志”加载扩展日志。")}</p>
                  </div>
                ) : logsState.status === "loading" ? (
                  <div className="empty-card extensions-empty-state extensions-logs-empty">
                    <strong>{t("web.extensions.log_loading_title", "正在读取日志")}</strong>
                    <p>{t("web.extensions.log_loading", "正在加载扩展日志。")}</p>
                  </div>
                ) : logsState.status === "error" ? (
                  <div className="error">{logsState.content}</div>
                ) : (
                  <div className="extensions-log-panel">
                    <div className="extensions-log-panel__meta">
                      <strong>{selected.name}</strong>
                      <span>{`${t("web.extensions.log_for", "当前日志属于")} ${selected.name}`}</span>
                    </div>
                    <pre className="log-view extensions-log-view">{logsState.content}</pre>
                  </div>
                )}
              </section>
            </div>
          ) : (
            <div className="empty-card">{t("web.extensions.empty", "暂无扩展。")}</div>
          )}
        </aside>
      </div>
    </div>
  );
}
