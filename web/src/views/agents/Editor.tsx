import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type FormEvent,
} from "react";

import {
  createAgent,
  createPermissionEventsStream,
  deleteAgent,
  listAgents,
  listModelEndpoints,
  listPermissionApprovals,
  listPermissionEvents,
  listSkills,
  updateAgent,
  type AgentFileAccessProfile,
  type AgentPermissionCommandEntry,
  type AgentPermissionProfile,
  type AgentProfile,
  type ModelEndpointConfig,
  type PermissionApprovalRecord,
  type PermissionEventRecord,
  type SkillConfig,
} from "@ennoia/api-client";
import type { ExtensionProviderContribution } from "@ennoia/ui-sdk";
import { Select } from "@/components/Select";
import { StatusNotice } from "@/components/StatusNotice";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useAgentsStore } from "@/stores/agents";
import { useModelEndpointsStore } from "@/stores/modelEndpoints";
import { useUiHelpers } from "@/stores/ui";

const DEFAULT_FILE_ACCESS_PROFILE: AgentFileAccessProfile = {
  default_root: "/workspace",
  roots: [
    { id: "workspace", path: "/workspace", mode: "read_write" },
    { id: "artifacts", path: "/artifacts", mode: "read_write" },
    { id: "temp", path: "/tmp", mode: "read_write" },
  ],
};

const EMPTY_AGENT: AgentProfile = {
  id: "",
  display_name: "",
  description: "",
  system_prompt: "",
  model_endpoint_id: "",
  model_id: "",
  generation_options: {},
  skills: [],
  enabled: true,
  permission_profile: {
    mode: "whitelist",
    entries: [],
  },
  file_access: DEFAULT_FILE_ACCESS_PROFILE,
};

const EMPTY_PERMISSION_PROFILE: AgentPermissionProfile = {
  mode: "whitelist",
  entries: [],
};

function buildCommandPermissionMatchOptions(
  t: (key: string, fallback: string, params?: Record<string, string | number>) => string,
) {
  return [
    { value: "prefix", label: t("web.permissions.match_prefix", "前缀匹配") },
    { value: "exact", label: t("web.permissions.match_exact", "完整匹配") },
    { value: "regex", label: t("web.permissions.match_regex", "正则匹配") },
  ];
}

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
  const notifyAgentsChanged = useAgentsStore((state) => state.notifyChanged);
  const modelEndpointsRevision = useModelEndpointsStore((state) => state.revision);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [modelEndpoints, setModelEndpoints] = useState<ModelEndpointConfig[]>([]);
  const [form, setForm] = useState<AgentProfile>(EMPTY_AGENT);
  const [policyForm, setPolicyForm] = useState<AgentPermissionProfile>(EMPTY_PERMISSION_PROFILE);
  const [permissionApprovals, setPermissionApprovals] = useState<PermissionApprovalRecord[]>([]);
  const [permissionEvents, setPermissionEvents] = useState<PermissionEventRecord[]>([]);
  const [busy, setBusy] = useState(false);
  const [policyBusy, setPolicyBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isNew = agentId.startsWith("new-");

  const selectedProvider = useMemo(
    () => modelEndpoints.find((item) => item.id === form.model_endpoint_id) ?? modelEndpoints[0] ?? null,
    [form.model_endpoint_id, modelEndpoints],
  );
  const selectedProviderContribution = useMemo(
    () => findProviderContribution(providerContributions, selectedProvider),
    [providerContributions, selectedProvider],
  );
  const generationOptions = selectedProviderContribution?.provider.generation_options ?? [];
  const modelOptions = useMemo(
    () => buildAgentModelOptions(selectedProvider, form.model_id, t),
    [form.model_id, selectedProvider, t],
  );

  const refreshModelEndpoints = useCallback(async () => {
    const nextModelEndpoints = await listModelEndpoints();
    setModelEndpoints(nextModelEndpoints);
  }, []);

  const hydratePermissions = useCallback(async (targetAgentId: string) => {
    const [approvals, events] = await Promise.all([
      listPermissionApprovals({ agent_id: targetAgentId, limit: 24 }),
      listPermissionEvents({ agent_id: targetAgentId, limit: 24 }),
    ]);
    setPermissionApprovals(approvals);
    setPermissionEvents(events);
  }, []);

  const hydrate = useCallback(async () => {
    setError(null);
    try {
      const [nextAgents, nextSkills, nextModelEndpoints] = await Promise.all([
        listAgents(),
        listSkills(),
        listModelEndpoints(),
      ]);
      setAgents(nextAgents);
      setSkills(nextSkills);
      setModelEndpoints(nextModelEndpoints);

      if (isNew) {
        setForm({
          ...EMPTY_AGENT,
          model_endpoint_id: nextModelEndpoints[0]?.id ?? "",
          model_id: resolveAgentModelId(nextModelEndpoints[0] ?? null, ""),
          generation_options: defaultGenerationOptions(
            findProviderContribution(providerContributions, nextModelEndpoints[0] ?? null),
          ),
        });
        setPolicyForm(EMPTY_PERMISSION_PROFILE);
        setPermissionApprovals([]);
        setPermissionEvents([]);
        return;
      }

      const current = nextAgents.find((item) => item.id === agentId);
      if (!current) {
        setError(t("web.agents.not_found", "未找到对应 Agent。"));
        return;
      }

      setForm(normalizeAgentForm(current));
      setPolicyForm(normalizePermissionProfile(current.permission_profile));
      await hydratePermissions(current.id);
    } catch (err) {
      setError(String(err));
    }
  }, [agentId, hydratePermissions, isNew, providerContributions, t]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  useEffect(() => {
    if (modelEndpointsRevision === 0) {
      return;
    }
    void refreshModelEndpoints();
  }, [modelEndpointsRevision, refreshModelEndpoints]);

  useEffect(() => {
    if (isNew || !form.id || typeof EventSource === "undefined") {
      return;
    }
    const stream = createPermissionEventsStream(form.id);
    const handleChanged = () => {
      void hydratePermissions(form.id);
    };
    stream.addEventListener("permissions.changed", handleChanged);
    stream.onerror = () => undefined;
    return () => {
      stream.removeEventListener("permissions.changed", handleChanged);
      stream.close();
    };
  }, [form.id, hydratePermissions, isNew]);

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
      const payload = normalizeAgentPayload(
        {
          ...form,
          permission_profile: normalizePermissionProfile(policyForm),
        },
        generationOptions,
      );
      if (isNew) {
        await createAgent(payload);
      } else {
        await updateAgent(agentId, payload);
      }
      notifyAgentsChanged();
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
      notifyAgentsChanged();
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
      await updateAgent(
        form.id,
        normalizeAgentPayload(
          {
            ...form,
            permission_profile: normalizePermissionProfile(policyForm),
          },
          generationOptions,
        ),
      );
      setForm((current) => ({
        ...current,
        permission_profile: normalizePermissionProfile(policyForm),
      }));
      await hydratePermissions(form.id);
    } catch (err) {
      setError(String(err));
    } finally {
      setPolicyBusy(false);
    }
  }

  return (
    <form className="resource-editor resource-editor--agent" onSubmit={handleSubmit}>
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
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
          {form.model_endpoint_id ? <span className="badge badge--muted">{form.model_endpoint_id}</span> : null}
          {form.model_id ? <span className="badge badge--muted">{form.model_id}</span> : null}
        </div>
      </div>

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
                    {t("web.agents.api_channel", "模型接入")}
                    <Select
                      value={form.model_endpoint_id}
                      onChange={(value) => {
                        const provider = modelEndpoints.find((item) => item.id === value);
                        const contribution = findProviderContribution(providerContributions, provider ?? null);
                        setForm({
                          ...form,
                          model_endpoint_id: value,
                          model_id: resolveAgentModelId(provider ?? null, form.model_id),
                          generation_options: defaultGenerationOptions(contribution),
                        });
                      }}
                      options={modelEndpoints.map((provider) => ({ value: provider.id, label: provider.display_name }))}
                    />
                  </label>
                  <label>
                    {t("web.agents.model", "模型")}
                    {modelOptions.length > 0 ? (
                      <Select
                        value={form.model_id}
                        onChange={(value) => setForm({ ...form, model_id: value })}
                        options={modelOptions}
                        placeholder={t("web.agents.model_select_placeholder", "请选择已配置模型")}
                      />
                    ) : (
                      <input
                        value={form.model_id}
                        onChange={(event) => setForm({ ...form, model_id: event.target.value })}
                        placeholder={t("web.agents.model_input_placeholder", "当前没有已配置模型，可临时手动输入")}
                        required
                      />
                    )}
                    <p className="helper-text">
                      {modelOptions.length > 0
                        ? t("web.agents.model_help", "这里只显示当前模型接入里已经配置并保存过的模型。")
                        : t("web.agents.model_empty_help", "当前模型接入还没有已配置模型；先去“模型接入”里添加并保存，或临时手动输入。")}
                    </p>
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
                      title={`/${skill.id} · ${skill.description || skill.mount.mode}`}
                    >
                      {skill.id}
                    </button>
                  ))}
                </div>
                <p className="helper-text">
                  {t("web.agents.skills_mount_help", "技能是否自动随 Agent 带上由各自的 mount.mode 决定；这里仅管理分配关系。")}
                </p>
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

              <>
                <section className="details-panel agent-editor__section">
                  <div className="panel-title">{t("web.agents.file_access", "文件访问")}</div>
                  <p className="helper-text">
                    {t("web.agents.file_access_help", "command.exec 的工作目录只接受这些虚拟根及其子路径。")}
                  </p>
                  <div className="agent-policy-editor">
                    <div className="resource-card agent-policy-rule">
                      <div className="agent-policy-rule__header">
                        <strong>{t("web.agents.file_access_current_entry", "当前入口")}</strong>
                      </div>
                      <div className="kv-list">
                        <span>{t("web.agents.file_access_default_root", "默认根")}</span>
                        <strong>{normalizeFileAccessProfile(form.file_access).default_root}</strong>
                        {normalizeFileAccessProfile(form.file_access).roots.flatMap((root) => [
                          <span key={`${root.id}-label`}>{root.id}</span>,
                          <strong key={`${root.id}-path`}>{root.path}</strong>,
                        ])}
                      </div>
                    </div>
                  </div>
                </section>

                <section className="details-panel agent-editor__section">
                  <div className="panel-title">{t("web.permissions.profile", "权限模式")}</div>
                  <p className="helper-text">
                    {t("web.permissions.profile_help", "长期权限配置属于 Agent 本身；这里只定义 command.exec 的命令匹配策略，文件访问由文件访问配置单独决定。")}
                  </p>
                  <div className="agent-policy-editor">
                    <label>
                      {t("web.permissions.command_default_policy", "命令默认策略")}
                      <Select
                        value={policyForm.mode}
                        onChange={(value) =>
                          setPolicyForm((current) => ({
                            ...current,
                            mode: value,
                          }))}
                        options={[
                          { value: "whitelist", label: t("web.permissions.mode_whitelist", "白名单模式：默认询问，命中规则直接允许") },
                          { value: "blacklist", label: t("web.permissions.mode_blacklist", "黑名单模式：默认允许，命中规则改为询问") },
                        ]}
                      />
                    </label>

                    <div className="agent-policy-editor__rules">
                      <div className="resource-card agent-policy-rule">
                        <div className="agent-policy-rule__header">
                          <strong>{t("web.permissions.command_entries", "命令匹配条目")}</strong>
                        </div>
                        <p className="helper-text">
                          {policyForm.mode === "blacklist"
                            ? t("web.permissions.entries_blacklist_help", "这里填写需要改成询问的命令调用。未命中的命令默认直接运行。")
                            : t("web.permissions.entries_whitelist_help", "这里填写可以直接运行的命令调用。未命中的命令默认进入询问。")}
                        </p>
                        <CommandPermissionEntriesEditor
                          t={t}
                          entries={policyForm.entries}
                          onChange={(entries) =>
                            setPolicyForm((current) => ({
                              ...current,
                              entries,
                            }))}
                        />
                      </div>

                      <div className="resource-card agent-policy-rule">
                        <div className="agent-policy-rule__header">
                          <strong>{t("web.permissions.current_behavior", "当前行为")}</strong>
                        </div>
                        <p className="helper-text">
                          {policyForm.mode === "blacklist"
                            ? t("web.permissions.behavior_blacklist", "command.exec 默认直接运行；只有命中这些条目的命令调用才会先询问。")
                            : t("web.permissions.behavior_whitelist", "command.exec 默认先询问；只有命中这些条目的命令调用才会直接运行。")}
                        </p>
                      </div>
                    </div>
                  </div>
                  {isNew ? (
                    <p className="helper-text">
                      {t("web.permissions.new_agent_save_hint", "新建 Agent 时，这里的配置会和基础信息一起在点击底部“保存”后创建。")}
                    </p>
                  ) : (
                    <div className="button-row button-row--wrap">
                      <button type="button" onClick={() => void handlePolicySave()} disabled={policyBusy}>
                        {policyBusy ? t("web.common.saving", "保存中") : t("web.action.save", "保存")}
                      </button>
                      <button type="button" className="secondary" onClick={() => void hydratePermissions(form.id)}>
                        {t("web.action.refresh", "刷新")}
                      </button>
                    </div>
                  )}
                </section>

              {!isNew && form.id ? (
                <>
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
              </>
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
  provider: ModelEndpointConfig | null,
) {
  if (!provider) {
    return null;
  }

  const matches = contributions.filter((item) => item.provider.kind === provider.kind);
  return matches.length === 1 ? matches[0] : null;
}

function resolveAgentModelId(provider: ModelEndpointConfig | null, currentModelId: string) {
  const normalizedCurrentModelId = currentModelId.trim();
  const models = provider?.available_models ?? [];
  if (normalizedCurrentModelId && models.some((model) => model.id === normalizedCurrentModelId)) {
    return normalizedCurrentModelId;
  }
  if (provider?.default_model?.trim()) {
    return provider.default_model.trim();
  }
  return models[0]?.id ?? normalizedCurrentModelId;
}

function buildAgentModelOptions(
  provider: ModelEndpointConfig | null,
  currentModelId: string,
  t: (key: string, fallback: string, params?: Record<string, string | number>) => string,
) {
  const options = (provider?.available_models ?? []).map((model) => ({ value: model.id, label: model.id }));
  const normalizedCurrentModelId = currentModelId.trim();
  if (!normalizedCurrentModelId || options.some((option) => option.value === normalizedCurrentModelId)) {
    return options;
  }
  return [
    {
      value: normalizedCurrentModelId,
      label: `${normalizedCurrentModelId} · ${t("web.agents.model_unknown_current", "当前值")}`,
    },
    ...options,
  ];
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
    permission_profile: normalizePermissionProfile(form.permission_profile),
    file_access: normalizeFileAccessProfile(form.file_access),
  };
}

function normalizeAgentForm(form: AgentProfile): AgentProfile {
  return {
    ...form,
    skills: [...(form.skills ?? [])],
    generation_options: { ...(form.generation_options ?? {}) },
    permission_profile: normalizePermissionProfile(form.permission_profile),
    file_access: normalizeFileAccessProfile(form.file_access),
  };
}

function normalizePermissionProfile(profile: AgentPermissionProfile | undefined): AgentPermissionProfile {
  return {
    mode: profile?.mode === "blacklist" ? "blacklist" : "whitelist",
    entries: [...(profile?.entries ?? [])]
      .map((entry) => ({
        match: normalizeCommandPermissionMatch(entry.match),
        value: entry.value.trim(),
      }))
      .filter((entry) => entry.value.length > 0),
  };
}

function normalizeFileAccessProfile(
  value: AgentFileAccessProfile | undefined,
): AgentFileAccessProfile {
  const roots = (value?.roots?.length ? value.roots : DEFAULT_FILE_ACCESS_PROFILE.roots)
    .map((root) => ({
      id: root.id.trim(),
      path: normalizeVirtualRoot(root.path),
      mode: root.mode.trim() || "read_write",
    }))
    .filter((root) => root.id.length > 0 && root.path.length > 0);
  const defaultRoot = normalizeVirtualRoot(value?.default_root ?? DEFAULT_FILE_ACCESS_PROFILE.default_root);
  return {
    default_root: roots.some((root) => root.path === defaultRoot)
      ? defaultRoot
      : DEFAULT_FILE_ACCESS_PROFILE.default_root,
    roots: roots.length > 0 ? roots : DEFAULT_FILE_ACCESS_PROFILE.roots,
  };
}

function normalizeVirtualRoot(value: string | undefined) {
  const normalized = (value ?? "").trim().replaceAll("\\", "/").replace(/\/+$/u, "");
  if (!normalized) {
    return "/workspace";
  }
  return normalized.startsWith("/") ? normalized : `/${normalized}`;
}

function normalizeCommandPermissionMatch(value: string | undefined) {
  if (value === "exact" || value === "regex") {
    return value;
  }
  return "prefix";
}

function CommandPermissionEntriesEditor({
  t,
  entries,
  onChange,
}: {
  t: (key: string, fallback: string, params?: Record<string, string | number>) => string;
  entries: AgentPermissionCommandEntry[];
  onChange: (entries: AgentPermissionCommandEntry[]) => void;
}) {
  function handleItemChange(
    itemIndex: number,
    patch: Partial<AgentPermissionCommandEntry>,
  ) {
    onChange(
      entries.map((entry, index) =>
        index === itemIndex ? { ...entry, ...patch } : entry,
      ),
    );
  }

  function handleItemRemove(itemIndex: number) {
    onChange(entries.filter((_, index) => index !== itemIndex));
  }

  function handleItemAdd() {
    onChange([...entries, { match: "prefix", value: "" }]);
  }

  return (
    <div className="stack">
      <div className="model-list">
        {entries.length > 0 ? (
          entries.map((entry, itemIndex) => (
            <div key={`permission-entry-${itemIndex}`} className="model-row">
              <Select
                value={normalizeCommandPermissionMatch(entry.match)}
                onChange={(value) =>
                  handleItemChange(itemIndex, {
                    match: normalizeCommandPermissionMatch(value),
                  })}
                options={buildCommandPermissionMatchOptions(t)}
              />
              <input
                value={entry.value}
                placeholder={commandPermissionEntryPlaceholder(entry.match, t)}
                onChange={(event) =>
                  handleItemChange(itemIndex, { value: event.target.value })}
              />
              <button
                type="button"
                className="secondary"
                onClick={() => handleItemRemove(itemIndex)}
              >
                {t("web.action.delete", "删除")}
              </button>
            </div>
          ))
        ) : (
          <div className="empty-card agent-editor__empty-state agent-policy-list-empty">
            <strong>{t("web.agents.empty_items_title", "当前没有条目")}</strong>
            <p>{t("web.agents.empty_items_body", "点击下方“新增条目”后，再补充命令匹配规则。")}</p>
          </div>
        )}
        <button type="button" className="secondary" onClick={handleItemAdd}>
          {t("web.permissions.add_entry", "新增条目")}
        </button>
      </div>
    </div>
  );
}

function commandPermissionEntryPlaceholder(
  match: string | undefined,
  t: (key: string, fallback: string, params?: Record<string, string | number>) => string,
) {
  switch (normalizeCommandPermissionMatch(match)) {
    case "exact":
      return t("web.permissions.entry_placeholder_exact", "例如 git status");
    case "regex":
      return t("web.permissions.entry_placeholder_regex", String.raw`例如 ^git\s+(push|pull)(\s|$)`);
    default:
      return t("web.permissions.entry_placeholder_prefix", "例如 git");
  }
}
