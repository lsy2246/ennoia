import { useEffect, useMemo, useState } from "react";

import {
  listAgents,
  listSkills,
  updateAgent,
  type AgentProfile,
  type SkillConfig,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
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
  const singleEntryCount = skills.filter((item) => item.actions.length <= 1).length;
  const totalAssignments = [...assignmentMap.values()].reduce((sum, current) => sum + current.length, 0);

  return (
    <div className="skills-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel skills-toolbar">
        <div className="skills-toolbar__row">
          <div className="page-heading">
            <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
            <h1>{t("web.skills.title", "技能是给 Agent 和会话使用的能力包。")}</h1>
            <p>{t("web.skills.description", "这里优先看能力说明、触发方式和挂载状态；内部 action 与同步策略不再作为主信息。")}</p>
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
            <span>{t("web.skills.summary_single_entry", "单入口技能")}</span>
            <strong>{singleEntryCount}</strong>
            <small>{t("web.skills.single_entry", "统一入口")}</small>
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
            <h1>{t("web.skills.catalog_title", "按能力查看技能与分配")}</h1>
            <p>{t("web.skills.catalog_description", "每个技能展示用途、触发方式、挂载模式和已分配 Agent，帮助你判断该把它给谁用。")}</p>
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
            {skills.map((skill) => {
              const assignedAgents = assignmentMap.get(skill.id) ?? [];

              return (
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
                      <span>{t("web.skills.trigger", "触发词")}</span>
                      <strong>/{skill.id}</strong>
                    </div>
                    <div className="skills-meta-item">
                      <span>{t("web.skills.source", "挂载模式")}</span>
                      <strong>{skill.mount.mode}</strong>
                    </div>
                    <div className="skills-meta-item">
                      <span>{t("web.skills.assignment_count", "已分配")}</span>
                      <strong>
                        {assignedAgents.length > 0
                          ? `${assignedAgents.length} ${t("web.skills.agent_unit", "Agent")}`
                          : t("web.skills.assignment_none", "未分配")}
                      </strong>
                    </div>
                  </div>

                  <div className="skills-card__section">
                    <div className="skills-subtitle">{t("web.skills.how_to_use", "使用方式")}</div>
                    <div className="empty-card skills-inline-empty">
                      <strong>/{skill.id}</strong>
                      <p className="helper-text">
                        {t("web.skills.usage_hint", "在会话输入框里插入这个技能片段，系统再按 skill 自己的 README 约定去解释具体输入。")}
                      </p>
                    </div>
                  </div>

                  <div className="skills-card__section">
                    <div className="skills-subtitle">{t("web.skills.assigned_agents", "已启用到这些 Agent")}</div>
                    <div className="chip-grid">
                      {agents.map((agent) => {
                        const active = assignedAgents.includes(agent.id);
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
              );
            })}
          </div>
        )}
      </section>
    </div>
  );
}
