import type {
  AgentFileAccessProfile,
  AgentPermissionProfile,
  AgentProfile,
  ModelEndpointConfig,
  ProviderModelDescriptor,
} from "@ennoia/api-client";

export const DEFAULT_FILE_ACCESS_PROFILE: AgentFileAccessProfile = {
  default_root: "/workspace",
  roots: [
    { id: "workspace", path: "/workspace", mode: "read_write" },
    { id: "artifacts", path: "/artifacts", mode: "read_write" },
    { id: "temp", path: "/tmp", mode: "read_write" },
  ],
};

export const EMPTY_AGENT: AgentProfile = {
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

export const EMPTY_PERMISSION_PROFILE: AgentPermissionProfile = {
  mode: "whitelist",
  entries: [],
};

export type AgentDraftInitializationState = {
  agentId: string;
  initializedAgentId: string | null;
};

type AgentDraftModelEndpoint = Pick<
  ModelEndpointConfig,
  "id" | "kind" | "default_model" | "available_models"
>;

type AgentDraftProviderContribution = {
  provider: {
    kind: string;
    generation_options?: Array<{
      id: string;
      default_value?: string | null;
    }>;
  };
};

export function shouldInitializeAgentDraft(state: AgentDraftInitializationState) {
  return state.agentId !== state.initializedAgentId;
}

export function createDefaultAgentDraft(
  modelEndpoints: AgentDraftModelEndpoint[],
  providerContributions: AgentDraftProviderContribution[],
): AgentProfile {
  const defaultEndpoint = modelEndpoints[0] ?? null;
  return {
    ...EMPTY_AGENT,
    permission_profile: { ...EMPTY_PERMISSION_PROFILE, entries: [] },
    file_access: {
      ...DEFAULT_FILE_ACCESS_PROFILE,
      roots: DEFAULT_FILE_ACCESS_PROFILE.roots.map((root) => ({ ...root })),
    },
    model_endpoint_id: defaultEndpoint?.id ?? "",
    model_id: resolveAgentModelId(defaultEndpoint, ""),
    generation_options: defaultGenerationOptions(
      findProviderContribution(providerContributions, defaultEndpoint),
    ),
  };
}

export function findProviderContribution<T extends AgentDraftProviderContribution>(
  contributions: T[],
  provider: AgentDraftModelEndpoint | null,
) {
  if (!provider) {
    return null;
  }

  const matches = contributions.filter((item) => item.provider.kind === provider.kind);
  return matches.length === 1 ? matches[0] : null;
}

export function resolveAgentModelId(
  provider: AgentDraftModelEndpoint | null,
  currentModelId: string,
) {
  const normalizedCurrentModelId = currentModelId.trim();
  const models: ProviderModelDescriptor[] = provider?.available_models ?? [];
  if (normalizedCurrentModelId && models.some((model) => model.id === normalizedCurrentModelId)) {
    return normalizedCurrentModelId;
  }
  if (provider?.default_model?.trim()) {
    return provider.default_model.trim();
  }
  return models[0]?.id ?? normalizedCurrentModelId;
}

export function defaultGenerationOptions(
  contribution: AgentDraftProviderContribution | null,
) {
  return Object.fromEntries(
    (contribution?.provider.generation_options ?? [])
      .filter((option) => option.default_value)
      .map((option) => [option.id, option.default_value!]),
  );
}
