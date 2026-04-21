import { useEffect, useState } from "react";

import {
  listProviders,
  type ProviderConfig,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function Providers() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [channels, setChannels] = useState<ProviderConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      setChannels(await listProviders());
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.channels.eyebrow", "API 上游渠道")}</span>
          <h1>{t("web.channels.title", "API 上游渠道是 Agent 访问模型能力的具体渠道实例。")}</h1>
          <p>{t("web.channels.description", "接口类型只在创建渠道时选择；日常使用和绑定都围绕渠道实例展开。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="button-row">
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "api-channel",
                entityId: `new-${Date.now()}`,
                title: t("web.channels.new", "新建渠道"),
                subtitle: t("web.channels.edit", "编辑 API 上游渠道"),
              })}
          >
            {t("web.channels.new", "新建渠道")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="card-grid">
          {channels.map((channel) => (
            <article key={channel.id} className="resource-card">
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

