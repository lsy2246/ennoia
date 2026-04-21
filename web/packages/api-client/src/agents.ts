import { fetchJson } from "./core";
import type { AgentProfile, ProviderConfig, ProviderModelsResponse, SkillConfig } from "./types";

export async function listAgents() {
  return fetchJson<AgentProfile[]>("/api/v1/agents");
}

export async function getAgent(agentId: string) {
  return fetchJson<AgentProfile>(`/api/v1/agents/${agentId}`);
}

export async function createAgent(payload: AgentProfile) {
  return fetchJson<AgentProfile>("/api/v1/agents", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateAgent(agentId: string, payload: AgentProfile) {
  return fetchJson<AgentProfile>(`/api/v1/agents/${agentId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteAgent(agentId: string) {
  return fetchJson<void>(`/api/v1/agents/${agentId}`, { method: "DELETE" });
}

export async function listSkills() {
  return fetchJson<SkillConfig[]>("/api/v1/skills");
}

export async function createSkill(payload: SkillConfig) {
  return fetchJson<SkillConfig>("/api/v1/skills", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateSkill(skillId: string, payload: SkillConfig) {
  return fetchJson<SkillConfig>(`/api/v1/skills/${skillId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteSkill(skillId: string) {
  return fetchJson<void>(`/api/v1/skills/${skillId}`, { method: "DELETE" });
}

export async function listProviders() {
  return fetchJson<ProviderConfig[]>("/api/v1/providers");
}

export async function createProvider(payload: ProviderConfig) {
  return fetchJson<ProviderConfig>("/api/v1/providers", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateProvider(providerId: string, payload: ProviderConfig) {
  return fetchJson<ProviderConfig>(`/api/v1/providers/${providerId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteProvider(providerId: string) {
  return fetchJson<void>(`/api/v1/providers/${providerId}`, { method: "DELETE" });
}

export async function getProviderModels(providerId: string) {
  return fetchJson<ProviderModelsResponse>(`/api/v1/providers/${providerId}/models`);
}

