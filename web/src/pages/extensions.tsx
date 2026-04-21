import { useEffect, useState } from "react";

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

export function Extensions() {
  const { t } = useUiHelpers();
  const [extensions, setExtensions] = useState<ExtensionRuntimeState[]>([]);
  const [selected, setSelected] = useState<ExtensionRuntimeState | null>(null);
  const [detail, setDetail] = useState<ExtensionDetail | null>(null);
  const [logs, setLogs] = useState("");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    const next = await listExtensions();
    setExtensions(next);
    const nextSelected = next.find((item) => item.id === selected?.id) ?? next[0] ?? null;
    setSelected(nextSelected);
    setDetail(nextSelected ? await getExtension(nextSelected.id).catch(() => null) : null);
  }

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
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  async function loadLogs(extensionId: string) {
    setLogs(await getExtensionLogs(extensionId));
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
        <button type="button" className="secondary" onClick={() => void refresh()}>{t("web.action.rescan", "重新扫描")}</button>
        <div className="card-grid">
          {extensions.map((extension) => (
            <article key={extension.id} className="resource-card" onClick={() => void selectExtension(extension)}>
              <header>
                <strong>{extension.name}</strong>
                <span>{extension.status}</span>
              </header>
              <p>{extension.id} · {extension.version}</p>
              <div className="tag-row">
                <span>{extension.kind}</span>
                <span>{extension.source_mode.toLowerCase()}</span>
                <span>{extension.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
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
              <span>{t("web.common.status", "状态")}</span><strong>{selected.status}</strong>
              <span>{t("web.extensions.install_dir", "扩展包目录")}</span><strong>{formatRelativePath(selected.install_dir)}</strong>
              <span>{t("web.extensions.source_root", "来源目录")}</span><strong>{formatRelativePath(selected.source_root)}</strong>
              <span>{t("web.extensions.diagnostics", "诊断")}</span><strong>{selected.diagnostics.length}</strong>
            </div>
            <div className="extension-summary-grid">
              <article className="memory-lane">
                <span>{t("web.extensions.contributes_ui", "贡献 UI")}</span>
                <strong>{detail?.frontend ? "frontend" : "—"}</strong>
                <small>{t("web.extensions.contributes_ui_help", "扩展可贡献页面、面板、主题和语言包。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.contributes_api", "贡献 API 上游接口")}</span>
                <strong>{detail?.backend?.status ?? "—"}</strong>
                <small>{t("web.extensions.contributes_api_help", "API 上游接口实现属于扩展能力，但这里只展示扩展包状态。")}</small>
              </article>
              <article className="memory-lane">
                <span>{t("web.extensions.package_state", "扩展包状态")}</span>
                <strong>{detail?.health ?? selected.status}</strong>
                <small>{detail?.generation ? `generation ${detail.generation}` : selected.version}</small>
              </article>
            </div>
            <div className="button-row">
              <button type="button" onClick={() => void handleAction(selected.enabled ? "disable" : "enable")}>{selected.enabled ? t("web.action.disable", "停用") : t("web.action.enable", "启用")}</button>
              <button type="button" className="secondary" onClick={() => void handleAction("reload")}>{t("web.action.reload", "重载")}</button>
              <button type="button" className="secondary" onClick={() => void handleAction("restart")}>{t("web.action.restart", "重启")}</button>
              <button type="button" className="secondary" onClick={() => void loadLogs(selected.id)}>{t("web.extensions.view_logs", "查看日志")}</button>
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

