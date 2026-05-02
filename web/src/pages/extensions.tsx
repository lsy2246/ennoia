import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import {
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

type ExtensionSettingsState = {
  status: "idle" | "loading" | "ready" | "saving" | "error";
  extensionId: string | null;
  values: Record<string, string | number | boolean>;
  message: string | null;
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

function takePreviewItems<T>(items: T[], limit = 4) {
  return items.slice(0, limit);
}

function localizeEntrypointKind(kind: string, t: (key: string, fallback: string) => string) {
  switch (kind) {
    case "page":
      return t("web.extensions.entrypoint_kind.page", "页面");
    case "panel":
      return t("web.extensions.entrypoint_kind.panel", "面板");
    default:
      return kind;
  }
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
    if (requestId === detailRequestRef.current) {
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
        setLogsState({ status: "idle", extensionId: null, content: "" });
        setSettingsState({ status: "idle", extensionId: null, values: {}, message: null });
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

  const selectedPageById = useMemo(
    () => new Map(selectedPages.map((page) => [page.id, page])),
    [selectedPages],
  );
  const selectedPanelById = useMemo(
    () => new Map(selectedPanels.map((panel) => [panel.id, panel])),
    [selectedPanels],
  );

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
  const selectedDiagnostics = detail?.diagnostics ?? selected?.diagnostics ?? [];
  const selectedHealth = detail?.health ?? selected?.status ?? t("web.common.unknown", "未知");
  const selectedEntrypoints = detail?.entrypoints ?? [];
  const selectedSettings = detail?.settings ?? [];
  const selectedCapabilityRows = detail?.capability_rows ?? [];
  const selectedResourceTypes = detail?.resource_types ?? [];
  const selectedCommands = detail?.commands ?? [];
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

  function openExtensionPanel(panelId: string, label: string) {
    if (!workbenchApi) {
      setError(t("web.extensions.open_page_unavailable", "工作台尚未就绪，无法打开扩展视图。"));
      return;
    }
    workbenchApi.addPanel({
      id: `route:extension-panel:${panelId}:${Date.now().toString(36)}`,
      title: label,
      component: "route",
      params: {
        routeId: panelId,
        href: `/extension-panels/${encodeURIComponent(panelId)}`,
        label,
        source: "extension",
      },
    });
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

  return (
    <div className="extensions-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
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
                        <span>{localizeExtensionStatus(extension.status, t)}</span>
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
                    <div className="panel-title">{t("web.extensions.entrypoints", "可用入口")}</div>
                    <p className="helper-text">{t("web.extensions.entrypoints_description", "这里列出扩展自己声明的可进入入口，用户先从这里进入，而不是先阅读底层运行信息。")}</p>
                  </div>
                  <span className="badge badge--muted">{selectedEntrypoints.length}</span>
                </div>
                {selectedEntrypoints.length === 0 ? (
                  <div className="empty-card extensions-empty-state">
                    <strong>{t("web.extensions.entrypoints_empty_title", "当前没有可进入入口")}</strong>
                    <p>{t("web.extensions.entrypoints_empty", "这个扩展还没有声明入口，或者入口尚未解析出来。")}</p>
                  </div>
                ) : (
                  <div className="extensions-entry-grid">
                    {selectedEntrypoints.map((entrypoint) => {
                      const page = entrypoint.page_id ? selectedPageById.get(entrypoint.page_id) : null;
                      const panel = entrypoint.panel_id ? selectedPanelById.get(entrypoint.panel_id) : null;
                      const label = resolveText(entrypoint.label);
                      const description = entrypoint.description
                        ? resolveText(entrypoint.description)
                        : page?.title ?? panel?.title ?? t("web.extensions.entrypoint_no_description", "这个入口没有额外说明。");
                      return (
                        <article key={entrypoint.id} className={`mini-card extensions-entry-card ${entrypoint.prominent ? "extensions-entry-card--prominent" : ""}`}>
                          <div className="extensions-entry-card__header">
                            <div className="stack">
                              <strong>{label}</strong>
                              <small>{entrypoint.id}</small>
                            </div>
                            <span className="badge badge--muted">{localizeEntrypointKind(entrypoint.kind, t)}</span>
                          </div>
                          <p>{description}</p>
                          <div className="extensions-inline-meta">
                            {page ? <span>{`${t("web.extensions.mount_point", "挂载点")} ${page.mount}`}</span> : null}
                            {panel ? <span>{`${t("web.extensions.slot", "槽位")} ${panel.slot}`}</span> : null}
                          </div>
                          <div className="button-row extensions-entry-card__footer">
                            <button
                              type="button"
                              className={entrypoint.prominent ? "" : "secondary"}
                              onClick={() => {
                                if (entrypoint.kind === "page" && entrypoint.page_id) {
                                  openExtensionPage(entrypoint.page_id, label);
                                }
                                if (entrypoint.kind === "panel" && entrypoint.panel_id) {
                                  openExtensionPanel(entrypoint.panel_id, label);
                                }
                              }}
                              disabled={
                                (entrypoint.kind === "page" && !entrypoint.page_id)
                                || (entrypoint.kind === "panel" && !entrypoint.panel_id)
                              }
                            >
                              {t("web.extensions.enter_entrypoint", "进入")}
                            </button>
                          </div>
                        </article>
                      );
                    })}
                  </div>
                )}
              </section>

              {selectedSettings.length > 0 ? (
                <section className="extensions-section">
                  <div className="extensions-section__header">
                    <div className="stack">
                      <div className="panel-title">{t("web.extensions.settings", "配置")}</div>
                      <p className="helper-text">{t("web.extensions.settings_description", "这里显示扩展自己声明的可填写配置字段，适合放扩展级别的默认值和开关。")}</p>
                    </div>
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => void handleSaveSettings()}
                      disabled={settingsState.status === "loading" || settingsState.status === "saving"}
                    >
                      {settingsState.status === "saving"
                        ? t("web.common.loading", "加载中…")
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
                <div className="extensions-section__header">
                  <div className="stack">
                    <div className="panel-title">{t("web.extensions.advanced", "高级信息")}</div>
                    <p className="helper-text">{t("web.extensions.advanced_description", "把运行参数、挂载清单和协议摘要放到后面，只有在排查或核对时再看。")}</p>
                  </div>
                </div>
                <div className="kv-list extensions-kv-list">
                  <span>ID</span><strong>{selected.id}</strong>
                  <span>{t("web.common.status", "状态")}</span><strong>{localizeExtensionStatus(selected.status, t)}</strong>
                  <span>{t("web.extensions.health", "健康状态")}</span><strong>{localizeExtensionStatus(selectedHealth, t)}</strong>
                  <span>{t("web.extensions.generation", "版本代次")}</span><strong>{detail?.generation ?? t("web.common.none", "无")}</strong>
                  <span>{t("web.extensions.runtime_startup", "启动策略")}</span><strong>{detail?.runtime.startup ?? t("web.common.none", "无")}</strong>
                  <span>{t("web.extensions.runtime_timeout", "超时限制")}</span><strong>{detail ? `${detail.runtime.timeout_ms} ms` : t("web.common.none", "无")}</strong>
                  <span>{t("web.extensions.runtime_memory", "内存上限")}</span><strong>{detail ? `${detail.runtime.memory_limit_mb} MB` : t("web.common.none", "无")}</strong>
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
                  <article className="mini-card extensions-runtime-card extensions-runtime-card--wide">
                    <div className="extensions-runtime-card__header">
                      <strong>{t("web.extensions.permissions", "权限边界")}</strong>
                    </div>
                    {detail ? (
                      <div className="kv-list extensions-kv-list extensions-kv-list--compact">
                        <span>{t("web.extensions.permission_storage", "存储")}</span>
                        <strong>{detail.permissions.storage ?? t("web.common.none", "无")}</strong>
                        <span>SQLite</span>
                        <strong>{detail.permissions.sqlite ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</strong>
                        <span>{t("web.extensions.permission_network", "网络")}</span>
                        <strong>{detail.permissions.network.length}</strong>
                        <span>{t("web.extensions.permission_events", "事件")}</span>
                        <strong>{detail.permissions.events.length}</strong>
                        <span>{t("web.extensions.permission_fs", "文件")}</span>
                        <strong>{detail.permissions.fs.length}</strong>
                        <span>{t("web.extensions.permission_env", "环境变量")}</span>
                        <strong>{detail.permissions.env.length}</strong>
                      </div>
                    ) : (
                      <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                    )}
                  </article>
                </div>
                <div className="extensions-summary-grid">
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.capabilities", "能力声明")}</span>
                        <strong>{selectedCapabilityRows.length}</strong>
                      </div>
                    </div>
                    <div className="chip-grid extensions-summary-card__chips">
                      {takePreviewItems(selectedCapabilityRows).map((capability) => (
                        <span key={capability.id} className="chip chip--active">
                          {`${capability.title ? resolveText(capability.title) : capability.contract} · ${capability.kind}`}
                        </span>
                      ))}
                    </div>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.resource_types", "资源类型")}</span>
                        <strong>{selectedResourceTypes.length}</strong>
                      </div>
                    </div>
                    <div className="chip-grid extensions-summary-card__chips">
                      {takePreviewItems(selectedResourceTypes).map((resourceType) => (
                        <span key={resourceType.id} className="chip chip--active">
                          {resourceType.title ? resolveText(resourceType.title) : resourceType.id}
                        </span>
                      ))}
                    </div>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.commands", "命令")}</span>
                        <strong>{selectedCommands.length}</strong>
                      </div>
                    </div>
                    <div className="chip-grid extensions-summary-card__chips">
                      {takePreviewItems(selectedCommands).map((command) => (
                        <span key={command.id} className="chip chip--active">
                          {resolveText(command.title)}
                        </span>
                      ))}
                    </div>
                  </article>
                  <article className="extensions-summary-card">
                    <div className="extensions-summary-card__header">
                      <div className="extensions-summary-card__title">
                        <span>{t("web.extensions.conversation", "会话装配")}</span>
                        <strong>{detail?.conversation.inject ? t("web.common.yes", "是") : t("web.common.no", "否")}</strong>
                      </div>
                    </div>
                    <div className="chip-grid extensions-summary-card__chips">
                      {detail?.conversation.capabilities.length ? detail.conversation.capabilities.map((item) => (
                        <span key={item} className="chip chip--active">{item}</span>
                      )) : (
                        <span className="badge badge--muted">{t("web.common.none", "无")}</span>
                      )}
                    </div>
                  </article>
                </div>
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
