import { useEffect, useMemo, useState } from "react";

import {
  listAgents,
  listSkills,
  updateAgent,
  type AgentProfile,
  type SkillConfig,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";

export function Skills() {
  const { t } = useUiHelpers();
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

  const enabledCount = skills.filter((item) => item.enabled).length;
  const blockedCount = skills.filter((item) => item.builtin_sync_blocked).length;
  const totalAssignments = [...assignmentMap.values()].reduce((sum, current) => sum + current.length, 0);

  return (
    <div className="skills-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel skills-toolbar">
        <div className="skills-toolbar__row">
          <div className="page-heading">
            <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
            <h1>{t("web.skills.title", "技能是标准能力包，由具体 Agent 选择挂载。")}</h1>
            <p>{t("web.skills.description", "技能页只展示包定义、动作和挂载状态；具体调用说明统一写在 skill 自己的 README.md。")}</p>
          </div>
          <div className="skills-toolbar__actions">
            <button type="button" className="secondary" onClick={() => void refresh()}>
              {t("web.action.rescan", "重新扫描")}
            </button>
          </div>
        </div>
        <div className="skills-overview-grid">
          <article className="metric-card skills-metric-card">
            <span>{t("web.skills.summary_total", "技能总数")}</span>
            <strong>{skills.length}</strong>
            <small>{t("web.nav.skills", "技能")}</small>
          </article>
          <article className="metric-card skills-metric-card">
            <span>{t("web.skills.summary_enabled", "已启用")}</span>
            <strong>{enabledCount}</strong>
            <small>{t("web.common.enabled", "启用")}</small>
          </article>
          <article className="metric-card skills-metric-card">
            <span>{t("web.skills.summary_docs", "阻止内置同步")}</span>
            <strong>{blockedCount}</strong>
            <small>{t("web.skills.docs", "同步拦截")}</small>
          </article>
          <article className="metric-card skills-metric-card">
            <span>{t("web.skills.summary_assignments", "已分配")}</span>
            <strong>{totalAssignments}</strong>
            <small>{t("web.skills.assigned_agents", "Agent 分配")}</small>
          </article>
        </div>
      </section>

      <section className="work-panel skills-catalog-panel">
        <div className="skills-section__header">
          <div className="page-heading">
            <span>{t("web.skills.catalog", "技能目录")}</span>
            <h1>{t("web.skills.catalog_title", "按技能查看动作与分配")}</h1>
            <p>{t("web.skills.catalog_description", "每个技能展示版本、挂载模式、动作列表和内置同步状态；你可以直接把它分配给某个 Agent。")}</p>
          </div>
          <span className="skills-catalog-count">{`${skills.length} ${t("web.skills.catalog_count", "项")}`}</span>
        </div>

        {skills.length === 0 ? (
          <div className="empty-card skills-empty-state">
            <strong>{t("web.skills.empty_title", "还没有技能")}</strong>
            <p>{t("web.skills.empty_body", "重新扫描后这里会出现可供 Agent 引用的技能目录。")}</p>
          </div>
        ) : (
          <div className="skills-grid">
            {skills.map((skill) => (
              <article key={skill.id} className="resource-card skills-card">
                <div className="skills-card__header">
                  <div className="stack skills-card__title">
                    <strong>{skill.id}</strong>
                    <small>{skill.version}</small>
                  </div>
                  <span className={`badge ${skill.enabled ? "badge--success" : "badge--muted"}`}>
                    {skill.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
                  </span>
                </div>

                <p className="skills-card__description">{skill.description || t("web.common.none", "无")}</p>

                <div className="skills-meta-grid">
                  <div className="skills-meta-item">
                    <span>{t("web.skills.source", "挂载模式")}</span>
                    <strong>{skill.mount.mode}</strong>
                  </div>
                  <div className="skills-meta-item">
                    <span>{t("web.skills.entry", "动作数")}</span>
                    <strong>{skill.actions.length}</strong>
                  </div>
                  <div className="skills-meta-item">
                    <span>{t("web.skills.docs", "内置同步")}</span>
                    <strong>{skill.builtin_sync_blocked ? "blocked" : "follow"}</strong>
                  </div>
                </div>

                <div className="skills-card__section">
                  <div className="skills-subtitle">Actions</div>
                  {skill.actions.length === 0 ? (
                    <div className="empty-card skills-inline-empty">这个技能当前没有声明可执行动作。</div>
                  ) : (
                    <div className="stack">
                      {skill.actions.map((action) => (
                        <div key={action.id} className="resource-card">
                          <strong>{action.id}</strong>
                          <p className="helper-text">{action.description || "无说明"}</p>
                          <div className="chip-grid">
                            <span className="chip chip--active">{action.invoke_mode}</span>
                            {action.requires.map((item) => (
                              <span key={`${action.id}:${item}`} className="chip">{item}</span>
                            ))}
                          </div>
                          <small>{formatRelativePath(action.entry)}</small>
                        </div>
                      ))}
                    </div>
                  )}
                </div>

                <div className="skills-card__section">
                  <div className="skills-subtitle">{t("web.skills.assigned_agents", "已启用到这些 Agent")}</div>
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
              </article>
            ))}
          </div>
        )}
      </section>
    </div>
  );
}
