import { Link } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { listAgents, type AgentProfile } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function AgentsPage() {
  const { t } = useUiHelpers();
  const [agents, setAgents] = useState<AgentProfile[]>([]);

  useEffect(() => {
    void listAgents().then(setAgents);
  }, []);

  return (
    <div className="page">
      <PageHeader
        title={t("shell.agents.page_title", "Agent")}
        description={t(
          "shell.agents.page_description",
          "Agent 是可长期配置的协作者档案，包含默认模型、工作区、技能目录和默认参与策略。",
        )}
      />

      <div className="stack-list">
        {agents.map((agent) => (
          <article key={agent.id} className="thread-card">
            <div>
              <div className="thread-card__title">
                <Link to="/agents/$agentId" params={{ agentId: agent.id }}>
                  {agent.display_name}
                </Link>
              </div>
              <p>
                {agent.default_model} · {agent.workspace_mode}
              </p>
            </div>
          </article>
        ))}
      </div>
    </div>
  );
}
