import { useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { listAgents, type AgentProfile } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function AgentDetailPage() {
  const { agentId } = useParams({ from: "/shell/agents/$agentId" });
  const { t } = useUiHelpers();
  const [agent, setAgent] = useState<AgentProfile | null>(null);

  useEffect(() => {
    void listAgents().then((items) => {
      setAgent(items.find((item) => item.id === agentId) ?? null);
    });
  }, [agentId]);

  if (!agent) {
    return <div className="page">{t("shell.action.loading", "加载中…")}</div>;
  }

  return (
    <div className="page">
      <section className="surface-panel">
        <h1>{agent.display_name}</h1>
        <dl className="data-pairs">
          <dt>ID</dt>
          <dd>{agent.id}</dd>
          <dt>{t("shell.agents.model", "模型")}</dt>
          <dd>{agent.default_model}</dd>
          <dt>{t("shell.agents.workspace", "工作区")}</dt>
          <dd>{agent.workspace_dir}</dd>
          <dt>{t("shell.extensions.skills", "技能目录")}</dt>
          <dd>{agent.skills_dir}</dd>
          <dt>{t("shell.agents.artifacts", "产物目录")}</dt>
          <dd>{agent.artifacts_dir}</dd>
        </dl>
      </section>
    </div>
  );
}
