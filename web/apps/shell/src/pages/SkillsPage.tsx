import { useEffect, useMemo, useState } from "react";

import {
  listAgents,
  listSkills,
  updateAgent,
  type AgentProfile,
  type SkillConfig,
} from "@ennoia/api-client";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function SkillsPage() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [savingKey, setSavingKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  const assignmentMap = useMemo(() => {
    const next = new Map<string, string[]>();
    for (const skill of skills) {
      next.set(
        skill.id,
        agents.filter((agent) => agent.skills.includes(skill.id)).map((agent) => agent.id),
      );
    }
    return next;
  }, [agents, skills]);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const [nextSkills, nextAgents] = await Promise.all([listSkills(), listAgents()]);
      setSkills(nextSkills);
      setAgents(nextAgents);
    } catch (err) {
      setError(String(err));
    }
  }

  async function toggleAssignment(skillId: string, agent: AgentProfile) {
    const key = `${skillId}:${agent.id}`;
    setSavingKey(key);
    setError(null);
    try {
      const nextSkills = agent.skills.includes(skillId)
        ? agent.skills.filter((item) => item !== skillId)
        : [...agent.skills, skillId];
      await updateAgent(agent.id, {
        ...agent,
        skills: nextSkills,
      });
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setSavingKey(null);
    }
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
          <h1>{t("web.skills.title", "Skill 是能力包；是否启用由具体 Agent 决定。")}</h1>
          <p>{t("web.skills.description", "Web 只负责发现、查看来源、重新扫描和分配给 Agent，不直接编辑技能目录内容。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="button-row">
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.rescan", "重新扫描")}
          </button>
        </div>
        <div className="stack">
          {skills.map((skill) => (
            <article key={skill.id} className="resource-card">
              <header>
                <strong>{skill.display_name}</strong>
                <span>{skill.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{skill.description || t("web.common.none", "无")}</p>
              <div className="kv-list">
                <span>{t("web.skills.source", "来源")}</span>
                <strong>{formatRelativePath(skill.source)}</strong>
                <span>{t("web.skills.entry", "入口")}</span>
                <strong>{skill.entry ? formatRelativePath(skill.entry) : t("web.common.none", "无")}</strong>
              </div>
              <div className="stack">
                <div className="panel-title">{t("web.skills.assigned_agents", "已启用到这些 Agent")}</div>
                <div className="chip-grid">
                  {agents.map((agent) => {
                    const active = assignmentMap.get(skill.id)?.includes(agent.id) ?? false;
                    return (
                      <button
                        key={agent.id}
                        type="button"
                        className={active ? "chip chip--active" : "chip"}
                        disabled={savingKey === `${skill.id}:${agent.id}`}
                        onClick={() => void toggleAssignment(skill.id, agent)}
                      >
                        {agent.display_name}
                      </button>
                    );
                  })}
                </div>
              </div>
              <div className="button-row">
                {(assignmentMap.get(skill.id) ?? []).map((agentId) => {
                  const agent = agents.find((item) => item.id === agentId);
                  if (!agent) {
                    return null;
                  }
                  return (
                    <button
                      key={agent.id}
                      type="button"
                      className="secondary"
                      onClick={() =>
                        openView({
                          kind: "agent",
                          entityId: agent.id,
                          title: agent.display_name,
                          subtitle: agent.provider_id,
                        })}
                    >
                      {t("web.skills.open_agent", "打开 Agent")} {agent.display_name}
                    </button>
                  );
                })}
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}
