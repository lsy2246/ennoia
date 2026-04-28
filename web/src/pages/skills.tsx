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
  const docsCount = skills.filter((item) => Boolean(item.docs)).length;
  const totalAssignments = [...assignmentMap.values()].reduce((sum, current) => sum + current.length, 0);

  return (
    <div className="skills-page">
      <section className="work-panel skills-toolbar">
        <div className="skills-toolbar__row">
          <div className="page-heading">
            <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
            <h1>{t("web.skills.title", "技能是工具与用法定义，由具体 Agent 选择启用。")}</h1>
            <p>{t("web.skills.description", "Web 只负责发现、查看来源、重新扫描和分配给 Agent，不直接编辑技能目录内容。")}</p>
          </div>
          <div className="skills-toolbar__actions">
            <button type="button" className="secondary" onClick={() => void refresh()}>
              {t("web.action.rescan", "重新扫描")}
            </button>
          </div>
        </div>
        {error ? <div className="error">{error}</div> : null}
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
            <span>{t("web.skills.summary_docs", "带文档")}</span>
            <strong>{docsCount}</strong>
            <small>{t("web.skills.docs", "文档")}</small>
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
            <h1>{t("web.skills.catalog_title", "按技能查看来源与分配")}</h1>
            <p>{t("web.skills.catalog_description", "每个技能展示来源、入口、文档和关键词；你可以直接把它分配给某个 Agent。")}</p>
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
                    <strong>{skill.display_name}</strong>
                    <small>{skill.id}</small>
                  </div>
                  <span className={`badge ${skill.enabled ? "badge--success" : "badge--muted"}`}>
                    {skill.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
                  </span>
                </div>

                <p className="skills-card__description">{skill.description || t("web.common.none", "无")}</p>

                <div className="skills-meta-grid">
                  <div className="skills-meta-item">
                    <span>{t("web.skills.source", "来源")}</span>
                    <strong>{formatRelativePath(skill.source)}</strong>
                  </div>
                  <div className="skills-meta-item">
                    <span>{t("web.skills.entry", "入口")}</span>
                    <strong>{skill.entry ? formatRelativePath(skill.entry) : t("web.common.none", "无")}</strong>
                  </div>
                  <div className="skills-meta-item">
                    <span>{t("web.skills.docs", "文档")}</span>
                    <strong>{skill.docs ? formatRelativePath(skill.docs) : t("web.common.none", "无")}</strong>
                  </div>
                </div>

                <div className="skills-card__section">
                  <div className="skills-subtitle">{t("web.skills.keywords", "关键词")}</div>
                  {skill.keywords.length === 0 ? (
                    <div className="empty-card skills-inline-empty">{t("web.skills.keywords_empty", "这个技能没有声明路由关键词。")}</div>
                  ) : (
                    <div className="chip-grid">
                      {skill.keywords.map((item) => (
                        <span key={item} className="chip chip--active">{item}</span>
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
