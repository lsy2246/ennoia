import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createProvider,
  deleteProvider,
  listProviders,
  updateProvider,
  type ProviderConfig,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

const EMPTY_CHANNEL: ProviderConfig = {
  id: "",
  display_name: "",
  kind: "openai",
  description: "",
  base_url: "https://api.openai.com/v1",
  api_key_env: "OPENAI_API_KEY",
  default_model: "gpt-5.4",
  available_models: ["gpt-5.4"],
  enabled: true,
};

const BUILTIN_INTERFACE_TYPES = ["openai"];

export function ProvidersPage() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [channels, setChannels] = useState<ProviderConfig[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      setChannels(await listProviders());
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="resource-layout resource-layout--single">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.channels.eyebrow", "API 上游渠道")}</span>
          <h1>{t("web.channels.title", "API 上游渠道是 Agent 访问模型能力的具体渠道实例。")}</h1>
          <p>{t("web.channels.description", "接口类型只在创建渠道时选择；日常使用和绑定都围绕渠道实例展开。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="button-row">
          <button
            type="button"
            onClick={() =>
              openView({
                kind: "api-channel",
                entityId: `new-${Date.now()}`,
                title: t("web.channels.new", "新建渠道"),
                subtitle: t("web.channels.edit", "编辑 API 上游渠道"),
              })}
          >
            {t("web.channels.new", "新建渠道")}
          </button>
          <button type="button" className="secondary" onClick={() => void refresh()}>
            {t("web.action.refresh", "刷新")}
          </button>
        </div>
        <div className="card-grid">
          {channels.map((channel) => (
            <article key={channel.id} className="resource-card">
              <header>
                <strong>{channel.display_name}</strong>
                <span>{channel.enabled ? t("web.common.enabled", "启用") : t("web.common.disabled", "停用")}</span>
              </header>
              <p>{channel.description || t("web.common.none", "无")}</p>
              <div className="tag-row">
                <span>{channel.kind}</span>
                <span>{channel.default_model}</span>
              </div>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "api-channel",
                      entityId: channel.id,
                      title: channel.display_name,
                      subtitle: channel.kind,
                    })}
                >
                  {t("web.action.open", "打开")}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
}

export function ApiChannelEditorView({ channelId }: { channelId: string }) {
  const { runtime, t } = useUiHelpers();
  const [channels, setChannels] = useState<ProviderConfig[]>([]);
  const [form, setForm] = useState<ProviderConfig>(EMPTY_CHANNEL);
  const [modelsText, setModelsText] = useState(EMPTY_CHANNEL.available_models.join(", "));
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const isNew = channelId.startsWith("new-");

  useEffect(() => {
    void hydrate();
  }, [channelId]);

  const interfaceTypes = useMemo(() => {
    const kinds = new Set(BUILTIN_INTERFACE_TYPES);
    for (const contribution of runtime?.registry.providers ?? []) {
      kinds.add(contribution.provider.kind || contribution.provider.id);
    }
    return [...kinds].sort();
  }, [runtime?.registry.providers]);

  async function hydrate() {
    setError(null);
    try {
      const next = await listProviders();
      setChannels(next);
      if (isNew) {
        setForm(EMPTY_CHANNEL);
        setModelsText(EMPTY_CHANNEL.available_models.join(", "));
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
      available_models: modelsText.split(",").map((item) => item.trim()).filter(Boolean),
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
      setModelsText(EMPTY_CHANNEL.available_models.join(", "));
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
          <span className="resource-editor__eyebrow">{t("web.channels.eyebrow", "API 上游渠道")}</span>
          <h2>{isNew ? t("web.channels.new", "新建渠道") : form.display_name || form.id}</h2>
          <p>{t("web.channels.editor_description", "一个渠道就是一个可绑定给 Agent 的实际访问入口；接口类型只表示已安装实现，不展示实现清单。")}</p>
        </div>
      </div>
      {error ? <div className="error">{error}</div> : null}
      <div className="form-grid">
        <label>
          ID
          <input value={form.id} onChange={(event) => setForm({ ...form, id: event.target.value })} required />
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
          <p className="helper-text">{t("web.channels.interface_type_help", "这里只能选择当前已实现的接口类型；扩展安装后会自动出现在这里。")}</p>
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
        <input value={form.base_url} onChange={(event) => setForm({ ...form, base_url: event.target.value })} />
      </label>
      <label>
        {t("web.channels.api_key_env", "API Key 环境变量")}
        <input value={form.api_key_env} onChange={(event) => setForm({ ...form, api_key_env: event.target.value })} />
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
        <button type="button" className="danger" disabled={busy || isNew || !form.id} onClick={() => void handleDelete()}>
          {t("web.action.delete", "删除")}
        </button>
      </div>
    </form>
  );
}
