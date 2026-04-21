import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createProvider,
  deleteProvider,
  listProviders,
  updateProvider,
  type ProviderConfig,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

const EMPTY_CHANNEL: ProviderConfig = {
  id: "",
  display_name: "",
  kind: "",
  description: "",
  base_url: "",
  api_key_env: "",
  default_model: "",
  available_models: [],
  enabled: true,
};

export function ApiChannelEditorView({ channelId }: { channelId: string }) {
  const { runtime, t } = useUiHelpers();
  const [channels, setChannels] = useState<ProviderConfig[]>([]);
  const [form, setForm] = useState<ProviderConfig>(EMPTY_CHANNEL);
  const [modelsText, setModelsText] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const isNew = channelId.startsWith("new-");

  const interfaceTypes = useMemo(() => {
    const kinds = new Set<string>();
    for (const channel of channels) {
      if (channel.kind) {
        kinds.add(channel.kind);
      }
    }
    for (const contribution of runtime?.registry.providers ?? []) {
      const kind = contribution.provider.kind || contribution.provider.id;
      if (kind) {
        kinds.add(kind);
      }
    }
    return [...kinds].sort();
  }, [channels, runtime?.registry.providers]);

  useEffect(() => {
    void hydrate();
  }, [channelId]);

  async function hydrate() {
    setError(null);
    try {
      const next = await listProviders();
      setChannels(next);

      if (isNew) {
        const defaultKind =
          next.find((item) => item.kind)?.kind ??
          runtime?.registry.providers[0]?.provider.kind ??
          runtime?.registry.providers[0]?.provider.id ??
          "";
        setForm({ ...EMPTY_CHANNEL, kind: defaultKind });
        setModelsText("");
        return;
      }

      const current = next.find((item) => item.id === channelId);
      if (current) {
        setForm({ ...current, available_models: [...current.available_models] });
        setModelsText(current.available_models.join(", "));
      }
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);

    const payload = {
      ...form,
      available_models: modelsText
        .split(",")
        .map((item) => item.trim())
        .filter(Boolean),
    };

    try {
      if (isNew) {
        await createProvider(payload);
      } else {
        await updateProvider(channelId, payload);
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
      await deleteProvider(form.id);
      setForm(EMPTY_CHANNEL);
      setModelsText("");
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
          <span className="resource-editor__eyebrow">
            {t("web.channels.eyebrow", "API 上游渠道")}
          </span>
          <h2>{isNew ? t("web.channels.new", "新建渠道") : form.display_name || form.id}</h2>
          <p>
            {t(
              "web.channels.editor_description",
              "一个渠道就是一个可绑定给 Agent 的实际访问入口；接口类型只表示已安装实现，不展示实现清单。",
            )}
          </p>
        </div>
      </div>
      {error ? <div className="error">{error}</div> : null}
      <div className="form-grid">
        <label>
          ID
          <input
            value={form.id}
            onChange={(event) => setForm({ ...form, id: event.target.value })}
            required
          />
        </label>
        <label>
          {t("web.channels.display_name", "显示名")}
          <input
            value={form.display_name}
            onChange={(event) => setForm({ ...form, display_name: event.target.value })}
            required
          />
        </label>
        <label>
          {t("web.channels.interface_type", "接口类型")}
          <select
            value={form.kind}
            onChange={(event) => setForm({ ...form, kind: event.target.value })}
            disabled={!isNew}
          >
            {interfaceTypes.map((kind) => (
              <option key={kind} value={kind}>
                {kind}
              </option>
            ))}
          </select>
          <p className="helper-text">
            {t(
              "web.channels.interface_type_help",
              "这里选择当前已经装配完成的上游接口类型；扩展装入后会自动出现在这里。",
            )}
          </p>
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
        {t("web.channels.base_url", "Base URL")}
        <input
          value={form.base_url}
          onChange={(event) => setForm({ ...form, base_url: event.target.value })}
        />
      </label>
      <label>
        {t("web.channels.api_key_env", "API Key 环境变量")}
        <input
          value={form.api_key_env}
          onChange={(event) => setForm({ ...form, api_key_env: event.target.value })}
        />
      </label>
      <div className="form-grid">
        <label>
          {t("web.channels.default_model", "默认模型")}
          <input
            value={form.default_model}
            onChange={(event) => setForm({ ...form, default_model: event.target.value })}
          />
        </label>
        <label>
          {t("web.channels.models", "可选模型")}
          <input value={modelsText} onChange={(event) => setModelsText(event.target.value)} />
        </label>
      </div>
      <label>
        {t("web.channels.description_field", "描述")}
        <textarea
          value={form.description}
          onChange={(event) => setForm({ ...form, description: event.target.value })}
          rows={4}
        />
      </label>
      <div className="button-row">
        <button type="submit" disabled={busy}>
          {t("web.action.save", "保存")}
        </button>
        <button
          type="button"
          className="danger"
          disabled={busy || isNew || !form.id}
          onClick={() => void handleDelete()}
        >
          {t("web.action.delete", "删除")}
        </button>
      </div>
    </form>
  );
}
