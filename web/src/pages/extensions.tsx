import { useCallback, useEffect, useMemo, useState } from "react";

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

export function Extensions() {
  const { resolveText, runtime, t } = useUiHelpers();
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const [extensions, setExtensions] = useState<ExtensionRuntimeState[]>([]);
  const [selected, setSelected] = useState<ExtensionRuntimeState | null>(null);
  const [detail, setDetail] = useState<ExtensionDetail | null>(null);
  const [logs, setLogs] = useState("");
  const [error, setError] = useState<string | null>(null);

  const refresh = useCallback(async (selectedId?: string | null) => {
    setError(null);
    const next = await listExtensions();
    setExtensions(next);
    const nextSelected = next.find((item) => item.id === selectedId) ?? next[0] ?? null;
    setSelected(nextSelected);
    setDetail(nextSelected ? await getExtension(nextSelected.id).catch(() => null) : null);
  }, []);

  useEffect(() => {
    void refresh();
  }, [refresh]);

  async function selectExtension(extension: ExtensionRuntimeState) {
    setSelected(extension);
    setDetail(await getExtension(extension.id).catch(() => null));
  }

  async function handleAction(action: "enable" | "disable" | "reload" | "restart") {
    if (!selected) {
      return;
    }
    setError(null);
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
      await refresh(selected.id);
    } catch (err) {
      setError(String(err));
    }
  }

  async function loadLogs(extensionId: string) {
    setLogs(await getExtensionLogs(extensionId));
  }

  const selectedPages = useMemo(
    () =>
      runtime?.registry.pages.filter((page) => page.extension_id === selected?.id) ?? [],
    [runtime?.registry.pages, selected?.id],
  );
  const surfaceCount = detail?.surfaces.length ?? 0;
  const capabilityCount = detail?.capability_rows.length ?? 0;
  const resourceTypeCount = detail?.resource_types.length ?? 0;
  const subscriptionCount = detail?.subscriptions.length ?? 0;

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
    <div className="resource-layout">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.extensions.eyebrow", "Extensions")}</span>
          <h1>{t("web.extensions.title", "扩展包是系统插件，不是技能。")}</h1>
          <p>{t("web.extensions.description", "这里按扩展包查看状态、贡献能力、重载和日志。来源目录只显示相对实例路径。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <button type="button" className="secondary" onClick={() => void refresh(selected?.id)}>{t("web.action.rescan", "重新扫描")}</button>
        <div className="card-grid">
          {extensions.map((extension) => (
            <article key={extension.id} className="resource-card" onClick={() => void selectExtension(extension)}>
              <header>
                <strong>{extension.name}</strong>
                <span className={`badge ${extension.status === "running" ? "badge--success" : extension.status === "error" ? "badge--danger" : "badge--muted"}`}>{extension.status}</span>
              </header>
              <p>{extension.id}</p>
              <div className="tag-row">
                <span>{extension.kind}</span>
                <span>{extension.source_mode.toLowerCase()}</span>
                <span className={extension.enabled ? "badge badge--success" : "badge badge--muted"}>{extension.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </div>
            </article>
          ))}
        </div>
      </section>

      <aside className="work-panel editor-form">
        <div className="panel-title">{t("web.extensions.details", "扩展详情")}</div>
        {selected ? (
          <>
            <div className="kv-list">
              <span>ID</span><strong>{selected.id}</strong>
              <span>{t("web.common.status", "状态")}</span><strong><span className={`badge ${selected.status === "running" ? "badge--success" : selected.status === "error" ? "badge--danger" : "badge--muted"}`}>{selected.status}</span></strong>
              <span>{t("web.extensions.install_dir", "扩展包目录")}</span><strong>{formatRelativePath(selected.install_dir)}</strong>
              <span>{t("web.extensions.source_root", "来源目录")}</span><strong>{formatRelativePath(selected.source_root)}</strong>
              <span>{t("web.extensions.diagnostics", "诊断")}</span><strong><span className={`badge ${selected.diagnostics.length > 0 ? "badge--warn" : "badge--muted"}`}>{selected.diagnostics.length}</span></strong>
            </div>
            <div className="extension-summary-grid">
              <article className="memory-lane">
                <span>{t("web.extensions.capabilities", "能力声明")}</span>
                <strong><span className={`badge ${capabilityCount > 0 ? "badge--success" : "badge--muted"}`}>{capabilityCount}</span></strong>
                <small>{t("web.extensions.capabilities_help", "manifest 的 capabilities 是系统能力入口；Provider、Interface、Memory 等视图都从这里派生。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.surfaces", "界面挂载")}</span>
                <strong><span className={`badge ${surfaceCount > 0 ? "badge--success" : "badge--muted"}`}>{surfaceCount}</span></strong>
                <small>{t("web.extensions.surfaces_help", "surfaces 统一表达 page、panel 等 UI 挂载点。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.subscriptions", "事件订阅")}</span>
                <strong><span className={`badge ${subscriptionCount > 0 ? "badge--success" : "badge--muted"}`}>{subscriptionCount}</span></strong>
                <small>{t("web.extensions.subscriptions_help", "subscriptions 只声明监听关系，实际 Hook 处理入口由 capability.entry 决定。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.resource_types", "资源类型")}</span>
                <strong><span className={`badge ${resourceTypeCount > 0 ? "badge--success" : "badge--muted"}`}>{resourceTypeCount}</span></strong>
                <small>{t("web.extensions.resource_types_help", "resource_types 用来声明扩展理解和产出的资源模型。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.package_state", "扩展包状态")}</span>
                <strong><span className={`badge ${(detail?.health ?? selected.status) === "running" ? "badge--success" : (detail?.health ?? selected.status) === "error" ? "badge--danger" : "badge--muted"}`}>{detail?.health ?? selected.status}</span></strong>
                <small>{detail?.generation ? `generation ${detail.generation}` : ""}</small>
              </article>
            </div>
            <div className="button-row">
              <button type="button" onClick={() => void handleAction(selected.enabled ? "disable" : "enable")}>{selected.enabled ? t("web.action.disable", "停用") : t("web.action.enable", "启用")}</button>
              <button type="button" className="secondary" onClick={() => void handleAction("reload")}>{t("web.action.reload", "重载")}</button>
              <button type="button" className="secondary" onClick={() => void handleAction("restart")}>{t("web.action.restart", "重启")}</button>
              <button type="button" className="secondary" onClick={() => void loadLogs(selected.id)}>{t("web.extensions.view_logs", "查看日志")}</button>
            </div>
            <div className="stack">
              <div className="panel-title">{t("web.extensions.pages", "扩展视图")}</div>
              {selectedPages.length === 0 ? (
                <div className="empty-card">{t("web.extensions.pages_empty", "这个扩展没有声明视图。")}</div>
              ) : (
                selectedPages.map((page) => {
                  const label = resolveText(page.page.title);
                  return (
                    <article key={page.page.id} className="mini-card">
                      <strong>{label}</strong>
                      <span className="badge badge--muted">{page.page.mount}</span>
                      <div className="button-row">
                        <button type="button" className="secondary" onClick={() => openExtensionPage(page.page.id, label)}>
                          {t("web.extensions.open_page", "打开视图")}
                        </button>
                      </div>
                    </article>
                  );
                })
              )}
            </div>
            <pre className="log-view">{logs || t("web.extensions.log_empty", "选择“查看日志”加载扩展日志。")}</pre>
          </>
        ) : (
          <div className="empty-card">{t("web.extensions.empty", "暂无扩展。")}</div>
        )}
      </aside>
    </div>
  );
}

