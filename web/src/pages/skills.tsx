import { useEffect, useState } from "react";

import {
  getSkill,
  getSkillSettings,
  getSkillStatus,
  listSkills,
  runSkillCheck,
  runSkillPrepare,
  saveSkillSettings,
  updateSkill,
  type SkillCheckAction,
  type SkillCheckItem,
  type SkillCheckResult,
  type SkillConfig,
  type SkillRuntimeStatus,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useUiHelpers } from "@/stores/ui";

type SkillDetailState = {
  status: "idle" | "loading" | "ready" | "error";
  skill: SkillConfig | null;
  check: SkillCheckResult | null;
  values: Record<string, string | number | boolean>;
  runningAction: string | null;
  message: { tone: "success" | "error"; text: string } | null;
};

function getSkillSettingFields(skill: SkillConfig | null | undefined) {
  return skill?.settings ?? [];
}

function skillStatusLabel(status: SkillRuntimeStatus) {
  switch (status) {
    case "ready":
      return "已就绪";
    case "partial":
      return "部分就绪";
    case "missing_config":
      return "缺少配置";
    case "env_missing":
      return "环境缺失";
    case "error":
      return "异常";
    case "unknown":
      return "未知";
    default:
      return status;
  }
}

function skillStatusBadgeClass(status: SkillRuntimeStatus) {
  switch (status) {
    case "ready":
      return "badge--success";
    case "partial":
    case "missing_config":
    case "env_missing":
      return "badge--warn";
    case "error":
      return "badge--danger";
    default:
      return "badge--muted";
  }
}

function skillItemStatusLabel(status: SkillCheckItem["status"]) {
  switch (status) {
    case "ok":
      return "通过";
    case "missing":
      return "缺失";
    case "warning":
      return "警告";
    case "error":
      return "异常";
    case "skipped":
      return "跳过";
    default:
      return status;
  }
}

function skillItemStatusBadgeClass(status: SkillCheckItem["status"]) {
  switch (status) {
    case "ok":
      return "badge--success";
    case "missing":
    case "warning":
      return "badge--warn";
    case "error":
      return "badge--danger";
    default:
      return "badge--muted";
  }
}

function skillCategoryLabel(category: SkillCheckItem["category"]) {
  switch (category) {
    case "config":
      return "配置";
    case "environment":
      return "环境";
    case "permission":
      return "权限";
    case "dependency":
      return "依赖";
    case "connectivity":
      return "连接";
    default:
      return category;
  }
}

function formatCheckedAt(value: string | null | undefined) {
  if (!value) {
    return "尚未检测";
  }
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function Skills() {
  const { resolveText, t } = useUiHelpers();
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [savingSkillId, setSavingSkillId] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [selectedSkillId, setSelectedSkillId] = useState<string | null>(null);
  const [detailState, setDetailState] = useState<SkillDetailState>({
    status: "idle",
    skill: null,
    check: null,
    values: {},
    runningAction: null,
    message: null,
  });
  const [savingSettings, setSavingSettings] = useState(false);

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
    const [nextSkill, nextCheck] = await Promise.all([
      getSkill(skillId),
      getSkillStatus(skillId),
    ]);
    setDetailState((current) => ({
      ...current,
      skill: nextSkill,
      check: nextCheck,
    }));
  }

  async function openSkillConfig(skillId: string) {
    setSelectedSkillId(skillId);
    setDetailState({
      status: "loading",
      skill: null,
      check: null,
      values: {},
      runningAction: null,
      message: null,
    });
    try {
      const [skill, settings, check] = await Promise.all([
        getSkill(skillId),
        getSkillSettings(skillId),
        getSkillStatus(skillId),
      ]);
      setDetailState({
        status: "ready",
        skill,
        check,
        values: settings.values,
        runningAction: null,
        message: null,
      });
    } catch (err) {
      setDetailState({
        status: "error",
        skill: null,
        check: null,
        values: {},
        runningAction: null,
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
      check: null,
      values: {},
      runningAction: null,
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
      await refresh();
      await refreshSelectedSkill(detailState.skill.id);
      setDetailState((current) => ({
        ...current,
        status: "ready",
        values: saved.values,
        message: {
          tone: "success",
          text: t("web.skills.settings_saved", "技能配置已保存。"),
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

  async function runSkillDiagnosticAction(action: SkillCheckAction) {
    if (!detailState.skill) {
      return;
    }
    const skillId = detailState.skill.id;
    const runAction = action.kind === "recheck"
      ? runSkillCheck
      : action.kind === "prepare"
        ? runSkillPrepare
        : null;
    if (!runAction) {
      setDetailState((current) => ({
        ...current,
        message: {
          tone: "error",
          text: `暂不支持的技能动作：${action.kind}`,
        },
      }));
      return;
    }

    setDetailState((current) => ({
      ...current,
      runningAction: action.key,
      message: null,
    }));
    setError(null);
    try {
      const result = await runAction(skillId);
      const [nextSkills, nextSkill] = await Promise.all([
        listSkills(),
        getSkill(skillId),
      ]);
      setSkills(nextSkills);
      setDetailState((current) => ({
        ...current,
        status: "ready",
        skill: nextSkill,
        check: result,
        runningAction: null,
      }));
    } catch (err) {
      const message = String(err);
      setError(message);
      setDetailState((current) => ({
        ...current,
        runningAction: null,
        message: {
          tone: "error",
          text: message,
        },
      }));
    }
  }

  const enabledCount = skills.filter((item) => item.enabled).length;
  const configurableCount = skills.filter((item) => getSkillSettingFields(item).length > 0).length;
  const totalActions = skills.reduce((sum, item) => sum + item.actions.length, 0);
  const selectedSkillSettings = getSkillSettingFields(detailState.skill);
  const selectedSkillCheck = detailState.check ?? (detailState.skill
    ? {
        status: detailState.skill.readiness.status,
        summary: detailState.skill.readiness.summary,
        checked_at: detailState.skill.readiness.checked_at,
        items: [],
        actions: [],
      }
    : null);

  return (
    <div className="skills-page">
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <section className="work-panel skills-toolbar">
        <div className="skills-toolbar__row">
          <div className="page-heading">
            <span>{t("web.skills.eyebrow", "Skill Registry")}</span>
            <h1>{t("web.skills.title", "技能是给 Agent 和会话使用的能力包。")}</h1>
            <p>{t("web.skills.description", "这里优先看能力说明、触发方式、动作入口和配置入口。")}</p>
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
            <span>{t("web.skills.summary_actions", "动作总数")}</span>
            <strong>{totalActions}</strong>
            <small>{t("web.skills.entry", "动作")}</small>
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
            <h1>{t("web.skills.catalog_title", "按能力查看技能与动作入口")}</h1>
            <p>{t("web.skills.catalog_description", "卡片里只保留用途、触发词、挂载模式和动作信息；配置放到单独面板。")}</p>
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
              const configureLabel = getSkillSettingFields(skill).length > 0
                ? t("web.skills.configure", "配置")
                : t("web.skills.view_detail", "查看");
              return (
                <article key={skill.id} className="resource-card skills-card">
                  <div className="skills-card__header">
                    <div className="stack skills-card__title">
                      <strong>{skill.id}</strong>
                      <span className="skills-card__status-row">
                        <small>{skill.version}</small>
                        <span className={`badge ${skillStatusBadgeClass(skill.readiness.status)}`}>
                          {skillStatusLabel(skill.readiness.status)}
                        </span>
                      </span>
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

                  <p className="skills-card__description">{skill.description || t("web.common.none", "无")}</p>

                  <div className="skills-meta-grid skills-meta-grid--compact">
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
                <p>{detailState.skill?.description || t("web.skills.config_description", "在这里查看技能动作入口并填写技能变量。")}</p>
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
                <p>{t("web.skills.config_loading", "正在读取技能说明和配置。")}</p>
              </div>
            ) : detailState.status === "error" ? (
              <div className="error">{detailState.message?.text ?? t("web.skills.config_load_error", "技能配置读取失败。")}</div>
            ) : detailState.skill ? (
              <div className="skill-config-modal__content">
                {selectedSkillCheck ? (
                  <section className="skill-config-modal__section">
                    <div className="skill-config-modal__section-header">
                      <div className="stack">
                        <div className="panel-title">{t("web.skills.runtime_status", "运行状态")}</div>
                        <p className="helper-text">{t("web.skills.runtime_status_description", "这里展示技能自己返回的诊断结果；宿主只负责执行和展示。")}</p>
                      </div>
                      <span className={`badge ${skillStatusBadgeClass(selectedSkillCheck.status)}`}>
                        {skillStatusLabel(selectedSkillCheck.status)}
                      </span>
                    </div>

                    <div className="skill-config-modal__summary">
                      <strong>{selectedSkillCheck.summary}</strong>
                      <small>
                        {t("web.skills.checked_at", "检测时间")}：{formatCheckedAt(selectedSkillCheck.checked_at)}
                      </small>
                    </div>

                    {selectedSkillCheck.actions.length > 0 ? (
                      <div className="skill-check-actions">
                        {selectedSkillCheck.actions.map((action) => (
                          <button
                            key={action.key}
                            type="button"
                            className="secondary"
                            disabled={detailState.runningAction !== null}
                            onClick={() => void runSkillDiagnosticAction(action)}
                          >
                            {detailState.runningAction === action.key
                              ? t("web.common.running", "执行中")
                              : action.label}
                          </button>
                        ))}
                      </div>
                    ) : null}

                    {selectedSkillCheck.items.length === 0 ? (
                      <div className="skills-inline-empty">
                        {t("web.skills.check_items_empty", "没有需要展示的检查项。")}
                      </div>
                    ) : (
                      <div className="skill-check-list">
                        {selectedSkillCheck.items.map((item) => (
                          <article
                            key={item.key}
                            className={`skill-check-card skill-check-card--${item.status}`}
                          >
                            <div className="skill-check-card__header">
                              <div className="stack">
                                <strong>{item.label}</strong>
                                <small>
                                  {skillCategoryLabel(item.category)}
                                  {item.required ? ` / ${t("web.skills.required", "必填")}` : ""}
                                </small>
                              </div>
                              <span className={`badge ${skillItemStatusBadgeClass(item.status)}`}>
                                {skillItemStatusLabel(item.status)}
                              </span>
                            </div>
                            {item.message ? <p>{item.message}</p> : null}
                            {item.fix_hint ? <small>{item.fix_hint}</small> : null}
                          </article>
                        ))}
                      </div>
                    )}
                  </section>
                ) : null}

                <section className="skill-config-modal__section">
                  <div className="skill-config-modal__section-header">
                    <div className="stack">
                      <div className="panel-title">{t("web.skills.entry", "动作")}</div>
                      <p className="helper-text">{t("web.skills.actions_description", "这里列出 skill 对外暴露的动作入口，宿主不再额外管理安装或检测逻辑。")}</p>
                    </div>
                  </div>
                  {detailState.skill.actions.length === 0 ? (
                    <div className="skills-inline-empty">
                      {t("web.skills.keywords_empty", "这个技能还没有声明可执行动作。")}
                    </div>
                  ) : (
                    <div className="skills-meta-grid">
                      {detailState.skill.actions.map((action) => (
                        <div key={action.id} className="skills-meta-item">
                          <span>{action.id}</span>
                          <strong>{action.entry}</strong>
                        </div>
                      ))}
                    </div>
                  )}
                </section>

                <section className="skill-config-modal__section">
                  <div className="skill-config-modal__section-header">
                    <div className="stack">
                      <div className="panel-title">{t("web.skills.settings", "基础配置")}</div>
                      <p className="helper-text">{t("web.skills.settings_description", "这里显示技能自己声明的变量；宿主只负责保存和回填。")}</p>
                    </div>
                    <button
                      type="button"
                      className="secondary"
                      onClick={() => void handleSaveSettings()}
                      disabled={savingSettings}
                    >
                      {savingSettings
                        ? t("web.common.saving", "保存中")
                        : t("web.skills.save_settings", "保存配置")}
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
              </div>
            ) : null}
          </section>
        </div>
      ) : null}
    </div>
  );
}
