import { useEffect, useState } from "react";

import {
  listProviders,
  type ProviderConfig,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useProvidersStore } from "@/stores/providers";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function Providers({ embedded = false }: { embedded?: boolean }) {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const providersRevision = useProvidersStore((state) => state.revision);
  const [channels, setChannels] = useState<ProviderConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, [providersRevision]);

  async function refresh() {
    setError(null);
    try {
      setChannels(await listProviders());
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div
      className={`resource-layout resource-layout--single ${embedded ? "providers-shell providers-shell--embedded" : ""}`}
    >
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className={embedded ? "providers-panel providers-panel--embedded" : "work-panel"}>
        {embedded ? (
          <div className="providers-embedded-header">
            <span className="settings-panel__eyebrow">{t("web.channels.embedded_eyebrow", "模型渠道")}</span>
            <div className="panel-title">{t("web.channels.embedded_title", "渠道实例")}</div>
            <p className="helper-text">
              {t(
                "web.channels.embedded_description",
                "在这里维护模型访问入口，日常绑定和调用都围绕具体渠道实例展开。",
              )}
            </p>
          </div>
        ) : (
          <div className="page-heading">
            <span>{t("web.channels.eyebrow", "API 上游渠道")}</span>
            <h1>{t("web.channels.title", "API 上游渠道是 Agent 访问模型能力的具体渠道实例。")}</h1>
            <p>{t("web.channels.description", "接口类型只在创建渠道时选择；日常使用和绑定都围绕渠道实例展开。")}</p>
          </div>
        )}
        <div className={`button-row ${embedded ? "button-row--wrap" : ""}`}>
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "api-channel",
                entityId: `new-${Date.now()}`,
                title: t("web.channels.new", "新建渠道"),
                titleKey: "web.channels.new",
                titleFallback: "新建渠道",
                subtitle: t("web.channels.edit", "编辑 API 上游渠道"),
                subtitleKey: "web.channels.edit",
                subtitleFallback: "编辑 API 上游渠道",
              })}
          >
            {t("web.channels.new", "新建渠道")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className={`card-grid ${embedded ? "providers-card-grid--embedded" : ""}`}>
          {channels.map((channel) => (
            <article key={channel.id} className={`resource-card ${embedded ? "providers-card--embedded" : ""}`}>
              <header>
                <strong>{channel.display_name}</strong>
                <span>{channel.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{channel.description || t("web.common.none", "无")}</p>
              <div className="tag-row">
                <span>{channel.kind}</span>
                <span>{channel.default_model}</span>
              </div>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "api-channel",
                      entityId: channel.id,
                      title: channel.display_name,
                      subtitle: channel.kind,
                    })}
                >
                  {t("web.action.open", "打开")}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

