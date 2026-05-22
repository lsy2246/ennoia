import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
  createExtensionEventsStream,
  getExtension,
  getExtensionLogs,
  getExtensionSettings,
  listExtensions,
  reloadExtension,
  restartExtension,
  saveExtensionSettings,
  setExtensionEnabled,
  type ExtensionDetail,
  type ExtensionRuntimeState,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

type ExtensionStatusFilter = "all" | "ready" | "error" | "disabled";

type ExtensionLogsState = {
  status: "idle" | "loading" | "ready" | "error";
  extensionId: string | null;
  content: string;
};

type ExtensionSettingsState = {
  status: "idle" | "loading" | "ready" | "saving" | "error";
  extensionId: string | null;
  values: Record<string, string | number | boolean>;
  message: string | null;
};

function statusBadgeClass(status: string) {
  const normalized = status.toLowerCase();
  if (normalized.includes("ready") || normalized.includes("run") || normalized.includes("ok")) {
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
    case "ready":
    case "running":
      return t("web.extensions.status.ready", "就绪");
    case "failed":
    case "error":
      return t("web.extensions.status.error", "异常");
    case "degraded":
      return t("web.extensions.status.degraded", "降级");
    case "stopped":
    case "disabled":
      return t("web.extensions.status.stopped", "已停用");
    case "discovering":
      return t("web.extensions.status.discovering", "发现中");
    case "resolving":
      return t("web.extensions.status.resolving", "解析中");
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
  if (["failed", "error"].includes(extension.status.toLowerCase())) {
    return 0;
  }
  if (!extension.enabled) {
    return 3;
  }
  if (extension.status.toLowerCase() === "ready") {
    return 1;
  }
  return 2;
}

function sharedOperationValue(values: Array<string | null | undefined>) {
  if (values.length === 0 || !values[0]) {
    return null;
  }
  return values.every((value) => value === values[0]) ? values[0] : null;
}

export function Extensions() {
  const { formatDateTime, resolveText, runtime, t } = useUiHelpers();
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const detailRequestRef = useRef(0);
  const selectedIdRef = useRef<string | null>(null);
  const [extensions, setExtensions] = useState<ExtensionRuntimeState[]>([]);
  const [selected, setSelected] = useState<ExtensionRuntimeState | null>(null);
  const [detail, setDetail] = useState<ExtensionDetail | null>(null);
  const [logsState, setLogsState] = useState<ExtensionLogsState>({
    status: "idle",
    extensionId: null,
    content: "",
  });
  const [settingsState, setSettingsState] = useState<ExtensionSettingsState>({
    status: "idle",
    extensionId: null,
    values: {},
    message: null,
  });
  const [query, setQuery] = useState("");
  const [statusFilter, setStatusFilter] = useState<ExtensionStatusFilter>("all");
  const [busy, setBusy] = useState(false);
  const [actionBusy, setActionBusy] = useState<"enable" | "disable" | "reload" | "restart" | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    selectedIdRef.current = selected?.id ?? null;
  }, [selected?.id]);

  const selectedSettings = detail?.settings ?? [];
  const selectedDiagnostics = detail?.diagnostics ?? selected?.diagnostics ?? [];
  const selectedHealth = detail?.health ?? selected?.status ?? t("web.common.unknown", "未知");
  const selectedOperations = detail?.operations ?? [];
  const selectedEvents = detail?.events ?? [];
  const selectedViews = detail?.views ?? [];
  const selectedConversation = detail?.conversation;
  const selectedCompat = detail?.compat;

  const loadExtensionDetail = useCallback(async (extensionId: string) => {
    const requestId = ++detailRequestRef.current;
    setSettingsState({
      status: "loading",
      extensionId,
      values: {},
      message: null,
    });
    const [nextDetail, nextSettings] = await Promise.all([
      getExtension(extensionId).catch(() => null),
      getExtensionSettings(extensionId).catch(() => null),
    ]);
    if (requestId !== detailRequestRef.current) {
      return;
    }
    setDetail(nextDetail);
    setSettingsState(
      nextSettings
        ? {
            status: "ready",
            extensionId,
            values: nextSettings.values,
            message: null,
          }
        : {
            status: "error",
            extensionId,
            values: {},
            message: null,
          },
    );
  }, []);

  const refresh = useCallback(
    async (selectedId?: string | null) => {
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
          setLogsState({ status: "idle", extensionId: null, content: "" });
          setSettingsState({ status: "idle", extensionId: null, values: {}, message: null });
        }
      } catch (err) {
        setError(String(err));
      } finally {
        setBusy(false);
      }
    },
    [loadExtensionDetail],
  );

  useEffect(() => {
    void refresh();
  }, [refresh]);

  useEffect(() => {
    if (typeof EventSource === "undefined") {
      return;
    }
    const stream = createExtensionEventsStream();
    const handleChanged = () => {
      void refresh(selectedIdRef.current);
    };
    stream.addEventListener("extension.graph_swapped", handleChanged);
    stream.onerror = () => undefined;
    return () => {
      stream.removeEventListener("extension.graph_swapped", handleChanged);
      stream.close();
    };
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

  function updateSettingValue(key: string, value: string | number | boolean) {
    setSettingsState((current) => ({
      ...current,
      values: {
        ...current.values,
        [key]: value,
      },
      message: null,
    }));
  }

  async function handleSaveSettings() {
    if (!selected || !detail) {
      return;
    }
    const payload = Object.fromEntries(
      selectedSettings.flatMap((field) => {
        const currentValue = settingsState.values[field.key];
        if (currentValue === undefined) {
          return [];
        }
        return [[field.key, currentValue] as const];
      }),
    );

    setSettingsState((current) => ({
      ...current,
      status: "saving",
      message: null,
    }));
    try {
      const saved = await saveExtensionSettings(selected.id, payload);
      setSettingsState({
        status: "ready",
        extensionId: selected.id,
        values: saved.values,
        message: t("web.extensions.settings_saved", "扩展配置已保存。"),
      });
    } catch (err) {
      setSettingsState((current) => ({
        ...current,
        status: "error",
        message: String(err),
      }));
    }
  }

  function openExtensionRoute(kind: "page" | "panel", id: string, label: string) {
    if (!workbenchApi) {
      setError(t("web.extensions.open_page_unavailable", "工作台尚未就绪，无法打开扩展视图。"));
      return;
    }
    workbenchApi.addPanel({
      id: `route:extension-${kind}:${id}:${Date.now().toString(36)}`,
      title: label,
      component: "route",
      params: {
        routeId: id,
        href: kind === "page"
          ? `/extension-pages/${encodeURIComponent(id)}`
          : `/extension-panels/${encodeURIComponent(id)}`,
        label,
        source: "extension",
      },
    });
  }

  const selectedPages = useMemo(
    () =>
      runtime?.registry.pages
        .filter((page) => page.extension_id === selected?.id)
        .map((page) => ({
          id: page.page.id,
          title: resolveText(page.page.title),
          mount: page.page.mount,
        })) ?? [],
    [resolveText, runtime?.registry.pages, selected?.id],
  );

  const selectedPanels = useMemo(
    () =>
      runtime?.registry.panels
        .filter((panel) => panel.extension_id === selected?.id)
        .map((panel) => ({
          id: panel.panel.id,
          title: resolveText(panel.panel.title),
          mount: panel.panel.mount,
          slot: panel.panel.slot,
        })) ?? [],
    [resolveText, runtime?.registry.panels, selected?.id],
  );

  const filteredExtensions = useMemo(() => {
    const normalizedQuery = query.trim().toLowerCase();
    return [...extensions]
      .filter((extension) => {
        if (statusFilter === "ready" && extension.status.toLowerCase() !== "ready") {
          return false;
        }
        if (statusFilter === "error" && !["failed", "error"].includes(extension.status.toLowerCase())) {
          return false;
        }
        if (statusFilter === "disabled" && extension.enabled) {
          return false;
        }
        if (!normalizedQuery) {
          return true;
        }
        return [
          extension.name,
          extension.id,
          extension.source_mode,
          extension.status,
        ].join("\n").toLowerCase().includes(normalizedQuery);
      })
      .sort((left, right) => {
        const weightDiff = extensionSortWeight(left) - extensionSortWeight(right);
        if (weightDiff !== 0) {
          return weightDiff;
        }
        return left.name.localeCompare(right.name);
      });
  }, [extensions, query, statusFilter]);

  const totalReady = extensions.filter((extension) => extension.status.toLowerCase() === "ready").length;
  const totalError = extensions.filter((extension) => ["failed", "error"].includes(extension.status.toLowerCase())).length;
  const totalDisabled = extensions.filter((extension) => !extension.enabled).length;
  const logsButtonLabel = selected && logsState.extensionId === selected.id && logsState.status === "ready"
    ? t("web.action.refresh", "刷新")
    : t("web.extensions.view_logs", "查看诊断文本");
  const selectedConversationResourceCount = selectedConversation?.resources.length ?? 0;
  const selectedConversationOperationCount = selectedConversation?.operations.length ?? 0;
  const selectedCapabilityCount = selectedViews.length + selectedOperations.length + selectedEvents.length;
  const selectedVersion = detail?.version ?? t("web.common.none", "无");
  const selectedGeneration = detail?.generation ?? t("web.common.none", "无");
  const hasDiagnostics = selectedDiagnostics.length > 0;
  const allOperationsAgent = selectedOperations.length > 0 && selectedOperations.every((operation) => operation.agent);
  const allOperationsScheduled = selectedOperations.length > 0 && selectedOperations.every((operation) => operation.schedule);
  const sharedOperationProvider = sharedOperationValue(selectedOperations.map((operation) => operation.provider?.kind));
  const sharedOperationInput = sharedOperationValue(selectedOperations.map((operation) => operation.input));
  const sharedOperationOutput = sharedOperationValue(selectedOperations.map((operation) => operation.output));
  const showAgentOperationTags = selectedOperations.some((operation) => operation.agent) && !allOperationsAgent;
  const showScheduleOperationTags = selectedOperations.some((operation) => operation.schedule) && !allOperationsScheduled;
  const showProviderOperationTags = selectedOperations.some((operation) => operation.provider) && !sharedOperationProvider;
  const showInputOperationTags = selectedOperations.some((operation) => operation.input) && !sharedOperationInput;
  const showOutputOperationTags = selectedOperations.some((operation) => operation.output) && !sharedOperationOutput;

  return (
    <div className="extensions-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel extensions-toolbar extensions-toolbar--compact">
        <div className="extensions-toolbar__row">
          <div className="page-heading">
            <span>{t("web.extensions.eyebrow", "Extensions")}</span>
            <h1>{t("web.extensions.title", "扩展是系统级能力包。")}</h1>
          </div>
          <div className="extensions-toolbar__side">
            <div className="extensions-overview-strip" aria-label={t("web.extensions.runtime_overview", "运行概览")}>
              <span><strong>{extensions.length}</strong>{t("web.extensions.summary_total", "扩展总数")}</span>
              <span><strong>{totalReady}</strong>{t("web.extensions.summary_ready", "就绪")}</span>
              <span><strong>{totalError}</strong>{t("web.extensions.summary_error", "异常")}</span>
              <span><strong>{totalDisabled}</strong>{t("web.extensions.summary_disabled", "停用")}</span>
            </div>
            <button type="button" className="secondary" onClick={() => void refresh(selected?.id)} disabled={busy}>
              {busy ? t("web.common.loading", "加载中...") : t("web.action.rescan", "重新扫描")}
            </button>
          </div>
        </div>
      </section>

      <div className="extensions-shell">
        <section className="work-panel extensions-catalog-panel">
          <div className="extensions-section__header">
            <div className="page-heading">
              <span>{t("web.extensions.catalog", "扩展目录")}</span>
              <h1>{t("web.extensions.catalog_title", "按状态定位扩展")}</h1>
            </div>
            <span className="extensions-catalog-count">
              {`${filteredExtensions.length} ${t("web.extensions.catalog_count", "项")}`}
            </span>
          </div>

          <div className="extensions-catalog-toolbar">
            <input
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder={t("web.extensions.search_placeholder", "搜索扩展名称、ID、状态或来源")}
            />
            <div className="extensions-filter-tabs">
              {[
                ["all", t("web.extensions.filter_all", "全部")],
                ["ready", t("web.extensions.filter_ready", "就绪")],
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
              filteredExtensions.map((extension) => (
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
                      <span>{localizeSourceMode(extension.source_mode, t)}</span>
                      <span>{formatRelativePath(extension.source_root)}</span>
                    </div>
                  </button>
                </article>
              ))
            )}
          </div>
        </section>

        <aside className="work-panel extensions-detail-panel">
          {selected ? (
            <div className="extensions-detail">
              <section className="extensions-detail-hero">
                <div className="extensions-detail-hero__content">
                  <div className="extensions-detail-hero__meta">
                    <span className={`badge ${statusBadgeClass(selected.status)}`}>
                      {localizeExtensionStatus(selected.status, t)}
                    </span>
                    <span className="badge badge--muted">{localizeSourceMode(selected.source_mode, t)}</span>
                  </div>
                  <h2>{selected.name}</h2>
                  {detail?.description ? <p>{detail.description}</p> : null}
                </div>
                <div className="extensions-detail-hero__actions">
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void handleAction(selected.enabled ? "disable" : "enable")}
                    disabled={actionBusy === "enable" || actionBusy === "disable"}
                  >
                    {selected.enabled ? t("web.common.disable", "停用") : t("web.common.enable", "启用")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void handleAction("reload")}
                    disabled={actionBusy === "reload"}
                  >
                    {t("web.action.reload", "重载")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => void handleAction("restart")}
                    disabled={actionBusy === "restart"}
                  >
                    {t("web.action.restart", "重启")}
                  </button>
                </div>
                <div className="extensions-detail-hero__facts">
                  <span><strong>{selectedVersion}</strong>{t("web.extensions.version", "版本")}</span>
                  <span><strong>{selectedCapabilityCount}</strong>{t("web.extensions.capability_total", "能力")}</span>
                  <span><strong>{selectedSettings.length}</strong>{t("web.extensions.settings", "配置")}</span>
                  <span>
                    <strong>{selectedConversation?.visible ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</strong>
                    {t("web.extensions.conversation", "会话")}
                  </span>
                  <span><strong>{selectedDiagnostics.length}</strong>{t("web.extensions.issues", "问题")}</span>
                </div>
              </section>

              <section className="extensions-section extensions-section--compact">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.capabilities", "能力清单")}</div>
                  </div>
                  <div className="extensions-section__meta">
                    {allOperationsAgent ? <span className="badge badge--muted">agent</span> : null}
                    {allOperationsScheduled ? <span className="badge badge--muted">schedule</span> : null}
                    {sharedOperationProvider ? <span className="badge badge--muted">{`provider:${sharedOperationProvider}`}</span> : null}
                    {sharedOperationInput ? <span className="badge badge--muted">{`in:${sharedOperationInput}`}</span> : null}
                    {sharedOperationOutput ? <span className="badge badge--muted">{`out:${sharedOperationOutput}`}</span> : null}
                  </div>
                </div>

                {selectedViews.length > 0 ? (
                  <div className="extensions-view-list">
                    {selectedViews.map((view) => {
                      const page = selectedPages.find((item) => item.id === view.name);
                      const panel = selectedPanels.find((item) => item.id === view.name);
                      const title = resolveText(view.title);
                      return (
                        <article key={view.name} className="mini-card extensions-view-card">
                          <div className="extensions-view-card__body">
                            <div className="extensions-view-card__title">
                              <strong>{title}</strong>
                              <span className="badge badge--muted">{view.type}</span>
                            </div>
                            <p>{view.name}</p>
                            <div className="extensions-inline-meta">
                              {view.route ? <span>{view.route}</span> : null}
                              {view.slot ? <span>{view.slot}</span> : null}
                            </div>
                          </div>
                          {page ? (
                            <button type="button" className="secondary" onClick={() => openExtensionRoute("page", page.id, page.title)}>
                              {t("web.extensions.open_view", "打开")}
                            </button>
                          ) : null}
                          {panel ? (
                            <button type="button" className="secondary" onClick={() => openExtensionRoute("panel", panel.id, panel.title)}>
                              {t("web.extensions.open_view", "打开")}
                            </button>
                          ) : null}
                        </article>
                      );
                    })}
                  </div>
                ) : null}

                {selectedOperations.length === 0 ? (
                  <div className="extensions-compact-empty">
                    <strong>{t("web.extensions.operations_empty_title", "没有操作")}</strong>
                  </div>
                ) : (
                  <div className="extensions-operation-list">
                    {selectedOperations.map((operation) => {
                      const operationTitle = operation.title ? resolveText(operation.title) : "";
                      const hasDistinctTitle = Boolean(operationTitle && operationTitle !== operation.name);
                      return (
                        <article key={operation.name} className="extensions-operation-row">
                          <div className="extensions-operation-row__main">
                            <strong>{operation.name}</strong>
                            {hasDistinctTitle ? <span>{operationTitle}</span> : null}
                            {operation.description ? <small>{operation.description}</small> : null}
                          </div>
                          <div className="extensions-operation-row__tags">
                            {showAgentOperationTags && operation.agent ? <span className="chip chip--active">agent</span> : null}
                            {showScheduleOperationTags && operation.schedule ? <span className="chip chip--active">schedule</span> : null}
                            {showProviderOperationTags && operation.provider ? <span className="chip chip--active">{`provider:${operation.provider.kind}`}</span> : null}
                            {showInputOperationTags && operation.input ? <span className="chip">{`in:${operation.input}`}</span> : null}
                            {showOutputOperationTags && operation.output ? <span className="chip">{`out:${operation.output}`}</span> : null}
                          </div>
                        </article>
                      );
                    })}
                  </div>
                )}

                <div className="extensions-detail-inline">
                  {selectedEvents.length > 0 ? (
                    <details className="extensions-detail-drawer">
                      <summary>
                        <span>{t("web.extensions.events", "事件")}</span>
                        <strong>{selectedEvents.length}</strong>
                      </summary>
                      <div className="kv-list extensions-kv-list extensions-kv-list--compact">
                        {selectedEvents.flatMap((event) => [
                          <span key={`${event.on}:on`}>{event.on}</span>,
                          <strong key={`${event.on}:operation`}>{event.operation}</strong>,
                        ])}
                      </div>
                    </details>
                  ) : null}

                  {selectedConversation?.visible || selectedConversationResourceCount > 0 || selectedConversationOperationCount > 0 ? (
                    <details className="extensions-detail-drawer">
                      <summary>
                        <span>{t("web.extensions.conversation_context", "会话上下文")}</span>
                        <strong>{selectedConversation?.visible ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</strong>
                      </summary>
                      <div className="extensions-conversation-compact">
                        <div>
                          <span>{t("web.extensions.conversation_resources", "资源")}</span>
                          <div className="extensions-chip-row">
                            {selectedConversation?.resources.length ? selectedConversation.resources.map((item) => (
                              <span key={item} className="chip chip--active">{item}</span>
                            )) : (
                              <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                            )}
                          </div>
                        </div>
                        <div>
                          <span>{t("web.extensions.conversation_operations", "操作")}</span>
                          <div className="extensions-chip-row">
                            {selectedConversation?.operations.length ? selectedConversation.operations.map((item) => (
                              <span key={item} className="chip chip--active">{item}</span>
                            )) : (
                              <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                            )}
                          </div>
                        </div>
                      </div>
                    </details>
                  ) : null}
                </div>
              </section>

              {selectedSettings.length > 0 ? (
                <section className="extensions-section">
                  <div className="extensions-section__header">
                    <div className="stack">
                      <div className="panel-title">{t("web.extensions.settings", "配置")}</div>
                    </div>
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => void handleSaveSettings()}
                      disabled={settingsState.status === "loading" || settingsState.status === "saving"}
                    >
                      {settingsState.status === "saving"
                        ? t("web.common.loading", "加载中...")
                        : t("web.extensions.save_settings", "保存配置")}
                    </button>
                  </div>
                  {settingsState.message ? (
                    <div className={settingsState.status === "error" ? "error" : "helper-text"}>
                      {settingsState.message}
                    </div>
                  ) : null}
                  <div className="form-grid extensions-settings-grid">
                    {selectedSettings.map((field) => {
                      const currentValue = settingsState.values[field.key] ?? field.default_value;
                      return (
                        <label
                          key={field.key}
                          className={`extensions-setting-field ${field.type === "textarea" ? "extensions-setting-field--wide" : ""}`}
                        >
                          <span className="extensions-setting-field__label">{resolveText(field.label)}</span>
                          {field.description ? (
                            <small className="extensions-setting-field__help">{resolveText(field.description)}</small>
                          ) : null}
                          {field.type === "textarea" ? (
                            <textarea
                              rows={4}
                              value={typeof currentValue === "string" ? currentValue : ""}
                              placeholder={field.placeholder ?? ""}
                              onChange={(event) => updateSettingValue(field.key, event.target.value)}
                            />
                          ) : null}
                          {field.type === "text" ? (
                            <input
                              value={typeof currentValue === "string" ? currentValue : ""}
                              placeholder={field.placeholder ?? ""}
                              onChange={(event) => updateSettingValue(field.key, event.target.value)}
                            />
                          ) : null}
                          {field.type === "number" ? (
                            <input
                              type="number"
                              value={typeof currentValue === "number" ? String(currentValue) : "0"}
                              onChange={(event) => updateSettingValue(field.key, Number(event.target.value || 0))}
                            />
                          ) : null}
                          {field.type === "select" ? (
                            <select
                              value={typeof currentValue === "string" ? currentValue : ""}
                              onChange={(event) => updateSettingValue(field.key, event.target.value)}
                            >
                              {field.options.map((option) => (
                                <option key={option.value} value={option.value}>
                                  {resolveText(option.label)}
                                </option>
                              ))}
                            </select>
                          ) : null}
                          {field.type === "boolean" ? (
                            <span className="check-row">
                              <input
                                type="checkbox"
                                checked={Boolean(currentValue)}
                                onChange={(event) => updateSettingValue(field.key, event.target.checked)}
                              />
                              <span>{t("web.extensions.boolean_enabled", "启用")}</span>
                            </span>
                          ) : null}
                        </label>
                      );
                    })}
                  </div>
                </section>
              ) : null}

              <section className="extensions-section extensions-section--compact">
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.runtime_diagnostics", "启动检查")}</div>
                  </div>
                  <span className={`badge ${hasDiagnostics ? "badge--warn" : statusBadgeClass(selectedHealth)}`}>
                    {hasDiagnostics ? selectedDiagnostics.length : localizeExtensionStatus(selectedHealth, t)}
                  </span>
                </div>
                {!hasDiagnostics ? (
                  <div className="extensions-status-line">
                    <span>{t("web.extensions.diagnostics_empty", "扫描与注册未发现问题。")}</span>
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

                <details className="extensions-detail-drawer extensions-technical-drawer">
                  <summary>
                    <span>{t("web.extensions.advanced", "技术信息")}</span>
                    <strong>{`${t("web.extensions.generation", "扫描批次")} ${selectedGeneration}`}</strong>
                  </summary>
                  <div className="kv-list extensions-kv-list">
                    <span>ID</span><strong>{selected.id}</strong>
                    <span>{t("web.extensions.version", "版本")}</span><strong>{selectedVersion}</strong>
                    <span>{t("web.extensions.compat", "兼容要求")}</span><strong>{selectedCompat?.ennoia ?? t("web.common.none", "无")}</strong>
                    <span>{t("web.common.status", "状态")}</span><strong>{localizeExtensionStatus(selected.status, t)}</strong>
                    <span>{t("web.extensions.health", "健康状态")}</span><strong>{localizeExtensionStatus(selectedHealth, t)}</strong>
                    <span>{t("web.extensions.generation", "扫描批次")}</span><strong>{selectedGeneration}</strong>
                    <span>{t("web.extensions.install_dir", "扩展目录")}</span><strong>{formatRelativePath(selected.install_dir)}</strong>
                    <span>{t("web.extensions.source_root", "来源目录")}</span><strong>{formatRelativePath(selected.source_root)}</strong>
                    <span>{t("web.extensions.docs", "文档入口")}</span><strong>{detail?.docs ? formatRelativePath(detail.docs) : t("web.common.none", "无")}</strong>
                  </div>
                </details>

                {hasDiagnostics ? (
                  <>
                    <div className="extensions-log-request">
                      <div>
                        <strong>{t("web.extensions.logs", "诊断文本")}</strong>
                        <span>{t("web.extensions.logs_description", "把当前诊断条目整理为文本，方便复制排查。")}</span>
                      </div>
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => void loadLogs(selected.id)}
                        disabled={logsState.status === "loading"}
                      >
                        {logsState.status === "loading" ? t("web.common.loading", "加载中...") : logsButtonLabel}
                      </button>
                    </div>
                    {logsState.status === "loading" ? (
                      <div className="extensions-status-line">
                        <strong>{t("web.extensions.log_loading_title", "正在读取诊断")}</strong>
                        <span>{t("web.extensions.log_loading", "正在生成诊断文本。")}</span>
                      </div>
                    ) : logsState.status === "error" ? (
                      <div className="error">{logsState.content}</div>
                    ) : logsState.status === "ready" ? (
                      <div className="extensions-log-panel">
                        <div className="extensions-log-panel__meta">
                          <strong>{selected.name}</strong>
                          <span>{`${t("web.extensions.log_for", "诊断文本属于")} ${selected.name}`}</span>
                        </div>
                        <pre className="log-view extensions-log-view">{logsState.content}</pre>
                      </div>
                    ) : null}
                  </>
                ) : null}
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
