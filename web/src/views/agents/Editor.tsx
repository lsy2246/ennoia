import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type Dispatch,
  type FormEvent,
  type SetStateAction,
} from "react";

import {
  createAgent,
  deleteAgent,
  getAgentPermissionPolicy,
  listAgents,
  listPermissionApprovals,
  listPermissionEvents,
  listProviders,
  listSkills,
  updateAgent,
  updateAgentPermissionPolicy,
  type AgentPermissionPolicy,
  type AgentPermissionRule,
  type AgentProfile,
  type PermissionApprovalRecord,
  type PermissionEventRecord,
  type ProviderConfig,
  type SkillConfig,
} from "@ennoia/api-client";
import type { ExtensionProviderContribution } from "@ennoia/ui-sdk";
import { Select } from "@/components/Select";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";

const EMPTY_AGENT: AgentProfile = {
  id: "",
  display_name: "",
  description: "",
  system_prompt: "",
  provider_id: "",
  model_id: "",
  generation_options: {},
  skills: [],
  enabled: true,
};

const EMPTY_POLICY: AgentPermissionPolicy = {
  mode: "default_deny",
  rules: [],
};

export function AgentEditorView({
  agentId,
}: {
  agentId: string;
}) {
  const { formatDateTime, resolveText, runtime, t } = useUiHelpers();
  const providerContributions = useMemo(
    () => runtime?.registry.providers ?? [],
    [runtime?.registry.providers],
  );
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [form, setForm] = useState<AgentProfile>(EMPTY_AGENT);
  const [policyForm, setPolicyForm] = useState<AgentPermissionPolicy>(EMPTY_POLICY);
  const [permissionApprovals, setPermissionApprovals] = useState<PermissionApprovalRecord[]>([]);
  const [permissionEvents, setPermissionEvents] = useState<PermissionEventRecord[]>([]);
  const [busy, setBusy] = useState(false);
  const [policyBusy, setPolicyBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isNew = agentId.startsWith("new-");

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.id === form.provider_id) ?? providers[0] ?? null,
    [form.provider_id, providers],
  );
  const selectedProviderContribution = useMemo(
    () => findProviderContribution(providerContributions, selectedProvider),
    [providerContributions, selectedProvider],
  );
  const generationOptions = selectedProviderContribution?.provider.generation_options ?? [];

  const hydratePermissions = useCallback(async (targetAgentId: string) => {
    const [policy, approvals, events] = await Promise.all([
      getAgentPermissionPolicy(targetAgentId),
      listPermissionApprovals({ agent_id: targetAgentId, limit: 24 }),
      listPermissionEvents({ agent_id: targetAgentId, limit: 24 }),
    ]);
    setPolicyForm(normalizePolicy(policy));
    setPermissionApprovals(approvals);
    setPermissionEvents(events);
  }, []);

  const hydrate = useCallback(async () => {
    setError(null);
    try {
      const [nextAgents, nextSkills, nextProviders] = await Promise.all([
        listAgents(),
        listSkills(),
        listProviders(),
      ]);
      setAgents(nextAgents);
      setSkills(nextSkills);
      setProviders(nextProviders);

      if (isNew) {
        setForm({
          ...EMPTY_AGENT,
          provider_id: nextProviders[0]?.id ?? "",
          model_id: nextProviders[0]?.default_model ?? "",
          generation_options: defaultGenerationOptions(
            findProviderContribution(providerContributions, nextProviders[0] ?? null),
          ),
        });
        setPolicyForm(EMPTY_POLICY);
        setPermissionApprovals([]);
        setPermissionEvents([]);
        return;
      }

      const current = nextAgents.find((item) => item.id === agentId);
      if (!current) {
        setError("未找到对应 Agent。");
        return;
      }

      setForm({
        ...current,
        skills: [...current.skills],
        generation_options: { ...(current.generation_options ?? {}) },
      });
      await hydratePermissions(current.id);
    } catch (err) {
      setError(String(err));
    }
  }, [agentId, hydratePermissions, isNew, providerContributions]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  function toggleSkill(skillId: string) {
    setForm((current) => ({
      ...current,
      skills: current.skills.includes(skillId)
        ? current.skills.filter((item) => item !== skillId)
        : [...current.skills, skillId],
    }));
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      const payload = normalizeAgentPayload(form, generationOptions);
      if (isNew) {
        await createAgent(payload);
      } else {
        await updateAgent(agentId, payload);
      }
      await hydrate();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete() {
    if (isNew || !form.id) {
      return;
    }
    setBusy(true);
    setError(null);
    try {
      await deleteAgent(form.id);
      setForm(EMPTY_AGENT);
      await hydrate();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handlePolicySave() {
    if (isNew || !form.id) {
      return;
    }
    setPolicyBusy(true);
    setError(null);
    try {
      await updateAgentPermissionPolicy(form.id, normalizePolicy(policyForm));
      await hydratePermissions(form.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setPolicyBusy(false);
    }
  }

  return (
    <form className="resource-editor resource-editor--agent" onSubmit={handleSubmit}>
      <div className="resource-editor__header agent-editor__header">
        <div className="page-heading agent-editor__hero-copy">
          <span className="resource-editor__eyebrow">{t("web.agents.eyebrow", "Agent Registry")}</span>
          <h2>{isNew ? t("web.agents.new", "新建 Agent") : form.display_name || form.id}</h2>
          <p>{t("web.agents.editor_description", "一个 Agent 就是一个可长期维护的协作者档案。")}</p>
        </div>
        <div className="agent-editor__hero-meta">
          <span className={`badge ${form.enabled ? "badge--success" : "badge--muted"}`}>
            {form.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}
          </span>
          {form.provider_id ? <span className="badge badge--muted">{form.provider_id}</span> : null}
          {form.model_id ? <span className="badge badge--muted">{form.model_id}</span> : null}
        </div>
      </div>

      {error ? <div className="error">{error}</div> : null}

      <div className="resource-editor__scroll">
        <div className="agent-editor__canvas">
          <div className="agent-editor__grid">
            <div className="agent-editor__column">
              <section className="details-panel agent-editor__section">
                <div className="panel-title">{t("web.agents.profile", "基本信息")}</div>
                <div className="form-grid agent-editor__form-grid">
                  <label>
                    ID
                    <input value={form.id} onChange={(event) => setForm({ ...form, id: event.target.value })} required />
                  </label>
                  <label>
                    {t("web.agents.display_name", "显示名")}
                    <input
                      value={form.display_name}
                      onChange={(event) => setForm({ ...form, display_name: event.target.value })}
                      required
                    />
                  </label>
                  <label>
                    {t("web.agents.api_channel", "API 上游渠道")}
                    <Select
                      value={form.provider_id}
                      onChange={(value) => {
                        const provider = providers.find((item) => item.id === value);
                        const contribution = findProviderContribution(providerContributions, provider ?? null);
                        setForm({
                          ...form,
                          provider_id: value,
                          model_id: provider?.default_model ?? form.model_id,
                          generation_options: defaultGenerationOptions(contribution),
                        });
                      }}
                      options={providers.map((provider) => ({ value: provider.id, label: provider.display_name }))}
                    />
                  </label>
                  <label>
                    {t("web.agents.model", "模型")}
                    <input
                      list={`agent-models-${agentId}`}
                      value={form.model_id}
                      onChange={(event) => setForm({ ...form, model_id: event.target.value })}
                      required
                    />
                    <datalist id={`agent-models-${agentId}`}>
                      {(selectedProvider?.available_models ?? []).map((model) => (
                        <option key={model.id} value={model.id} />
                      ))}
                    </datalist>
                  </label>
                  <label className="check-row agent-editor__check-row">
                    <input
                      type="checkbox"
                      checked={form.enabled}
                      onChange={(event) => setForm({ ...form, enabled: event.target.checked })}
                    />
                    {t("web.common.enabled", "启用")}
                  </label>
                </div>
              </section>

              {generationOptions.length > 0 ? (
                <section className="details-panel agent-editor__section">
                  <div className="panel-title">{t("web.agents.generation_options", "生成参数")}</div>
                  <p className="helper-text">
                    {t("web.agents.generation_options_help", "这些参数由当前上游扩展声明；未声明的上游不会显示。")}
                  </p>
                  <div className="form-grid agent-editor__form-grid">
                    {generationOptions.map((option: ExtensionProviderContribution["provider"]["generation_options"][number]) => {
                      const value = form.generation_options?.[option.id] ?? option.default_value ?? "";
                      return (
                        <label key={option.id}>
                          {resolveText(option.label)}
                          {option.value_type === "select" && option.allowed_values.length > 0 ? (
                            <Select
                              value={value}
                              onChange={(nextValue) =>
                                setForm({
                                  ...form,
                                  generation_options: {
                                    ...(form.generation_options ?? {}),
                                    [option.id]: nextValue,
                                  },
                                })}
                              options={[
                                ...(!option.required ? [{ value: "", label: t("web.common.none", "无") }] : []),
                                ...option.allowed_values.map((item: string) => ({ value: item, label: item })),
                              ]}
                            />
                          ) : (
                            <input
                              value={value}
                              required={option.required}
                              onChange={(event) =>
                                setForm({
                                  ...form,
                                  generation_options: {
                                    ...(form.generation_options ?? {}),
                                    [option.id]: event.target.value,
                                  },
                                })}
                            />
                          )}
                        </label>
                      );
                    })}
                  </div>
                </section>
              ) : null}

              <section className="details-panel agent-editor__section">
                <label>
                  {t("web.agents.description_field", "描述")}
                  <textarea
                    value={form.description}
                    onChange={(event) => setForm({ ...form, description: event.target.value })}
                    rows={4}
                  />
                </label>
              </section>

              <section className="details-panel agent-editor__section">
                <label>
                  {t("web.agents.system_prompt", "System Prompt")}
                  <textarea
                    className="agent-editor__textarea agent-editor__textarea--code"
                    value={form.system_prompt}
                    onChange={(event) => setForm({ ...form, system_prompt: event.target.value })}
                    rows={12}
                  />
                </label>
              </section>

              <section className="details-panel agent-editor__section">
                <div className="panel-title">{t("web.agents.skills", "技能")}</div>
                <div className="chip-grid">
                  {skills.map((skill) => (
                    <button
                      key={skill.id}
                      type="button"
                      className={form.skills.includes(skill.id) ? "chip chip--active" : "chip"}
                      onClick={() => toggleSkill(skill.id)}
                    >
                      {skill.display_name}
                    </button>
                  ))}
                </div>
              </section>
            </div>

            <div className="agent-editor__column agent-editor__column--side">
              <section className="details-panel agent-editor__section">
                <div className="panel-title">{t("web.agents.working_dir", "工作目录")}</div>
                <div className="kv-list">
                  <span>{t("web.agents.working_dir", "工作目录")}</span>
                  <strong>{formatRelativePath(form.working_dir || "")}</strong>
                  <span>{t("web.agents.skills", "技能")}</span>
                  <strong>{formatRelativePath(form.skills_dir || "")}</strong>
                </div>
                <p className="helper-text">
                  {t("web.agents.working_dir_help", "Agent 工作目录自动派生到 agents/{agent_id}/work，无需单独配置。")}
                </p>
              </section>

              {!isNew && form.id ? (
                <>
                  <section className="details-panel agent-editor__section">
                    <div className="panel-title">{t("web.permissions.policy", "权限策略")}</div>
                    <p className="helper-text">
                      {t("web.permissions.agent_embedded_help", "长期权限配置属于 Agent 本身；即时审批仍应在会话里处理。")}
                    </p>
                    <div className="agent-policy-editor">
                      <label>
                        {t("web.permissions.default_mode", "默认模式")}
                        <Select
                          value={policyForm.mode}
                          onChange={(value) =>
                            setPolicyForm((current) => ({
                              ...current,
                              mode: value,
                            }))}
                          options={[
                            { value: "default_deny", label: t("web.permissions.default_deny", "默认拒绝") },
                            { value: "default_allow", label: t("web.permissions.default_allow", "默认允许") },
                          ]}
                        />
                      </label>

                      <div className="agent-policy-editor__header">
                        <div className="panel-title">{t("web.permissions.rules", "规则")}</div>
                        <button
                          type="button"
                          className="secondary"
                          onClick={() =>
                            setPolicyForm((current) => ({
                              ...current,
                              rules: [...current.rules, createEmptyPermissionRule()],
                            }))}
                        >
                          {t("web.permissions.add_rule", "新增规则")}
                        </button>
                      </div>

                      <div className="agent-policy-editor__rules">
                        {policyForm.rules.length === 0 ? (
                          <div className="empty-card agent-editor__empty-state">
                            <strong>{t("web.agents.empty_rules_title", "当前没有规则")}</strong>
                            <p>{t("web.agents.empty_rules_body", "你可以先新增一条规则，再为动作、路径或主机范围补充匹配条件。")}</p>
                          </div>
                        ) : (
                          policyForm.rules.map((rule, index) => (
                            <article key={rule.id || `rule-${index}`} className="resource-card agent-policy-rule">
                              <div className="agent-policy-rule__header">
                                <strong>{rule.id || t("web.permissions.unnamed_rule", "未命名规则")}</strong>
                                <button
                                  type="button"
                                  className="secondary"
                                  onClick={() =>
                                    setPolicyForm((current) => ({
                                      ...current,
                                      rules: current.rules.filter((_, ruleIndex) => ruleIndex !== index),
                                    }))}
                                >
                                  {t("web.action.delete", "删除")}
                                </button>
                              </div>

                              <div className="form-grid agent-editor__form-grid">
                                <label>
                                  ID
                                  <input
                                    value={rule.id}
                                    onChange={(event) =>
                                      updatePermissionRule(setPolicyForm, index, { id: event.target.value })}
                                  />
                                </label>
                                <label>
                                  {t("web.permissions.effect", "效果")}
                                  <Select
                                    value={rule.effect}
                                    onChange={(value) =>
                                      updatePermissionRule(setPolicyForm, index, { effect: value })}
                                    options={[
                                      { value: "allow", label: t("web.permissions.allow", "允许") },
                                      { value: "ask", label: t("web.permissions.ask", "询问") },
                                      { value: "deny", label: t("web.permissions.deny", "拒绝") },
                                    ]}
                                  />
                                </label>
                                <label>
                                  {t("web.permissions.conversation_scope", "会话范围")}
                                  <Select
                                    value={rule.conversation_scope ?? ""}
                                    onChange={(value) =>
                                      updatePermissionRule(setPolicyForm, index, {
                                        conversation_scope: value || null,
                                      })}
                                    options={[
                                      { value: "", label: t("web.permissions.scope_any", "任意") },
                                      { value: "current", label: t("web.permissions.scope_current", "当前会话") },
                                    ]}
                                  />
                                </label>
                                <label>
                                  {t("web.permissions.run_scope", "运行范围")}
                                  <Select
                                    value={rule.run_scope ?? ""}
                                    onChange={(value) =>
                                      updatePermissionRule(setPolicyForm, index, {
                                        run_scope: value || null,
                                      })}
                                    options={[
                                      { value: "", label: t("web.permissions.scope_any", "任意") },
                                      { value: "current", label: t("web.permissions.scope_current_run", "当前运行") },
                                    ]}
                                  />
                                </label>
                              </div>

                              <div className="agent-policy-rule__grid">
                                <PermissionRuleListEditor
                                  t={t}
                                  label={t("web.permissions.actions", "动作")}
                                  placeholder={t("web.permissions.actions_placeholder", "例如 provider.generate")}
                                  values={rule.actions}
                                  onChange={(values) =>
                                    updatePermissionRule(setPolicyForm, index, { actions: values })}
                                />
                                <PermissionRuleListEditor
                                  t={t}
                                  label={t("web.permissions.extension_scope", "扩展范围")}
                                  placeholder={t("web.permissions.extension_scope_placeholder", "例如 openai")}
                                  values={rule.extension_scope}
                                  onChange={(values) =>
                                    updatePermissionRule(setPolicyForm, index, { extension_scope: values })}
                                />
                                <PermissionRuleListEditor
                                  t={t}
                                  label={t("web.permissions.path_include", "允许路径")}
                                  placeholder={t("web.permissions.path_include_placeholder", "例如 ~/.ennoia/agents/**")}
                                  values={rule.path_include}
                                  onChange={(values) =>
                                    updatePermissionRule(setPolicyForm, index, { path_include: values })}
                                />
                                <PermissionRuleListEditor
                                  t={t}
                                  label={t("web.permissions.path_exclude", "排除路径")}
                                  placeholder={t("web.permissions.path_exclude_placeholder", "例如 ~/.ennoia/tmp/**")}
                                  values={rule.path_exclude}
                                  onChange={(values) =>
                                    updatePermissionRule(setPolicyForm, index, { path_exclude: values })}
                                />
                                <div className="agent-policy-rule__wide">
                                  <PermissionRuleListEditor
                                    t={t}
                                    label={t("web.permissions.host_scope", "主机范围")}
                                    placeholder={t("web.permissions.host_scope_placeholder", "例如 api.openai.com")}
                                    values={rule.host_scope}
                                    onChange={(values) =>
                                      updatePermissionRule(setPolicyForm, index, { host_scope: values })}
                                  />
                                </div>
                              </div>
                            </article>
                          ))
                        )}
                      </div>
                    </div>
                    <div className="button-row button-row--wrap">
                      <button type="button" onClick={() => void handlePolicySave()} disabled={policyBusy}>
                        {policyBusy ? t("web.common.saving", "保存中") : t("web.action.save", "保存")}
                      </button>
                      <button type="button" className="secondary" onClick={() => void hydratePermissions(form.id)}>
                        {t("web.action.refresh", "刷新")}
                      </button>
                    </div>
                  </section>

                  <section className="details-panel agent-editor__section">
                    <div className="panel-title">{t("web.permissions.approvals", "最近审批")}</div>
                    <div className="agent-editor__card-list">
                      {permissionApprovals.length === 0 ? (
                        <div className="empty-card agent-editor__empty-state">
                          <strong>{t("web.agents.empty_approvals_title", "当前没有审批记录")}</strong>
                          <p>{t("web.agents.empty_approvals_body", "当 Agent 触发需要确认的动作后，这里会显示最近的审批结果。")}</p>
                        </div>
                      ) : (
                        permissionApprovals.slice(0, 8).map((approval) => (
                          <article key={approval.approval_id} className="mini-card agent-editor__mini-card">
                            <strong>{approval.action}</strong>
                            <span>{approval.reason}</span>
                            <span className={`badge ${approvalStatusClass(approval.status)}`}>{approval.status}</span>
                            <span>{formatDateTime(approval.created_at)}</span>
                            <span>{approval.scope.conversation_id ?? t("web.common.none", "无")}</span>
                          </article>
                        ))
                      )}
                    </div>
                  </section>

                  <section className="details-panel agent-editor__section">
                    <div className="panel-title">{t("web.permissions.events", "最近权限事件")}</div>
                    <div className="agent-editor__card-list">
                      {permissionEvents.length === 0 ? (
                        <div className="empty-card agent-editor__empty-state">
                          <strong>{t("web.agents.empty_events_title", "当前没有权限事件")}</strong>
                          <p>{t("web.agents.empty_events_body", "Agent 产生 allow、ask 或 deny 判断后，这里会保留最近的权限事件。")}</p>
                        </div>
                      ) : (
                        permissionEvents.slice(0, 8).map((event) => (
                          <article key={event.event_id} className="mini-card agent-editor__mini-card">
                            <strong>{event.action}</strong>
                            <span className={`badge ${permissionDecisionClass(event.decision)}`}>{event.decision}</span>
                            <span>{event.target.kind}:{event.target.id}</span>
                            <span>{formatDateTime(event.created_at)}</span>
                          </article>
                        ))
                      )}
                    </div>
                  </section>
                </>
              ) : null}
            </div>
          </div>
        </div>
      </div>

      <div className="resource-editor__footer">
        <button type="submit" disabled={busy}>
          {t("web.action.save", "保存")}
        </button>
        <button
          type="button"
          className="danger"
          disabled={busy || isNew || !agents.some((item) => item.id === form.id)}
          onClick={() => void handleDelete()}
        >
          {t("web.action.delete", "删除")}
        </button>
      </div>
    </form>
  );
}

function approvalStatusClass(status: string) {
  if (status === "approved") {
    return "badge--success";
  }
  if (status === "pending") {
    return "badge--warn";
  }
  if (status === "expired") {
    return "badge--muted";
  }
  return "badge--danger";
}

function permissionDecisionClass(decision: string) {
  if (decision === "allow") {
    return "badge--success";
  }
  if (decision === "ask") {
    return "badge--warn";
  }
  return "badge--danger";
}

function findProviderContribution(
  contributions: ExtensionProviderContribution[],
  provider: ProviderConfig | null,
) {
  if (!provider) {
    return null;
  }

  const matches = contributions.filter((item) => item.provider.kind === provider.kind);
  return matches.length === 1 ? matches[0] : null;
}

function defaultGenerationOptions(contribution: ExtensionProviderContribution | null) {
  return Object.fromEntries(
    (contribution?.provider.generation_options ?? [])
      .filter((option: ExtensionProviderContribution["provider"]["generation_options"][number]) => option.default_value)
      .map((option: ExtensionProviderContribution["provider"]["generation_options"][number]) => [option.id, option.default_value!]),
  );
}

function normalizeAgentPayload(
  form: AgentProfile,
  options: ExtensionProviderContribution["provider"]["generation_options"],
) {
  const generation_options = Object.fromEntries(
    options.flatMap((option: ExtensionProviderContribution["provider"]["generation_options"][number]) => {
      const value = form.generation_options?.[option.id] ?? option.default_value ?? "";
      if (!value.trim()) {
        return [];
      }
      return [[option.id, value]];
    }),
  );

  return {
    ...form,
    generation_options,
  };
}

function createEmptyPermissionRule(): AgentPermissionRule {
  return {
    id: `rule-${Math.random().toString(36).slice(2, 10)}`,
    effect: "ask",
    actions: [],
    extension_scope: [],
    conversation_scope: null,
    run_scope: null,
    path_include: [],
    path_exclude: [],
    host_scope: [],
  };
}

function normalizePolicy(policy: AgentPermissionPolicy): AgentPermissionPolicy {
  return {
    mode: policy.mode || "default_deny",
    rules: (policy.rules ?? []).map((rule) => ({
      id: rule.id ?? "",
      effect: rule.effect || "ask",
      actions: [...(rule.actions ?? [])],
      extension_scope: [...(rule.extension_scope ?? [])],
      conversation_scope: rule.conversation_scope ?? null,
      run_scope: rule.run_scope ?? null,
      path_include: [...(rule.path_include ?? [])],
      path_exclude: [...(rule.path_exclude ?? [])],
      host_scope: [...(rule.host_scope ?? [])],
    })),
  };
}

function updatePermissionRule(
  setPolicyForm: Dispatch<SetStateAction<AgentPermissionPolicy>>,
  index: number,
  patch: Partial<AgentPermissionRule>,
) {
  setPolicyForm((current) => ({
    ...current,
    rules: current.rules.map((rule, ruleIndex) =>
      ruleIndex === index ? { ...rule, ...patch } : rule,
    ),
  }));
}

function PermissionRuleListEditor({
  t,
  label,
  placeholder,
  values,
  onChange,
}: {
  t: (key: string, fallback: string, params?: Record<string, string | number>) => string;
  label: string;
  placeholder: string;
  values: string[];
  onChange: (values: string[]) => void;
}) {
  function handleItemChange(itemIndex: number, nextValue: string) {
    onChange(
      values.map((value, index) => (index === itemIndex ? nextValue : value)),
    );
  }

  function handleItemRemove(itemIndex: number) {
    onChange(values.filter((_, index) => index !== itemIndex));
  }

  function handleItemAdd() {
    onChange([...values, ""]);
  }

  return (
    <div className="stack">
      <label>{label}</label>
      <div className="model-list">
        {values.length > 0 ? (
          values.map((value, itemIndex) => (
            <div key={`${label}-${itemIndex}`} className="model-row">
              <input
                value={value}
                placeholder={placeholder}
                onChange={(event) => handleItemChange(itemIndex, event.target.value)}
              />
              <button
                type="button"
                className="secondary"
                onClick={() => handleItemRemove(itemIndex)}
              >
                删除
              </button>
            </div>
          ))
        ) : (
          <div className="empty-card agent-editor__empty-state agent-policy-list-empty">
            <strong>{t("web.agents.empty_items_title", "当前没有条目")}</strong>
            <p>{t("web.agents.empty_items_body", "点击下方“新增条目”后，再补充这个范围的匹配规则。")}</p>
          </div>
        )}
        <button type="button" className="secondary" onClick={handleItemAdd}>
          新增条目
        </button>
      </div>
    </div>
  );
}
