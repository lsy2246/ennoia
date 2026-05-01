import { useCallback, useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createProvider,
  deleteProvider,
  discoverProviderModels,
  listProviders,
  updateProvider,
  type ProviderConfig,
  type ProviderModelDescriptor,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useProvidersStore } from "@/stores/providers";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

const EMPTY_CHANNEL: ProviderConfig = {
  id: "",
  display_name: "",
  kind: "",
  description: "",
  base_url: "",
  api_key_env: "",
  default_model: "",
  available_models: [],
  model_discovery: {
    manual_allowed: true,
  },
  enabled: true,
};

type ModelEntry = {
  key: string;
  id: string;
  maxContextTokens: string;
  maxInputTokens: string;
};

let modelSequence = 0;

function stringifyTokenValue(value?: number | null) {
  return typeof value === "number" && Number.isFinite(value) ? String(value) : "";
}

function createModelEntry(model?: ProviderModelDescriptor): ModelEntry {
  modelSequence += 1;
  return {
    key: `model-${modelSequence}`,
    id: model?.id ?? "",
    maxContextTokens: stringifyTokenValue(model?.max_context_tokens),
    maxInputTokens: stringifyTokenValue(model?.max_input_tokens),
  };
}

function parseTokenValue(value?: string | null) {
  const normalized = typeof value === "string" ? value.trim() : "";
  if (!normalized) {
    return null;
  }
  if (!/^\d+$/.test(normalized)) {
    return null;
  }
  const parsed = Number.parseInt(normalized, 10);
  if (!Number.isSafeInteger(parsed) || parsed <= 0) {
    return null;
  }
  return parsed;
}

function normalizeModelDescriptors(models: Array<ProviderModelDescriptor | string | null | undefined>) {
  const seen = new Set<string>();
  const normalized: ProviderModelDescriptor[] = [];
  for (const model of models) {
    const id = (typeof model === "string" ? model : model?.id ?? "").trim();
    if (!id || seen.has(id)) {
      continue;
    }
    seen.add(id);
    normalized.push({
      id,
      max_context_tokens: typeof model === "string" ? null : model?.max_context_tokens ?? null,
      max_input_tokens: typeof model === "string" ? null : model?.max_input_tokens ?? null,
    });
  }
  return normalized;
}

function serializeModelEntries(entries: ModelEntry[]) {
  return normalizeModelDescriptors(
    entries.map((entry) => ({
      id: entry.id,
      max_context_tokens: parseTokenValue(entry.maxContextTokens),
      max_input_tokens: parseTokenValue(entry.maxInputTokens),
    })),
  );
}

function resolveProviderImplementationKind(
  contribution: NonNullable<ReturnType<typeof useUiHelpers>["runtime"]>["registry"]["providers"][number],
) {
  return contribution.provider.kind || contribution.provider.id || "";
}

export function ApiChannelEditorView({ channelId, panelId }: { channelId: string; panelId?: string }) {
  const { runtime, t } = useUiHelpers();
  const closeView = useWorkbenchStore((state) => state.closeView);
  const updateViewDescriptor = useWorkbenchStore((state) => state.updateViewDescriptor);
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const notifyProvidersChanged = useProvidersStore((state) => state.notifyChanged);
  const [form, setForm] = useState<ProviderConfig>(EMPTY_CHANNEL);
  const [modelEntries, setModelEntries] = useState<ModelEntry[]>([]);
  const [defaultModelKey, setDefaultModelKey] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [modelsBusy, setModelsBusy] = useState(false);
  const isNew = channelId.startsWith("new-");

  const interfaceTypes = useMemo(() => {
    return (runtime?.registry.providers ?? [])
      .map((contribution) => {
        const kind = resolveProviderImplementationKind(contribution);
        return kind ? [kind, kind] : null;
      })
      .filter((item): item is [string, string] => item !== null)
      .filter(([kind], index, entries) => entries.findIndex(([entryKind]) => entryKind === kind) === index)
      .sort(([left], [right]) => left.localeCompare(right));
  }, [runtime?.registry.providers]);

  const providerContributions = useMemo(
    () => runtime?.registry.providers ?? [],
    [runtime?.registry.providers],
  );
  const selectedContribution = useMemo(
    () =>
      providerContributions.find(
        (item) => resolveProviderImplementationKind(item) === form.kind,
      ) ?? null,
    [form.kind, providerContributions],
  );

  useEffect(() => {
    if (!workbenchApi || !panelId) {
      return;
    }

    const nextTitle =
      (form.display_name ?? "").trim() || (form.id ?? "").trim() || (isNew ? t("web.channels.new", "新建渠道") : channelId);
    const panel = workbenchApi.getPanel?.(panelId);
    panel?.api?.setTitle?.(nextTitle);
    updateViewDescriptor(panelId, { title: nextTitle });
  }, [channelId, form.display_name, form.id, isNew, panelId, t, updateViewDescriptor, workbenchApi]);

  const applyModelState = useCallback((models: ProviderModelDescriptor[], preferredDefault?: string) => {
    const normalizedModels = normalizeModelDescriptors(models);
    const entries = normalizedModels.map((item) => createModelEntry(item));
    const defaultModel =
      preferredDefault && normalizedModels.some((item) => item.id === preferredDefault)
        ? preferredDefault
        : normalizedModels[0]?.id ?? "";
    const defaultEntry = entries.find((entry) => entry.id === defaultModel) ?? entries[0] ?? null;

    setModelEntries(entries);
    setDefaultModelKey(defaultEntry?.key ?? null);
    return { models: normalizedModels, defaultModel };
  }, []);

  function syncFormModels(entries: ModelEntry[], nextDefaultKey: string | null) {
    const normalizedModels = serializeModelEntries(entries);
    const defaultEntry = entries.find((entry) => entry.key === nextDefaultKey);
    const preferredDefault = (defaultEntry?.id ?? "").trim();
    const defaultModel =
      preferredDefault && normalizedModels.some((item) => item.id === preferredDefault)
        ? preferredDefault
        : normalizedModels[0]?.id ?? "";

    setForm((current) => ({
      ...current,
      available_models: normalizedModels,
      default_model: defaultModel,
    }));
  }

  const applyInterfaceDefaults = useCallback(
    (kind: string, options?: { resetIdentity?: boolean }) => {
      const contribution = providerContributions.find(
        (item) => resolveProviderImplementationKind(item) === kind,
      );
      const modelDiscovery = {
        manual_allowed: contribution?.provider.model_discovery
          ? contribution.provider.manual_model
          : true,
      };

      setForm((current) => ({
        ...(options?.resetIdentity ? EMPTY_CHANNEL : current),
        kind,
        model_discovery: modelDiscovery,
      }));
      if (options?.resetIdentity) {
        setModelEntries([]);
        setDefaultModelKey(null);
      }
    },
    [providerContributions],
  );

  const hydrate = useCallback(async () => {
    setError(null);
    try {
      const next = await listProviders();

      if (isNew) {
        const defaultKind = interfaceTypes[0]?.[0] ?? "";
        applyInterfaceDefaults(defaultKind, { resetIdentity: true });
        return;
      }

      const current = next.find((item) => item.id === channelId);
      if (current) {
        const normalized = applyModelState(current.available_models, current.default_model);
        setForm({
          ...current,
          available_models: normalized.models,
          default_model: normalized.defaultModel,
        });
      }
    } catch (err) {
      setError(String(err));
    }
  }, [applyInterfaceDefaults, applyModelState, channelId, interfaceTypes, isNew]);

  useEffect(() => {
    void hydrate();
  }, [hydrate]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);

    try {
      const payload = {
        ...form,
        available_models: serializeModelEntries(modelEntries),
      };
      payload.kind = (payload.kind ?? "").trim();
      payload.default_model =
        payload.available_models.find(
          (model) =>
            ((modelEntries.find((entry) => entry.key === defaultModelKey)?.id ?? "").trim() === model.id),
        )?.id ??
        payload.available_models[0]?.id ??
        "";

      if (!payload.kind) {
        throw new Error(
          t("web.channels.interface_type_required", "请先选择一个可用的接口类型。"),
        );
      }

      const saved = isNew
        ? await createProvider(payload)
        : await updateProvider(channelId, payload);

      if (panelId && workbenchApi) {
        const panel = workbenchApi.getPanel?.(panelId);
        const nextTitle = saved.display_name || saved.id || payload.display_name || payload.id;
        panel?.api?.setTitle?.(nextTitle);
        panel?.update?.({
          params: {
            panelKind: "resource",
            descriptor: {
              ...(panel?.params?.descriptor ?? {}),
              panelId,
              kind: "api-channel",
              entityId: saved.id,
              title: nextTitle,
              subtitle: saved.kind,
              openedAt: Date.now(),
            },
          },
        });
        updateViewDescriptor(panelId, {
          kind: "api-channel",
          entityId: saved.id,
          title: nextTitle,
          subtitle: saved.kind,
        });
      }

      const normalized = applyModelState(saved.available_models, saved.default_model);
      setForm({
        ...saved,
        available_models: normalized.models,
        default_model: normalized.defaultModel,
      });
      if (isNew) {
        notifyProvidersChanged();
        return;
      }
      notifyProvidersChanged();
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
      notifyProvidersChanged();
      if (panelId) {
        closeView(panelId);
        return;
      }
      setForm(EMPTY_CHANNEL);
      setModelEntries([]);
      setDefaultModelKey(null);
      await hydrate();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDiscoverModels() {
    setModelsBusy(true);
    setError(null);
    try {
      const response = await discoverProviderModels({
        ...form,
        available_models: serializeModelEntries(modelEntries),
      });
      const nextModels = response.models.length > 0 ? response.models : form.available_models;
      const normalized = applyModelState(nextModels, form.default_model);
      setForm((current) => ({
        ...current,
        available_models: normalized.models,
        default_model: normalized.defaultModel,
      }));
    } catch (err) {
      setError(String(err));
    } finally {
      setModelsBusy(false);
    }
  }

  function handleModelChange(key: string, value: string) {
    const nextEntries = modelEntries.map((entry) => (entry.key === key ? { ...entry, id: value } : entry));
    setModelEntries(nextEntries);
    syncFormModels(nextEntries, defaultModelKey);
  }

  function handleModelBudgetChange(
    key: string,
    field: "maxContextTokens" | "maxInputTokens",
    value: string,
  ) {
    const nextEntries = modelEntries.map((entry) =>
      entry.key === key ? { ...entry, [field]: value } : entry,
    );
    setModelEntries(nextEntries);
    syncFormModels(nextEntries, defaultModelKey);
  }

  function handleModelAdd() {
    const nextEntry = createModelEntry();
    const nextEntries = [...modelEntries, nextEntry];
    const nextDefaultKey = defaultModelKey ?? nextEntry.key;
    setModelEntries(nextEntries);
    setDefaultModelKey(nextDefaultKey);
    syncFormModels(nextEntries, nextDefaultKey);
  }

  function handleModelRemove(key: string) {
    const nextEntries = modelEntries.filter((entry) => entry.key !== key);
    const nextDefaultKey =
      defaultModelKey === key ? (nextEntries[0]?.key ?? null) : defaultModelKey;
    setModelEntries(nextEntries);
    setDefaultModelKey(nextDefaultKey);
    syncFormModels(nextEntries, nextDefaultKey);
  }

  function handleDefaultModelSelect(key: string) {
    setDefaultModelKey(key);
    syncFormModels(modelEntries, key);
  }

  return (
    <form className="resource-editor" onSubmit={handleSubmit}>
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
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
            onChange={(event) => applyInterfaceDefaults(event.target.value)}
            disabled={!isNew}
          >
            {interfaceTypes.length === 0 ? (
              <option value="">
                {t("web.channels.interface_type_empty", "当前没有可用接口类型")}
              </option>
            ) : null}
            {interfaceTypes.map(([kind, label]) => (
              <option key={kind} value={kind}>
                {label}
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
            required={form.enabled}
            readOnly
          />
          <p className="helper-text">
            {selectedContribution?.provider.model_discovery
              ? t("web.channels.model_discovery_extension", "当前接口实现可以返回模型列表；保存时仍以这里的默认模型为准。")
              : t("web.channels.model_discovery_manual", "当前接口没有模型发现能力，请手动维护模型列表和默认模型。")}
          </p>
        </label>
        <div className="stack">
          <div className="model-toolbar">
            <div>
              <div className="panel-title">{t("web.channels.models", "模型列表")}</div>
              <p className="helper-text">
                {t(
                  "web.channels.models_help",
                  "每行一个模型，直接维护模型名、总上下文大小和最大输入上限；保存前会自动去重并清理空项。",
                )}
              </p>
            </div>
            {selectedContribution?.provider.model_discovery ? (
              <button
                type="button"
                className="secondary"
                disabled={modelsBusy || busy || !(form.kind ?? "").trim()}
                onClick={() => void handleDiscoverModels()}
              >
                {t("web.channels.refresh_models", "一键获取上游模型")}
              </button>
            ) : null}
          </div>
          <div className="model-list">
            {modelEntries.length > 0 ? (
              modelEntries.map((entry) => (
                <div key={entry.key} className="model-row">
                  <input
                    value={entry.id}
                    placeholder={t("web.channels.model_placeholder", "模型 ID，例如 gpt-5.4")}
                    onChange={(event) => handleModelChange(entry.key, event.target.value)}
                  />
                  <input
                    type="number"
                    min="1"
                    value={entry.maxContextTokens}
                    placeholder={t("web.channels.model_context_placeholder", "总上下文大小")}
                    onChange={(event) =>
                      handleModelBudgetChange(entry.key, "maxContextTokens", event.target.value)
                    }
                  />
                  <input
                    type="number"
                    min="1"
                    value={entry.maxInputTokens}
                    placeholder={t("web.channels.model_input_placeholder", "最大输入上限")}
                    onChange={(event) =>
                      handleModelBudgetChange(entry.key, "maxInputTokens", event.target.value)
                    }
                  />
                  <button
                    type="button"
                    className={defaultModelKey === entry.key ? "secondary" : ""}
                    onClick={() => handleDefaultModelSelect(entry.key)}
                  >
                    {defaultModelKey === entry.key
                      ? t("web.channels.model_is_default", "默认模型")
                      : t("web.channels.model_set_default", "设为默认")}
                  </button>
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => handleModelRemove(entry.key)}
                  >
                    {t("web.action.delete", "删除")}
                  </button>
                </div>
              ))
            ) : (
              <div className="empty-card">
                {t("web.channels.models_empty", "还没有模型。先新增一项，或直接一键获取上游模型。")}
              </div>
            )}
            <button type="button" className="secondary" onClick={handleModelAdd}>
              {t("web.channels.model_add", "新增模型")}
            </button>
          </div>
        </div>
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
        <button type="submit" disabled={busy || (isNew && interfaceTypes.length === 0)}>
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
