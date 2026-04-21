import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createAgent,
  deleteAgent,
  listAgents,
  listProviders,
  listSkills,
  updateAgent,
  type AgentProfile,
  type ProviderConfig,
  type SkillConfig,
} from "@ennoia/api-client";
import { formatRelativePath } from "@/lib/pathDisplay";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

const EMPTY_AGENT: AgentProfile = {
  id: "",
  display_name: "",
  description: "",
  system_prompt: "",
  provider_id: "",
  model_id: "",
  reasoning_effort: "high",
  workspace_root: "",
  skills: [],
  enabled: true,
};

export function AgentsPage() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const [nextAgents, nextProviders] = await Promise.all([listAgents(), listProviders()]);
      setAgents(nextAgents);
      setProviders(nextProviders);
    } catch (err) {
      setError(String(err));
    }
  }

  function providerLabel(providerId: string) {
    return providers.find((item) => item.id === providerId)?.display_name ?? providerId;
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.agents.eyebrow", "Agent Registry")}</span>
          <h1>{t("web.agents.title", "Agent 是可配置的协作者档案。")}</h1>
          <p>{t("web.agents.description", "从这里查看 Agent 清单，并把任意 Agent 作为独立工作视图打开。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="button-row">
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "agent",
                entityId: `new-${Date.now()}`,
                title: t("web.agents.new", "新建 Agent"),
                subtitle: t("web.agents.edit", "编辑 Agent"),
              })}
          >
            {t("web.agents.new", "新建 Agent")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="card-grid">
          {agents.map((agent) => (
            <article key={agent.id} className="resource-card">
              <header>
                <strong>{agent.display_name}</strong>
                <span>{agent.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{agent.description || t("web.common.none", "无")}</p>
              <div className="tag-row">
                <span>{providerLabel(agent.provider_id)}</span>
                <span>{agent.model_id}</span>
                <span>{agent.reasoning_effort}</span>
                <span>{agent.skills.length} skills</span>
              </div>
              <p className="helper-text">
                {t("web.agents.derived_workspace_help", "工作区根路径只在设置中配置。Agent 工作区自动派生为 workspace/agents/{agent_id}。")}
                {" · "}
                {formatRelativePath(agent.workspace_root)}
              </p>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "agent",
                      entityId: agent.id,
                      title: agent.display_name,
                      subtitle: providerLabel(agent.provider_id),
                    })}
                >
                  {t("web.action.open", "打开")}
                </button>
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "api-channel",
                      entityId: agent.provider_id,
                      title: providerLabel(agent.provider_id),
                      subtitle: t("web.channels.eyebrow", "API 上游渠道"),
                    })}
                >
                  {t("web.agents.open_channel", "打开渠道")}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export function AgentEditorView({
  agentId,
  onOpenApiChannel,
}: {
  agentId: string;
  onOpenApiChannel: (channelId: string) => void;
}) {
  const { t } = useUiHelpers();
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [providers, setProviders] = useState<ProviderConfig[]>([]);
  const [form, setForm] = useState<AgentProfile>(EMPTY_AGENT);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const isNew = agentId.startsWith("new-");

  useEffect(() => {
    void hydrate();
  }, [agentId]);

  const selectedProvider = useMemo(
    () => providers.find((provider) => provider.id === form.provider_id) ?? providers[0] ?? null,
    [form.provider_id, providers],
  );

  async function hydrate() {
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
        });
        return;
      }
      const current = nextAgents.find((item) => item.id === agentId);
      if (current) {
        setForm({ ...current, skills: [...current.skills] });
      }
    } catch (err) {
      setError(String(err));
    }
  }

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
      if (isNew) {
        await createAgent(form);
      } else {
        await updateAgent(agentId, form);
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

  return (
    <form className="resource-editor" onSubmit={handleSubmit}>
      <div className="resource-editor__header">
        <div>
          <span className="resource-editor__eyebrow">{t("web.agents.eyebrow", "Agent Registry")}</span>
          <h2>{isNew ? t("web.agents.new", "新建 Agent") : form.display_name || form.id}</h2>
          <p>{t("web.agents.editor_description", "一个 Agent 就是一个可长期维护的协作者档案。")}</p>
        </div>
        {form.provider_id ? (
          <button type="button" className="secondary" onClick={() => onOpenApiChannel(form.provider_id)}>
            {t("web.agents.open_channel", "打开渠道")}
          </button>
        ) : null}
      </div>
      {error ? <div className="error">{error}</div> : null}
      <div className="form-grid">
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
          <select
            value={form.provider_id}
            onChange={(event) => {
              const provider = providers.find((item) => item.id === event.target.value);
              setForm({
                ...form,
                provider_id: event.target.value,
                model_id: provider?.default_model ?? form.model_id,
              });
            }}
          >
            {providers.map((provider) => (
              <option key={provider.id} value={provider.id}>
                {provider.display_name}
              </option>
            ))}
          </select>
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
              <option key={model} value={model} />
            ))}
          </datalist>
        </label>
        <label>
          {t("web.agents.reasoning", "思考等级")}
          <select
            value={form.reasoning_effort}
            onChange={(event) => setForm({ ...form, reasoning_effort: event.target.value })}
          >
            <option value="low">low</option>
            <option value="medium">medium</option>
            <option value="high">high</option>
            <option value="max">max</option>
          </select>
        </label>
        <label className="check-row">
          <input
            type="checkbox"
            checked={form.enabled}
            onChange={(event) => setForm({ ...form, enabled: event.target.checked })}
          />
          {t("web.common.enabled", "启用")}
        </label>
      </div>
      <label>
        {t("web.agents.description_field", "描述")}
        <textarea
          value={form.description}
          onChange={(event) => setForm({ ...form, description: event.target.value })}
          rows={3}
        />
      </label>
      <label>
        {t("web.agents.system_prompt", "System Prompt")}
        <textarea
          value={form.system_prompt}
          onChange={(event) => setForm({ ...form, system_prompt: event.target.value })}
          rows={7}
        />
      </label>
      <div className="stack">
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
      </div>
      <div className="details-panel">
        <div className="panel-title">{t("web.agents.derived_workspace", "派生工作区")}</div>
        <div className="kv-list">
          <span>{t("web.agents.derived_workspace", "派生工作区")}</span>
          <strong>{formatRelativePath(form.workspace_root || form.workspace_dir || "")}</strong>
          <span>{t("web.agents.skills", "技能")}</span>
          <strong>{formatRelativePath(form.skills_dir || "")}</strong>
        </div>
        <p className="helper-text">{t("web.agents.derived_workspace_help", "工作区根路径只在设置中配置。Agent 工作区自动派生为 workspace/agents/{agent_id}。")}</p>
      </div>
      <div className="button-row">
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
