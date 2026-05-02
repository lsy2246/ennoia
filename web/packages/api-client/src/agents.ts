import { fetchJson } from "./core";
import type {
  AgentProfile,
  ModelEndpointConfig,
  ModelEndpointModelsResponse,
  SkillConfig,
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
  const response = await fetchJson<ModelEndpointModelsResponse>(
    `${MODEL_ENDPOINTS_API}/${modelEndpointId}/models`,
  );
  return normalizeModelEndpointModelsResponse(response, modelEndpointId);
}

export async function discoverModelEndpointModels(payload: ModelEndpointConfig) {
  const response = await fetchJson<ModelEndpointModelsResponse>(
    `${MODEL_ENDPOINTS_API}/discover-models`,
    {
      method: "POST",
      body: JSON.stringify(payload),
    },
  );
  return normalizeModelEndpointModelsResponse(response, payload.id);
}
