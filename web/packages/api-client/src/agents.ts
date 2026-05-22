import { fetchJson } from "./core";
import { invokeProviderMethod } from "./extensions";
import type {
  AgentProfile,
  ModelEndpointConfig,
  ModelEndpointModelsResponse,
  SkillCheckResult,
  SkillConfig,
  SkillSettingsResponse,
} from "./types";

const MODEL_ENDPOINTS_API = "/api/model-endpoints";

function normalizeModelEndpointModelsResponse(
  response: ModelEndpointModelsResponse,
  fallbackId: string,
): ModelEndpointModelsResponse {
  return {
    ...response,
    model_endpoint_id: response.model_endpoint_id || fallbackId,
  };
}

export async function listAgents() {
  return fetchJson<AgentProfile[]>("/api/agents");
}

export async function getAgent(agentId: string) {
  return fetchJson<AgentProfile>(`/api/agents/${agentId}`);
}

export async function createAgent(payload: AgentProfile) {
  return fetchJson<AgentProfile>("/api/agents", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateAgent(agentId: string, payload: AgentProfile) {
  return fetchJson<AgentProfile>(`/api/agents/${agentId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteAgent(agentId: string) {
  return fetchJson<void>(`/api/agents/${agentId}`, { method: "DELETE" });
}

export async function listSkills() {
  return fetchJson<SkillConfig[]>("/api/skills");
}

export async function getSkill(skillId: string) {
  return fetchJson<SkillConfig>(`/api/skills/${skillId}`);
}

export async function createSkill(payload: SkillConfig) {
  return fetchJson<SkillConfig>("/api/skills", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateSkill(skillId: string, payload: SkillConfig) {
  return fetchJson<SkillConfig>(`/api/skills/${skillId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteSkill(skillId: string) {
  return fetchJson<void>(`/api/skills/${skillId}`, { method: "DELETE" });
}

export async function getSkillSettings(skillId: string) {
  return fetchJson<SkillSettingsResponse>(`/api/skills/${skillId}/config`);
}

export async function saveSkillSettings(
  skillId: string,
  values: Record<string, string | number | boolean>,
) {
  return fetchJson<SkillSettingsResponse>(`/api/skills/${skillId}/config`, {
    method: "PUT",
    body: JSON.stringify({ values }),
  });
}

export async function getSkillStatus(skillId: string) {
  return fetchJson<SkillCheckResult>(`/api/skills/${skillId}/status`);
}

export async function runSkillCheck(skillId: string) {
  return fetchJson<SkillCheckResult>(`/api/skills/${skillId}/check`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function runSkillPrepare(skillId: string) {
  return fetchJson<SkillCheckResult>(`/api/skills/${skillId}/prepare`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function listModelEndpoints() {
  return fetchJson<ModelEndpointConfig[]>(MODEL_ENDPOINTS_API);
}

export async function createModelEndpoint(payload: ModelEndpointConfig) {
  return fetchJson<ModelEndpointConfig>(MODEL_ENDPOINTS_API, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateModelEndpoint(modelEndpointId: string, payload: ModelEndpointConfig) {
  return fetchJson<ModelEndpointConfig>(`${MODEL_ENDPOINTS_API}/${modelEndpointId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteModelEndpoint(modelEndpointId: string) {
  return fetchJson<void>(`${MODEL_ENDPOINTS_API}/${modelEndpointId}`, { method: "DELETE" });
}

export async function getModelEndpointModels(modelEndpointId: string) {
  const modelEndpoint = await fetchJson<ModelEndpointConfig>(
    `${MODEL_ENDPOINTS_API}/${modelEndpointId}`,
  );
  return discoverModelEndpointModels(modelEndpoint);
}

export async function discoverModelEndpointModels(payload: ModelEndpointConfig) {
  const response = await invokeProviderMethod<ModelEndpointModelsResponse>(
    payload.kind,
    "list_models",
    {
      model_endpoint: payload,
    },
  );
  return normalizeModelEndpointModelsResponse(response, payload.id);
}
