import { fetchJson } from "./core";
import type { AgentProfile, ProviderConfig, ProviderModelsResponse, SkillConfig } from "./types";

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

export async function listProviders() {
  return fetchJson<ProviderConfig[]>("/api/providers");
}

export async function createProvider(payload: ProviderConfig) {
  return fetchJson<ProviderConfig>("/api/providers", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateProvider(providerId: string, payload: ProviderConfig) {
  return fetchJson<ProviderConfig>(`/api/providers/${providerId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteProvider(providerId: string) {
  return fetchJson<void>(`/api/providers/${providerId}`, { method: "DELETE" });
}

export async function getProviderModels(providerId: string) {
  return fetchJson<ProviderModelsResponse>(`/api/providers/${providerId}/models`);
}

export async function discoverProviderModels(payload: ProviderConfig) {
  return fetchJson<ProviderModelsResponse>("/api/providers/discover-models", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

