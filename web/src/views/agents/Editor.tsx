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
import type { ExtensionProviderContribution } from "@ennoia/ui-sdk";
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


export function AgentEditorView({
  agentId,
  onOpenApiChannel,
}: {
  agentId: string;
  onOpenApiChannel: (channelId: string) => void;
}) {
  const { resolveText, runtime, t } = useUiHelpers();
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
  const selectedProviderContribution = useMemo(
    () => findProviderContribution(runtime?.registry.providers ?? [], selectedProvider),
    [runtime?.registry.providers, selectedProvider],
  );
  const generationOptions = selectedProviderContribution?.provider.generation_options ?? [];

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
          generation_options: defaultGenerationOptions(
            findProviderContribution(runtime?.registry.providers ?? [], nextProviders[0] ?? null),
          ),
        });
        return;
      }
      const current = nextAgents.find((item) => item.id === agentId);
      if (current) {
        setForm({
          ...current,
          skills: [...current.skills],
          generation_options: { ...(current.generation_options ?? {}) },
        });
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
              const contribution = findProviderContribution(runtime?.registry.providers ?? [], provider ?? null);
              setForm({
                ...form,
                provider_id: event.target.value,
                model_id: provider?.default_model ?? form.model_id,
                generation_options: defaultGenerationOptions(contribution),
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
        <label className="check-row">
          <input
            type="checkbox"
            checked={form.enabled}
            onChange={(event) => setForm({ ...form, enabled: event.target.checked })}
          />
          {t("web.common.enabled", "启用")}
        </label>
      </div>
      {generationOptions.length > 0 ? (
        <div className="details-panel">
          <div className="panel-title">{t("web.agents.generation_options", "生成参数")}</div>
          <p className="helper-text">
            {t("web.agents.generation_options_help", "这些参数由当前上游扩展声明；未声明的上游不会显示。")}
          </p>
          <div className="form-grid">
            {generationOptions.map((option: ExtensionProviderContribution["provider"]["generation_options"][number]) => {
              const value = form.generation_options?.[option.id] ?? option.default_value ?? "";
              return (
                <label key={option.id}>
                  {resolveText(option.label)}
                  {option.value_type === "select" && option.allowed_values.length > 0 ? (
                    <select
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
                    >
                      {!option.required ? <option value="">{t("web.common.none", "无")}</option> : null}
                      {option.allowed_values.map((item: string) => (
                        <option key={item} value={item}>
                          {item}
                        </option>
                      ))}
                    </select>
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
        </div>
      ) : null}
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
        <div className="panel-title">{t("web.agents.working_dir", "工作目录")}</div>
        <div className="kv-list">
          <span>{t("web.agents.working_dir", "工作目录")}</span>
          <strong>{formatRelativePath(form.working_dir || "")}</strong>
          <span>{t("web.agents.skills", "技能")}</span>
          <strong>{formatRelativePath(form.skills_dir || "")}</strong>
        </div>
        <p className="helper-text">{t("web.agents.working_dir_help", "Agent 工作目录自动派生到 agents/{agent_id}/work，无需单独配置。")}</p>
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
