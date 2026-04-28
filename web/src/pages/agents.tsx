import { useEffect, useState } from "react";

import {
  listAgents,
  listProviders,
  type AgentProfile,
  type ProviderConfig,
} from "@ennoia/api-client";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function Agents() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const [nextAgents, nextProviders] = await Promise.all([listAgents(), listProviders()]);
      setAgents(nextAgents);
      setProviders(nextProviders);
    } catch (err) {
      setError(String(err));
    }
  }

  function providerLabel(providerId: string) {
    return providers.find((item) => item.id === providerId)?.display_name ?? providerId;
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.agents.eyebrow", "Agent Registry")}</span>
          <h1>{t("web.agents.title", "Agent 是可配置的协作者档案。")}</h1>
          <p>{t("web.agents.description", "从这里查看 Agent 清单，并把任意 Agent 作为独立工作视图打开。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="button-row">
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "agent",
                entityId: `new-${Date.now()}`,
                title: t("web.agents.new", "新建 Agent"),
                titleKey: "web.agents.new",
                titleFallback: "新建 Agent",
                subtitle: t("web.agents.edit", "编辑 Agent"),
                subtitleKey: "web.agents.edit",
                subtitleFallback: "编辑 Agent",
              })}
          >
            {t("web.agents.new", "新建 Agent")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="card-grid">
          {agents.map((agent) => (
            <article key={agent.id} className="resource-card">
              <header>
                <strong>{agent.display_name}</strong>
                <span className={`badge ${agent.enabled ? "badge--success" : "badge--muted"}`}>{agent.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{agent.description || t("web.common.none", "无")}</p>
              <div className="tag-row">
                <span>{providerLabel(agent.provider_id)}</span>
                <span>{agent.model_id}</span>
                <span className="badge badge--accent">{agent.skills.length} skills</span>
              </div>
              <p className="helper-text">
                {t("web.agents.working_dir_help", "Agent 工作目录自动派生到 agents/{agent_id}/work，无需单独配置。")}
                {" · "}
                {formatRelativePath(agent.working_dir || "")}
              </p>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "agent",
                      entityId: agent.id,
                      title: agent.display_name,
                      subtitle: providerLabel(agent.provider_id),
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


