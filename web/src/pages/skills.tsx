import { useEffect, useMemo, useState } from "react";

import {
  getSkill,
  getSkillSettings,
  getSkillStatus,
  listSkills,
  runSkillCheck,
  saveSkillSettings,
  updateSkill,
  type SkillCheckCategory,
  type SkillCheckItemStatus,
  type SkillCheckResult,
  type SkillConfig,
  type SkillDiagnosticsSpec,
  type SkillReadinessSummary,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useUiHelpers } from "@/stores/ui";

type SkillDetailState = {
  status: "idle" | "loading" | "ready" | "error";
  skill: SkillConfig | null;
  values: Record<string, string | number | boolean>;
  readiness: SkillCheckResult | null;
  message: { tone: "success" | "error"; text: string } | null;
};

function fallbackReadiness(): SkillReadinessSummary {
  return {
    status: "unknown",
    summary: "",
    checked_at: null,
  };
}

function getSkillReadiness(skill: SkillConfig | null | undefined): SkillReadinessSummary {
  return skill?.readiness ?? fallbackReadiness();
}

function getSkillDiagnostics(skill: SkillConfig | null | undefined): SkillDiagnosticsSpec {
  return skill?.diagnostics ?? { manual_check: false, check: null };
}

function getSkillSettingFields(skill: SkillConfig | null | undefined) {
  return skill?.settings ?? [];
}

function readinessBadgeClass(status: SkillConfig["readiness"]["status"]) {
  switch (status) {
    case "ready":
      return "badge--success";
    case "partial":
      return "badge--warn";
    case "missing_config":
    case "env_missing":
    case "error":
      return "badge--danger";
    default:
      return "badge--muted";
  }
}

function localizeReadiness(
  status: SkillConfig["readiness"]["status"],
  t: (key: string, fallback: string) => string,
) {
  switch (status) {
    case "ready":
      return t("web.skills.readiness.ready", "已就绪");
    case "partial":
      return t("web.skills.readiness.partial", "部分就绪");
    case "missing_config":
      return t("web.skills.readiness.missing_config", "缺少配置");
    case "env_missing":
      return t("web.skills.readiness.env_missing", "环境未满足");
    case "error":
      return t("web.skills.readiness.error", "检测失败");
    default:
      return t("web.skills.readiness.unknown", "未检测");
  }
}

function localizeCheckCategory(
  category: SkillCheckCategory,
  t: (key: string, fallback: string) => string,
) {
  switch (category) {
    case "config":
      return t("web.skills.check_category.config", "配置");
    case "environment":
      return t("web.skills.check_category.environment", "环境");
    case "permission":
      return t("web.skills.check_category.permission", "权限");
    case "dependency":
      return t("web.skills.check_category.dependency", "依赖");
    case "connectivity":
      return t("web.skills.check_category.connectivity", "连通性");
    default:
      return category;
  }
}

function checkItemBadgeClass(status: SkillCheckItemStatus) {
  switch (status) {
    case "ok":
      return "badge--success";
    case "warning":
      return "badge--warn";
    case "missing":
    case "error":
      return "badge--danger";
    default:
      return "badge--muted";
  }
}

function localizeCheckItemStatus(
  status: SkillCheckItemStatus,
  t: (key: string, fallback: string) => string,
) {
  switch (status) {
    case "ok":
      return t("web.skills.check_item.ok", "通过");
    case "warning":
      return t("web.skills.check_item.warning", "注意");
    case "missing":
      return t("web.skills.check_item.missing", "缺失");
    case "error":
      return t("web.skills.check_item.error", "失败");
    case "skipped":
      return t("web.skills.check_item.skipped", "跳过");
    default:
      return status;
  }
}

export function Skills() {
  const { formatDateTime, resolveText, t } = useUiHelpers();
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [savingSkillId, setSavingSkillId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null);
  const [detailState, setDetailState] = useState<SkillDetailState>({
    status: "idle",
    skill: null,
    values: {},
    readiness: null,
    message: null,
  });
  const [savingSettings, setSavingSettings] = useState(false);
  const [checkingSkill, setCheckingSkill] = useState(false);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const nextSkills = await listSkills();
      setSkills(nextSkills);
    } catch (err) {
      setError(String(err));
    }
  }

  async function refreshSelectedSkill(skillId: string) {
    const nextSkill = await getSkill(skillId);
    setDetailState((current) => ({
      ...current,
      skill: nextSkill,
    }));
  }

  async function openSkillConfig(skillId: string) {
    setSelectedSkillId(skillId);
    setDetailState({
      status: "loading",
      skill: null,
      values: {},
      readiness: null,
      message: null,
    });
    try {
      const [skill, settings, readiness] = await Promise.all([
        getSkill(skillId),
        getSkillSettings(skillId),
        getSkillStatus(skillId),
      ]);
      setDetailState({
        status: "ready",
        skill,
        values: settings.values,
        readiness,
        message: null,
      });
    } catch (err) {
      setDetailState({
        status: "error",
        skill: null,
        values: {},
        readiness: null,
        message: {
          tone: "error",
          text: String(err),
        },
      });
    }
  }

  function closeSkillConfig() {
    setSelectedSkillId(null);
    setDetailState({
      status: "idle",
      skill: null,
      values: {},
      readiness: null,
      message: null,
    });
  }

  function updateSettingValue(key: string, value: string | number | boolean) {
    setDetailState((current) => ({
      ...current,
      values: {
        ...current.values,
        [key]: value,
      },
      message: null,
    }));
  }

  async function toggleSkillEnabled(skill: SkillConfig) {
    setSavingSkillId(skill.id);
    setError(null);
    try {
      await updateSkill(skill.id, {
        ...skill,
        enabled: !skill.enabled,
      });
      await refresh();
      if (selectedSkillId === skill.id) {
        await refreshSelectedSkill(skill.id);
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setSavingSkillId(null);
    }
  }

  async function handleSaveSettings() {
    if (!detailState.skill) {
      return;
    }
    setSavingSettings(true);
    setError(null);
    try {
      const saved = await saveSkillSettings(detailState.skill.id, detailState.values);
      const readiness = await runSkillCheck(detailState.skill.id);
      await refresh();
      await refreshSelectedSkill(detailState.skill.id);
      setDetailState((current) => ({
        ...current,
        status: "ready",
        values: saved.values,
        readiness,
        message: {
          tone: "success",
          text: t("web.skills.settings_saved", "技能配置已保存，并已重新检测。"),
        },
      }));
    } catch (err) {
      const message = String(err);
      setError(message);
      setDetailState((current) => ({
        ...current,
        message: {
          tone: "error",
          text: message,
        },
      }));
    } finally {
      setSavingSettings(false);
    }
  }

  async function handleRunCheck() {
    if (!detailState.skill) {
      return;
    }
    setCheckingSkill(true);
    setError(null);
    try {
      const readiness = await runSkillCheck(detailState.skill.id);
      await refresh();
      await refreshSelectedSkill(detailState.skill.id);
      setDetailState((current) => ({
        ...current,
        readiness,
        message: {
          tone: "success",
          text: t("web.skills.check_ran", "技能检测已刷新。"),
        },
      }));
    } catch (err) {
      const message = String(err);
      setError(message);
      setDetailState((current) => ({
        ...current,
        message: {
          tone: "error",
          text: message,
        },
      }));
    } finally {
      setCheckingSkill(false);
    }
  }

  const enabledCount = skills.filter((item) => item.enabled).length;
  const readyCount = skills.filter((item) => getSkillReadiness(item).status === "ready").length;
  const configurableCount = skills.filter((item) => getSkillSettingFields(item).length > 0 || getSkillDiagnostics(item).check).length;
  const selectedSkillReadiness = getSkillReadiness(detailState.skill);
  const selectedSkillDiagnostics = getSkillDiagnostics(detailState.skill);
  const selectedSkillSettings = getSkillSettingFields(detailState.skill);

  const issueCount = useMemo(
    () => detailState.readiness?.items.filter((item) => item.status !== "ok" && item.status !== "skipped").length ?? 0,
    [detailState.readiness],
  );

  return (
    <div className="skills-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel skills-toolbar">
        <div className="skills-toolbar__row">
          <div className="page-heading">
            <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
            <h1>{t("web.skills.title", "技能是给 Agent 和会话使用的能力包。")}</h1>
            <p>{t("web.skills.description", "这里优先看能力说明、触发方式、配置入口和是否就绪；具体变量与环境检查收进配置面板里。")}</p>
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
            <span>{t("web.skills.summary_ready", "已就绪")}</span>
            <strong>{readyCount}</strong>
            <small>{t("web.skills.readiness.ready", "已就绪")}</small>
          </article>
          <article className="metric-card skills-metric-card">
            <span>{t("web.skills.summary_configurable", "可配置")}</span>
            <strong>{configurableCount}</strong>
            <small>{t("web.skills.configure", "配置")}</small>
          </article>
        </div>
      </section>

      <section className="work-panel skills-catalog-panel">
        <div className="skills-section__header">
          <div className="page-heading">
            <span>{t("web.skills.catalog", "技能目录")}</span>
            <h1>{t("web.skills.catalog_title", "按能力查看技能与就绪情况")}</h1>
            <p>{t("web.skills.catalog_description", "卡片里只保留用途、触发词、挂载模式和轻量就绪摘要；详细配置与环境检测放到单独面板。")}</p>
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
              const readiness = getSkillReadiness(skill);
              const configureLabel = readiness.status === "ready"
                ? t("web.skills.configure", "配置")
                : t("web.skills.configure_needed", "去配置");
              return (
                <article key={skill.id} className="resource-card skills-card">
                  <div className="skills-card__header">
                    <div className="stack skills-card__title">
                      <strong>{skill.id}</strong>
                      <small>{skill.version}</small>
                    </div>
                    <div className="skills-card__actions">
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => void openSkillConfig(skill.id)}
                      >
                        {configureLabel}
                      </button>
                      <button
                        type="button"
                        className="secondary skills-card__toggle"
                        disabled={savingSkillId === skill.id}
                        onClick={() => void toggleSkillEnabled(skill)}
                      >
                        {savingSkillId === skill.id
                          ? t("web.common.saving", "保存中")
                          : skill.enabled
                            ? t("web.common.disabled", "停用")
                            : t("web.common.enabled", "启用")}
                      </button>
                    </div>
                  </div>

                  <div className="skills-card__readiness">
                    <span className={`badge ${readinessBadgeClass(readiness.status)}`}>
                      {localizeReadiness(readiness.status, t)}
                    </span>
                    <p className="skills-card__summary">
                      {readiness.summary || t("web.skills.readiness_unknown_summary", "尚未生成技能检测摘要。")}
                    </p>
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
                  </div>
                </article>
              );
            })}
          </div>
        )}
      </section>

      {selectedSkillId ? (
        <div className="session-manager-modal-root">
          <button
            type="button"
            className="session-manager-modal__backdrop"
            aria-label={t("web.common.close", "关闭")}
            onClick={closeSkillConfig}
          />
          <section
            className="session-manager-modal session-manager-modal--wide skill-config-modal"
            role="dialog"
            aria-modal="true"
            aria-label={t("web.skills.config_dialog", "技能配置")}
          >
            <div className="session-manager-modal__header">
              <div>
                <span>{t("web.skills.configure", "配置")}</span>
                <h2>{detailState.skill?.id ?? selectedSkillId}</h2>
                <p>{detailState.skill?.description || t("web.skills.config_description", "在这里填写技能变量并查看环境检测结果。")}</p>
              </div>
              <button type="button" className="secondary" onClick={closeSkillConfig}>
                {t("web.common.close", "关闭")}
              </button>
            </div>

            {detailState.message ? (
              <div className={detailState.message.tone === "error" ? "error" : "success"}>
                {detailState.message.text}
              </div>
            ) : null}

            {detailState.status === "loading" ? (
              <div className="empty-card skills-empty-state">
                <strong>{t("web.common.loading", "加载中…")}</strong>
                <p>{t("web.skills.config_loading", "正在读取技能配置和检测结果。")}</p>
              </div>
            ) : detailState.status === "error" ? (
              <div className="error">{detailState.message?.text ?? t("web.skills.config_load_error", "技能配置读取失败。")}</div>
            ) : detailState.skill ? (
              <div className="skill-config-modal__content">
                <section className="skill-config-modal__section">
                  <div className="skill-config-modal__section-header">
                    <div className="stack">
                      <div className="panel-title">{t("web.skills.readiness_title", "就绪摘要")}</div>
                      <p className="helper-text">{t("web.skills.readiness_description", "由技能自身定义配置要求和手动检测逻辑，宿主只负责统一呈现。")}</p>
                    </div>
                    <span className={`badge ${readinessBadgeClass(detailState.readiness?.status ?? selectedSkillReadiness.status)}`}>
                      {localizeReadiness(detailState.readiness?.status ?? selectedSkillReadiness.status, t)}
                    </span>
                  </div>
                  <div className="skill-config-modal__summary">
                    <strong>{detailState.readiness?.summary || selectedSkillReadiness.summary || t("web.skills.readiness_unknown_summary", "尚未生成技能检测摘要。")}</strong>
                    <span>
                      {detailState.readiness?.checked_at
                        ? `${t("web.skills.checked_at", "最近检测")} ${formatDateTime(detailState.readiness.checked_at)}`
                        : t("web.skills.not_checked", "还没有检测记录")}
                    </span>
                  </div>
                </section>

                <section className="skill-config-modal__section">
                  <div className="skill-config-modal__section-header">
                    <div className="stack">
                      <div className="panel-title">{t("web.skills.settings", "基础配置")}</div>
                      <p className="helper-text">{t("web.skills.settings_description", "这里显示技能自己声明的变量；保存后会自动触发一次重新检测。")}</p>
                    </div>
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => void handleSaveSettings()}
                      disabled={savingSettings || checkingSkill}
                    >
                      {savingSettings
                        ? t("web.common.saving", "保存中")
                        : t("web.skills.save_settings", "保存并检测")}
                    </button>
                  </div>

                  {selectedSkillSettings.length === 0 ? (
                    <div className="skills-inline-empty">
                      {t("web.skills.settings_empty", "这个技能没有声明可填写的配置项。")}
                    </div>
                  ) : (
                    <div className="form-grid skills-settings-grid">
                      {selectedSkillSettings.map((field) => {
                        const currentValue = detailState.values[field.key] ?? field.default_value;
                        return (
                          <label
                            key={field.key}
                            className={`skills-setting-field ${field.type === "textarea" ? "skills-setting-field--wide" : ""}`}
                          >
                            <span className="skills-setting-field__label">
                              {resolveText(field.label)}
                              {field.required ? <em>{t("web.skills.required", "必填")}</em> : null}
                            </span>
                            {field.description ? (
                              <small className="skills-setting-field__help">{resolveText(field.description)}</small>
                            ) : null}
                            {field.type === "textarea" ? (
                              <textarea
                                rows={4}
                                value={typeof currentValue === "string" ? currentValue : ""}
                                placeholder={field.placeholder ?? ""}
                                onChange={(event) => updateSettingValue(field.key, event.target.value)}
                              />
                            ) : null}
                            {field.type === "text" ? (
                              <input
                                value={typeof currentValue === "string" ? currentValue : ""}
                                placeholder={field.placeholder ?? ""}
                                onChange={(event) => updateSettingValue(field.key, event.target.value)}
                              />
                            ) : null}
                            {field.type === "number" ? (
                              <input
                                type="number"
                                value={typeof currentValue === "number" ? String(currentValue) : "0"}
                                onChange={(event) => updateSettingValue(field.key, Number(event.target.value || 0))}
                              />
                            ) : null}
                            {field.type === "select" ? (
                              <select
                                value={typeof currentValue === "string" ? currentValue : ""}
                                onChange={(event) => updateSettingValue(field.key, event.target.value)}
                              >
                                {field.options.map((option) => (
                                  <option key={option.value} value={option.value}>
                                    {resolveText(option.label)}
                                  </option>
                                ))}
                              </select>
                            ) : null}
                            {field.type === "boolean" ? (
                              <span className="check-row">
                                <input
                                  type="checkbox"
                                  checked={Boolean(currentValue)}
                                  onChange={(event) => updateSettingValue(field.key, event.target.checked)}
                                />
                                <span>{t("web.common.enabled", "启用")}</span>
                              </span>
                            ) : null}
                          </label>
                        );
                      })}
                    </div>
                  )}
                </section>

                <section className="skill-config-modal__section">
                  <div className="skill-config-modal__section-header">
                    <div className="stack">
                      <div className="panel-title">{t("web.skills.diagnostics", "环境检测")}</div>
                      <p className="helper-text">{t("web.skills.diagnostics_description", "检测项由技能自己定义，适合检查依赖、环境变量、可执行文件和连通性。")}</p>
                    </div>
                    {selectedSkillDiagnostics.manual_check || selectedSkillDiagnostics.check ? (
                      <button
                        type="button"
                        className="secondary"
                        onClick={() => void handleRunCheck()}
                        disabled={checkingSkill || savingSettings}
                      >
                        {checkingSkill
                          ? t("web.skills.check_running", "检测中")
                          : t("web.skills.run_check", "重新检测")}
                      </button>
                    ) : null}
                  </div>

                  {detailState.readiness?.items.length ? (
                    <>
                      <div className="skill-config-modal__issue-summary">
                        <span className={`badge ${issueCount > 0 ? "badge--warn" : "badge--success"}`}>
                          {issueCount > 0
                            ? `${issueCount} ${t("web.skills.issues", "项待处理")}`
                            : t("web.skills.issue_free", "无待处理项")}
                        </span>
                      </div>
                      <div className="skill-check-list">
                        {detailState.readiness.items.map((item) => (
                          <article key={item.key} className="mini-card skill-check-card">
                            <div className="skill-check-card__header">
                              <div className="stack">
                                <strong>{item.label}</strong>
                                <small>{localizeCheckCategory(item.category, t)}</small>
                              </div>
                              <span className={`badge ${checkItemBadgeClass(item.status)}`}>
                                {localizeCheckItemStatus(item.status, t)}
                              </span>
                            </div>
                            {item.message ? <p>{item.message}</p> : null}
                            {item.fix_hint ? <small>{item.fix_hint}</small> : null}
                          </article>
                        ))}
                      </div>
                    </>
                  ) : (
                    <div className="empty-card skills-empty-state">
                      <strong>{t("web.skills.diagnostics_empty_title", "当前没有额外检测项")}</strong>
                      <p>{t("web.skills.diagnostics_empty", "这个技能还没有返回逐项检测结果，或所有检查都通过且未输出明细。")}</p>
                    </div>
                  )}
                </section>
              </div>
            ) : null}
          </section>
        </div>
      ) : null}
    </div>
  );
}
