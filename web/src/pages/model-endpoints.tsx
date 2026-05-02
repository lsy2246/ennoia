import { useEffect, useState } from "react";

import {
  listModelEndpoints,
  type ModelEndpointConfig,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useModelEndpointsStore } from "@/stores/modelEndpoints";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function ModelEndpointsPage({ embedded = false }: { embedded?: boolean }) {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const modelEndpointsRevision = useModelEndpointsStore((state) => state.revision);
  const [modelEndpoints, setModelEndpoints] = useState<ModelEndpointConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, [modelEndpointsRevision]);

  async function refresh() {
    setError(null);
    try {
      setModelEndpoints(await listModelEndpoints());
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div
      className={`resource-layout resource-layout--single ${embedded ? "model-endpoints-shell model-endpoints-shell--embedded" : ""}`}
    >
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className={embedded ? "model-endpoints-panel model-endpoints-panel--embedded" : "work-panel"}>
        {embedded ? (
          <div className="model-endpoints-embedded-header">
            <span className="settings-panel__eyebrow">{t("web.model_endpoints.embedded_eyebrow", "模型接入")}</span>
            <div className="panel-title">{t("web.model_endpoints.embedded_title", "接入实例")}</div>
            <p className="helper-text">
              {t(
                "web.model_endpoints.embedded_description",
                "在这里维护模型访问入口，日常绑定和调用都围绕具体模型接入实例展开。",
              )}
            </p>
          </div>
        ) : (
          <div className="page-heading">
            <span>{t("web.model_endpoints.eyebrow", "模型接入")}</span>
            <h1>{t("web.model_endpoints.title", "模型接入是 Agent 访问模型能力的具体配置实例。")}</h1>
            <p>{t("web.model_endpoints.description", "模型提供方只在创建时选择；日常使用和绑定都围绕模型接入实例展开。")}</p>
          </div>
        )}
        <div className={`button-row ${embedded ? "button-row--wrap" : ""}`}>
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "model-endpoint",
                entityId: `new-${Date.now()}`,
                title: t("web.model_endpoints.new", "新建模型接入"),
                titleKey: "web.model_endpoints.new",
                titleFallback: "新建模型接入",
                subtitle: t("web.model_endpoints.edit", "编辑模型接入"),
                subtitleKey: "web.model_endpoints.edit",
                subtitleFallback: "编辑模型接入",
              })}
          >
            {t("web.model_endpoints.new", "新建模型接入")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className={`card-grid ${embedded ? "model-endpoints-card-grid--embedded" : ""}`}>
          {modelEndpoints.map((modelEndpoint) => (
            <article
              key={modelEndpoint.id}
              className={`resource-card ${embedded ? "model-endpoints-card--embedded" : ""}`}
            >
              <header>
                <strong>{modelEndpoint.display_name}</strong>
                <span>{modelEndpoint.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{modelEndpoint.description || t("web.common.none", "无")}</p>
              <div className="tag-row">
                <span>{modelEndpoint.kind}</span>
                <span>{modelEndpoint.default_model}</span>
              </div>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "model-endpoint",
                      entityId: modelEndpoint.id,
                      title: modelEndpoint.display_name,
                      subtitle: modelEndpoint.kind,
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


