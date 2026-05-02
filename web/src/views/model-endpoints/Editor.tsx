import { useCallback, useDeferredValue, useEffect, useMemo, useRef, useState, type FormEvent } from "react";

import {
  createModelEndpoint,
  deleteModelEndpoint,
  discoverModelEndpointModels,
  listModelEndpoints,
  updateModelEndpoint,
  type ModelEndpointConfig,
  type ProviderModelDescriptor,
} from "@ennoia/api-client";
import { StatusNotice } from "@/components/StatusNotice";
import { useModelEndpointsStore } from "@/stores/modelEndpoints";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

const EMPTY_CHANNEL: ModelEndpointConfig = {
  id: "",
  display_name: "",
  kind: "",
  description: "",
  base_url: "",
  api_key: "",
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
const DISCOVERY_PAGE_SIZE_OPTIONS = [20, 50, 100] as const;
const DEFAULT_DISCOVERY_PAGE_SIZE = DISCOVERY_PAGE_SIZE_OPTIONS[0];

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

function resolveModelEndpointTemplate(kind: string, endpoints: ModelEndpointConfig[]) {
  const normalizedKind = kind.trim();
  if (!normalizedKind) {
    return null;
  }
  return endpoints.find((item) => item.id === normalizedKind)
    ?? endpoints.find((item) => item.kind === normalizedKind && item.id === item.kind)
    ?? endpoints.find((item) => item.kind === normalizedKind)
    ?? null;
}

function applyTemplateToDraft(
  current: ModelEndpointConfig,
  kind: string,
  template: ModelEndpointConfig | null,
  modelDiscovery: ModelEndpointConfig["model_discovery"],
  resetIdentity: boolean,
  models: ProviderModelDescriptor[],
  defaultModel: string,
) {
  const base = resetIdentity ? EMPTY_CHANNEL : current;
  return {
    ...base,
    kind,
    base_url: template?.base_url ?? base.base_url,
    api_key: base.api_key,
    api_key_env: template?.api_key_env ?? base.api_key_env,
    default_model: defaultModel,
    available_models: models,
    model_discovery: modelDiscovery,
  };
}

export function ModelEndpointEditorView({ modelEndpointId, panelId }: { modelEndpointId: string; panelId?: string }) {
  const { runtime, t } = useUiHelpers();
  const closeView = useWorkbenchStore((state) => state.closeView);
  const updateViewDescriptor = useWorkbenchStore((state) => state.updateViewDescriptor);
  const workbenchApi = useWorkbenchStore((state) => state.api);
  const notifyProvidersChanged = useModelEndpointsStore((state) => state.notifyChanged);
  const [form, setForm] = useState<ModelEndpointConfig>(EMPTY_CHANNEL);
  const [modelEntries, setModelEntries] = useState<ModelEntry[]>([]);
  const [defaultModelKey, setDefaultModelKey] = useState<string | null>(null);
  const [selectedModelKeys, setSelectedModelKeys] = useState<string[]>([]);
  const [discoveredModels, setDiscoveredModels] = useState<ProviderModelDescriptor[]>([]);
  const [discoveryOpen, setDiscoveryOpen] = useState(false);
  const [discoveryQuery, setDiscoveryQuery] = useState("");
  const [showUnaddedOnly, setShowUnaddedOnly] = useState(false);
  const [selectedDiscoveredIds, setSelectedDiscoveredIds] = useState<string[]>([]);
  const [discoveryPage, setDiscoveryPage] = useState(1);
  const [discoveryPageSize, setDiscoveryPageSize] = useState<number>(DEFAULT_DISCOVERY_PAGE_SIZE);
  const [modelEndpointTemplates, setModelEndpointTemplates] = useState<ModelEndpointConfig[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [modelsBusy, setModelsBusy] = useState(false);
  const draftInitializedRef = useRef(false);
  const isNew = modelEndpointId.startsWith("new-");
  const deferredDiscoveryQuery = useDeferredValue(discoveryQuery);

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
  const configuredModelIds = useMemo(
    () => new Set(modelEntries.map((entry) => entry.id.trim()).filter(Boolean)),
    [modelEntries],
  );
  const selectedModelKeySet = useMemo(() => new Set(selectedModelKeys), [selectedModelKeys]);
  const selectedDiscoveredIdSet = useMemo(() => new Set(selectedDiscoveredIds), [selectedDiscoveredIds]);
  const filteredDiscoveredModels = useMemo(() => {
    const query = deferredDiscoveryQuery.trim().toLowerCase();
    return discoveredModels.filter((model) => {
      const isAdded = configuredModelIds.has(model.id);
      if (showUnaddedOnly && isAdded) {
        return false;
      }
      if (!query) {
        return true;
      }
      return model.id.toLowerCase().includes(query);
    });
  }, [configuredModelIds, deferredDiscoveryQuery, discoveredModels, showUnaddedOnly]);
  const discoveryTotalPages = useMemo(
    () => Math.max(1, Math.ceil(filteredDiscoveredModels.length / discoveryPageSize)),
    [discoveryPageSize, filteredDiscoveredModels.length],
  );
  const pagedDiscoveredModels = useMemo(() => {
    const startIndex = (discoveryPage - 1) * discoveryPageSize;
    return filteredDiscoveredModels.slice(startIndex, startIndex + discoveryPageSize);
  }, [discoveryPage, discoveryPageSize, filteredDiscoveredModels]);
  const discoveryPageStart = filteredDiscoveredModels.length === 0 ? 0 : (discoveryPage - 1) * discoveryPageSize + 1;
  const discoveryPageEnd = Math.min(discoveryPage * discoveryPageSize, filteredDiscoveredModels.length);
  const visibleSelectableDiscoveredIds = useMemo(
    () => pagedDiscoveredModels
      .filter((model) => !configuredModelIds.has(model.id))
      .map((model) => model.id),
    [configuredModelIds, pagedDiscoveredModels],
  );
  const selectedImportableModels = useMemo(
    () => discoveredModels.filter((model) => selectedDiscoveredIdSet.has(model.id) && !configuredModelIds.has(model.id)),
    [configuredModelIds, discoveredModels, selectedDiscoveredIdSet],
  );

  useEffect(() => {
    if (!workbenchApi || !panelId) {
      return;
    }

    const nextTitle =
      (form.display_name ?? "").trim() || (form.id ?? "").trim() || (isNew ? t("web.model_endpoints.new", "新建模型接入") : modelEndpointId);
    const panel = workbenchApi.getPanel?.(panelId);
    panel?.api?.setTitle?.(nextTitle);
    updateViewDescriptor(panelId, { title: nextTitle });
  }, [modelEndpointId, form.display_name, form.id, isNew, panelId, t, updateViewDescriptor, workbenchApi]);

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
    setSelectedModelKeys([]);
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
    (kind: string, options?: { resetIdentity?: boolean; templates?: ModelEndpointConfig[] }) => {
      const contribution = providerContributions.find(
        (item) => resolveProviderImplementationKind(item) === kind,
      );
      const template = resolveModelEndpointTemplate(kind, options?.templates ?? modelEndpointTemplates);
      const modelDiscovery = {
        manual_allowed: contribution?.provider.model_discovery
          ? contribution.provider.manual_model
          : true,
      };
      const nextModels = normalizeModelDescriptors(template?.available_models ?? []);
      const preferredDefault = template?.default_model ?? "";
      const defaultModel =
        preferredDefault && nextModels.some((item) => item.id === preferredDefault)
          ? preferredDefault
          : nextModels[0]?.id ?? "";
      const nextModelState = applyModelState(nextModels, defaultModel);
      setForm((current) =>
        applyTemplateToDraft(
          current,
          kind,
          template,
          modelDiscovery,
          Boolean(options?.resetIdentity),
          nextModelState.models,
          nextModelState.defaultModel,
        ));
    },
    [applyModelState, modelEndpointTemplates, providerContributions],
  );

  useEffect(() => {
    draftInitializedRef.current = false;
    setForm(EMPTY_CHANNEL);
    setModelEntries([]);
    setDefaultModelKey(null);
    setSelectedModelKeys([]);
    setDiscoveredModels([]);
    setDiscoveryOpen(false);
    setDiscoveryQuery("");
    setShowUnaddedOnly(false);
    setSelectedDiscoveredIds([]);
    setDiscoveryPage(1);
    setDiscoveryPageSize(DEFAULT_DISCOVERY_PAGE_SIZE);
  }, [modelEndpointId]);

  useEffect(() => {
    setDiscoveryPage((current) => Math.min(current, discoveryTotalPages));
  }, [discoveryTotalPages]);

  useEffect(() => {
    if (draftInitializedRef.current) {
      return;
    }

    let cancelled = false;
    async function hydrate() {
      setError(null);
      setSuccess(null);
      try {
        const next = await listModelEndpoints();
        if (cancelled) {
          return;
        }
        setModelEndpointTemplates(next);

        if (isNew) {
          const defaultKind = interfaceTypes[0]?.[0] ?? "";
          if (!defaultKind) {
            return;
          }
          applyInterfaceDefaults(defaultKind, { resetIdentity: true, templates: next });
          draftInitializedRef.current = true;
          return;
        }

        const current = next.find((item) => item.id === modelEndpointId);
        if (current) {
          const normalized = applyModelState(current.available_models, current.default_model);
          setForm({
            ...current,
            available_models: normalized.models,
            default_model: normalized.defaultModel,
          });
          draftInitializedRef.current = true;
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    }

    void hydrate();
    return () => {
      cancelled = true;
    };
  }, [applyInterfaceDefaults, applyModelState, interfaceTypes, isNew, modelEndpointId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    setSuccess(null);

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
          t("web.model_endpoints.interface_type_required", "请先选择一个可用的接口类型。"),
        );
      }

      const saved = isNew
        ? await createModelEndpoint(payload)
        : await updateModelEndpoint(modelEndpointId, payload);

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
              kind: "model-endpoint",
              entityId: saved.id,
              title: nextTitle,
              subtitle: saved.kind,
              openedAt: Date.now(),
            },
          },
        });
        updateViewDescriptor(panelId, {
          kind: "model-endpoint",
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
      setSuccess(t("web.model_endpoints.save_success", "模型接入已保存。"));
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
    setSuccess(null);
    try {
      await deleteModelEndpoint(form.id);
      notifyProvidersChanged();
      if (panelId) {
        closeView(panelId);
        return;
      }
      setForm(EMPTY_CHANNEL);
      setModelEntries([]);
      setDefaultModelKey(null);
      setSelectedModelKeys([]);
      draftInitializedRef.current = false;
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDiscoverModels() {
    setModelsBusy(true);
    setError(null);
    setSuccess(null);
    try {
      const response = await discoverModelEndpointModels({
        ...form,
        available_models: serializeModelEntries(modelEntries),
      });
      const nextModels = normalizeModelDescriptors(response.models);
      setDiscoveredModels(nextModels);
      setSelectedDiscoveredIds([]);
      setDiscoveryOpen(true);
      setShowUnaddedOnly(nextModels.some((model) => !configuredModelIds.has(model.id)));
      setDiscoveryPage(1);
      if (nextModels.length === 0) {
        setSuccess(t("web.model_endpoints.discover_empty", "上游没有返回可导入的模型。"));
      } else {
        setSuccess(
          t("web.model_endpoints.discover_success", "已获取 {count} 个上游模型候选。").replace(
            "{count}",
            String(nextModels.length),
          ),
        );
      }
    } catch (err) {
      setError(String(err));
    } finally {
      setModelsBusy(false);
    }
  }

  function handleDiscoverySelectionChange(modelId: string, checked: boolean) {
    setSelectedDiscoveredIds((current) => {
      if (checked) {
        return current.includes(modelId) ? current : [...current, modelId];
      }
      return current.filter((item) => item !== modelId);
    });
  }

  function handleSelectVisibleDiscovered() {
    setSelectedDiscoveredIds((current) => {
      const next = new Set(current);
      for (const modelId of visibleSelectableDiscoveredIds) {
        next.add(modelId);
      }
      return Array.from(next);
    });
  }

  function handleClearDiscoveredSelection() {
    setSelectedDiscoveredIds([]);
  }

  function handleImportSelectedModels() {
    if (selectedImportableModels.length === 0) {
      return;
    }

    const nextEntries = [...modelEntries, ...selectedImportableModels.map((model) => createModelEntry(model))];
    const nextDefaultKey = defaultModelKey ?? nextEntries[0]?.key ?? null;

    setModelEntries(nextEntries);
    setDefaultModelKey(nextDefaultKey);
    setSelectedDiscoveredIds((current) => current.filter((id) => !selectedImportableModels.some((model) => model.id === id)));
    syncFormModels(nextEntries, nextDefaultKey);
    setSuccess(
      t("web.model_endpoints.import_success", "已导入 {count} 个模型到当前配置。").replace(
        "{count}",
        String(selectedImportableModels.length),
      ),
    );
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
    setSelectedModelKeys((current) => current.filter((item) => item !== key));
    syncFormModels(nextEntries, nextDefaultKey);
  }

  function handleDefaultModelSelect(key: string) {
    setDefaultModelKey(key);
    syncFormModels(modelEntries, key);
  }

  function handleModelSelectionChange(key: string, checked: boolean) {
    setSelectedModelKeys((current) => {
      if (checked) {
        return current.includes(key) ? current : [...current, key];
      }
      return current.filter((item) => item !== key);
    });
  }

  function handleRemoveSelectedModels() {
    if (selectedModelKeys.length === 0) {
      return;
    }

    const selectedKeySet = new Set(selectedModelKeys);
    const nextEntries = modelEntries.filter((entry) => !selectedKeySet.has(entry.key));
    const nextDefaultKey =
      defaultModelKey && selectedKeySet.has(defaultModelKey)
        ? (nextEntries[0]?.key ?? null)
        : defaultModelKey;

    setModelEntries(nextEntries);
    setDefaultModelKey(nextDefaultKey);
    setSelectedModelKeys([]);
    syncFormModels(nextEntries, nextDefaultKey);
  }

  return (
    <form className="resource-editor" onSubmit={handleSubmit}>
      <StatusNotice message={error} tone="error" onDismiss={() => setError(null)} />
      <StatusNotice message={success} tone="success" onDismiss={() => setSuccess(null)} />
      <div className="resource-editor__header">
        <div>
          <span className="resource-editor__eyebrow">
            {t("web.model_endpoints.eyebrow", "模型接入")}
          </span>
          <h2>{isNew ? t("web.model_endpoints.new", "新建模型接入") : form.display_name || form.id}</h2>
          <p>
            {t(
              "web.model_endpoints.editor_description",
              "一个模型接入就是一个可绑定给 Agent 的实际访问入口；模型提供方只表示已安装实现，不展示实现清单。",
            )}
          </p>
        </div>
      </div>
      <div className="resource-editor__scroll">
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
            {t("web.model_endpoints.display_name", "显示名")}
            <input
              value={form.display_name}
              onChange={(event) => setForm({ ...form, display_name: event.target.value })}
              required
            />
          </label>
          <label>
            {t("web.model_endpoints.interface_type", "接口类型")}
            <select
              value={form.kind}
              onChange={(event) => applyInterfaceDefaults(event.target.value)}
              disabled={!isNew}
            >
              {interfaceTypes.length === 0 ? (
                <option value="">
                  {t("web.model_endpoints.interface_type_empty", "当前没有可用接口类型")}
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
                "web.model_endpoints.interface_type_help",
                "这里只能选择当前已经装配完成的模型提供方；扩展装入后会自动出现在这里。",
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
          {t("web.model_endpoints.base_url", "Base URL")}
          <input
            value={form.base_url}
            onChange={(event) => setForm({ ...form, base_url: event.target.value })}
          />
        </label>
        <label>
          {t("web.model_endpoints.api_key", "API Key")}
          <input
            type="password"
            value={form.api_key}
            onChange={(event) => setForm({ ...form, api_key: event.target.value })}
            placeholder={t("web.model_endpoints.api_key_placeholder", "直接填写 API Key")}
          />
          <p className="helper-text">
            {t("web.model_endpoints.api_key_help", "可直接保存并使用 API Key；如果已填写这里，会优先于环境变量。")}
          </p>
        </label>
        <label>
          {t("web.model_endpoints.api_key_env", "API Key 环境变量")}
          <input
            value={form.api_key_env}
            onChange={(event) => setForm({ ...form, api_key_env: event.target.value })}
          />
          <p className="helper-text">
            {form.kind === "openai"
              ? t("web.model_endpoints.api_key_env_help_openai", "OpenAI 默认读取 OPENAI_API_KEY；这里填写服务进程里的环境变量名。")
              : t("web.model_endpoints.api_key_env_help", "这里填写服务进程里的环境变量名。")}
          </p>
        </label>
        <div className="form-grid">
          <label>
            {t("web.model_endpoints.default_model", "默认模型")}
            <input
              value={form.default_model}
              required={form.enabled}
              readOnly
            />
            <p className="helper-text">
              {selectedContribution?.provider.model_discovery
                ? t("web.model_endpoints.model_discovery_extension", "当前接口实现可以返回模型列表；保存时仍以这里的默认模型为准。")
                : t("web.model_endpoints.model_discovery_manual", "当前接口没有模型发现能力，请手动维护模型列表和默认模型。")}
            </p>
          </label>
          <div className="stack">
            <div className="model-toolbar">
              <div>
                <div className="panel-title">{t("web.model_endpoints.models", "模型列表")}</div>
                <p className="helper-text">
                  {t(
                    "web.model_endpoints.models_help",
                    "先从候选池挑需要的模型，再维护已选模型的预算与默认项；保存前会自动去重并清理空项。",
                  )}
                </p>
              </div>
              <div className="button-row button-row--wrap">
                {selectedContribution?.provider.model_discovery ? (
                  <button
                    type="button"
                    className="secondary"
                    disabled={modelsBusy || busy || !(form.kind ?? "").trim()}
                    onClick={() => void handleDiscoverModels()}
                  >
                    {modelsBusy
                      ? t("web.model_endpoints.refresh_models_loading", "正在获取上游模型…")
                      : t("web.model_endpoints.refresh_models", "获取上游模型")}
                  </button>
                ) : null}
                {discoveredModels.length > 0 ? (
                  <button
                    type="button"
                    className="secondary"
                    onClick={() => setDiscoveryOpen((current) => !current)}
                  >
                    {discoveryOpen
                      ? t("web.model_endpoints.discovery_hide", "收起候选池")
                      : t("web.model_endpoints.discovery_show", "查看候选池")}
                  </button>
                ) : null}
                <button type="button" className="secondary" onClick={handleModelAdd}>
                  {t("web.model_endpoints.model_add", "手动新增模型")}
                </button>
                <button
                  type="button"
                  className="secondary"
                  disabled={selectedModelKeys.length === 0}
                  onClick={handleRemoveSelectedModels}
                >
                  {t("web.model_endpoints.remove_selected", "删除选中模型")}
                </button>
              </div>
            </div>
            {discoveryOpen ? (
              <section className="details-panel model-discovery-panel">
                <div className="model-discovery-panel__header">
                  <div>
                    <div className="panel-title">{t("web.model_endpoints.discovery_title", "上游模型候选池")}</div>
                    <p className="helper-text">
                      {t(
                        "web.model_endpoints.discovery_help",
                        "这里只展示本次从上游拿到的候选模型；只有你导入的条目才会进入当前配置并参与保存。",
                      )}
                    </p>
                  </div>
                  <span className="model-discovery-panel__count">
                    {t("web.model_endpoints.discovery_count", "候选 {count}").replace(
                      "{count}",
                      String(filteredDiscoveredModels.length),
                    )}
                  </span>
                </div>
                <div className="model-discovery__controls">
                  <input
                    value={discoveryQuery}
                    placeholder={t("web.model_endpoints.discovery_search", "搜索模型 ID")}
                    onChange={(event) => {
                      setDiscoveryQuery(event.target.value);
                      setDiscoveryPage(1);
                    }}
                  />
                  <label className="check-row model-discovery__toggle">
                    <input
                      type="checkbox"
                      checked={showUnaddedOnly}
                      onChange={(event) => {
                        setShowUnaddedOnly(event.target.checked);
                        setDiscoveryPage(1);
                      }}
                    />
                    {t("web.model_endpoints.discovery_unadded_only", "只看未添加模型")}
                  </label>
                  <label className="model-discovery__page-size">
                    <span>{t("web.model_endpoints.discovery_page_size", "每页")}</span>
                    <select
                      value={String(discoveryPageSize)}
                      onChange={(event) => {
                        setDiscoveryPageSize(Number(event.target.value));
                        setDiscoveryPage(1);
                      }}
                    >
                      {DISCOVERY_PAGE_SIZE_OPTIONS.map((size) => (
                        <option key={size} value={size}>
                          {t("web.model_endpoints.discovery_page_size_option", "{count} 条").replace(
                            "{count}",
                            String(size),
                          )}
                        </option>
                      ))}
                    </select>
                  </label>
                </div>
                <div className="model-bulk-actions">
                  <span className="helper-text">
                    {t("web.model_endpoints.discovery_selected", "已选 {count}").replace(
                      "{count}",
                      String(selectedImportableModels.length),
                    )}
                  </span>
                  <div className="button-row button-row--wrap">
                    <button
                      type="button"
                      className="secondary"
                      disabled={visibleSelectableDiscoveredIds.length === 0}
                      onClick={handleSelectVisibleDiscovered}
                    >
                      {t("web.model_endpoints.discovery_select_visible", "选择当前页")}
                    </button>
                    <button
                      type="button"
                      className="secondary"
                      disabled={selectedDiscoveredIds.length === 0}
                      onClick={handleClearDiscoveredSelection}
                    >
                      {t("web.model_endpoints.discovery_clear_selection", "清空候选选择")}
                    </button>
                    <button
                      type="button"
                      disabled={selectedImportableModels.length === 0}
                      onClick={handleImportSelectedModels}
                    >
                      {t("web.model_endpoints.discovery_import", "导入选中模型")}
                    </button>
                  </div>
                </div>
                <div className="model-discovery-list">
                  {pagedDiscoveredModels.length > 0 ? (
                    pagedDiscoveredModels.map((model) => {
                      const isAdded = configuredModelIds.has(model.id);
                      return (
                        <label
                          key={model.id}
                          className={`model-discovery-item ${isAdded ? "model-discovery-item--added" : ""}`}
                        >
                          <input
                            type="checkbox"
                            checked={selectedDiscoveredIdSet.has(model.id)}
                            disabled={isAdded}
                            onChange={(event) => handleDiscoverySelectionChange(model.id, event.target.checked)}
                          />
                          <div className="model-discovery-item__body">
                            <div className="model-discovery-item__title">
                              <strong>{model.id}</strong>
                              <span className={`badge ${isAdded ? "badge--muted" : ""}`}>
                                {isAdded
                                  ? t("web.model_endpoints.discovery_added", "已添加")
                                  : t("web.model_endpoints.discovery_available", "可导入")}
                              </span>
                            </div>
                            <div className="model-discovery-item__meta">
                              <span>
                                {t("web.model_endpoints.model_context_label", "总上下文")}
                                {": "}
                                {model.max_context_tokens ?? t("web.common.none", "无")}
                              </span>
                              <span>
                                {t("web.model_endpoints.model_input_label", "最大输入")}
                                {": "}
                                {model.max_input_tokens ?? t("web.common.none", "无")}
                              </span>
                            </div>
                          </div>
                        </label>
                      );
                    })
                  ) : (
                    <div className="empty-card">
                      {t("web.model_endpoints.discovery_empty_filtered", "当前筛选条件下没有可显示的候选模型。")}
                    </div>
                  )}
                </div>
                {filteredDiscoveredModels.length > 0 ? (
                  <div className="model-discovery-pagination">
                    <span className="helper-text">
                      {t("web.model_endpoints.discovery_pagination_summary", "显示 {start}-{end} / {total}").replace(
                        "{start}",
                        String(discoveryPageStart),
                      ).replace(
                        "{end}",
                        String(discoveryPageEnd),
                      ).replace(
                        "{total}",
                        String(filteredDiscoveredModels.length),
                      )}
                    </span>
                    <div className="button-row button-row--wrap">
                      <button
                        type="button"
                        className="secondary"
                        disabled={discoveryPage <= 1}
                        onClick={() => setDiscoveryPage((current) => Math.max(1, current - 1))}
                      >
                        {t("web.model_endpoints.discovery_prev_page", "上一页")}
                      </button>
                      <span className="model-discovery-pagination__current">
                        {t("web.model_endpoints.discovery_page_indicator", "第 {page} / {total} 页").replace(
                          "{page}",
                          String(discoveryPage),
                        ).replace(
                          "{total}",
                          String(discoveryTotalPages),
                        )}
                      </span>
                      <button
                        type="button"
                        className="secondary"
                        disabled={discoveryPage >= discoveryTotalPages}
                        onClick={() => setDiscoveryPage((current) => Math.min(discoveryTotalPages, current + 1))}
                      >
                        {t("web.model_endpoints.discovery_next_page", "下一页")}
                      </button>
                    </div>
                  </div>
                ) : null}
              </section>
            ) : null}
            <div className="model-list">
              {modelEntries.length > 0 ? (
                modelEntries.map((entry) => (
                  <div key={entry.key} className="model-row">
                    <label className="model-row__check">
                      <input
                        type="checkbox"
                        checked={selectedModelKeySet.has(entry.key)}
                        onChange={(event) => handleModelSelectionChange(entry.key, event.target.checked)}
                      />
                    </label>
                    <input
                      value={entry.id}
                      placeholder={t("web.model_endpoints.model_placeholder", "模型 ID，例如 gpt-5.4")}
                      onChange={(event) => handleModelChange(entry.key, event.target.value)}
                    />
                    <input
                      type="number"
                      min="1"
                      value={entry.maxContextTokens}
                      placeholder={t("web.model_endpoints.model_context_placeholder", "总上下文大小")}
                      onChange={(event) =>
                        handleModelBudgetChange(entry.key, "maxContextTokens", event.target.value)
                      }
                    />
                    <input
                      type="number"
                      min="1"
                      value={entry.maxInputTokens}
                      placeholder={t("web.model_endpoints.model_input_placeholder", "最大输入上限")}
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
                        ? t("web.model_endpoints.model_is_default", "默认模型")
                        : t("web.model_endpoints.model_set_default", "设为默认")}
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
                  {t("web.model_endpoints.models_empty", "还没有模型。先获取上游模型候选，或手动新增一项。")}
                </div>
              )}
            </div>
          </div>
        </div>
        <label>
          {t("web.model_endpoints.description_field", "描述")}
          <textarea
            value={form.description}
            onChange={(event) => setForm({ ...form, description: event.target.value })}
            rows={4}
          />
        </label>
      </div>
      <div className="button-row resource-editor__footer">
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
