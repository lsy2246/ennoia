import { fetchJson } from "./core";
import type {
  AgentPermissionPolicy,
  PermissionApprovalRecord,
  PermissionEventRecord,
  PermissionPolicySummary,
} from "./types";

export async function listPermissionPolicySummaries() {
  return fetchJson<PermissionPolicySummary[]>("/api/permissions/policies");
}

export async function getAgentPermissionPolicy(agentId: string) {
  return fetchJson<AgentPermissionPolicy>(`/api/agents/${agentId}/policy`);
}

export async function updateAgentPermissionPolicy(
  agentId: string,
  policy: AgentPermissionPolicy,
) {
  return fetchJson<AgentPermissionPolicy>(`/api/agents/${agentId}/policy`, {
    method: "PUT",
    body: JSON.stringify(policy),
  });
}

export async function listPermissionEvents(query?: {
  agent_id?: string;
  action?: string;
  decision?: string;
  limit?: number;
}) {
  const params = new URLSearchParams();
  if (query?.agent_id) {
    params.set("agent_id", query.agent_id);
  }
  if (query?.action) {
    params.set("action", query.action);
  }
  if (query?.decision) {
    params.set("decision", query.decision);
  }
  if (typeof query?.limit === "number") {
    params.set("limit", String(query.limit));
  }
  const suffix = params.toString();
  return fetchJson<PermissionEventRecord[]>(
    `/api/permissions/events${suffix ? `?${suffix}` : ""}`,
  );
}

export async function listPermissionApprovals(query?: {
  agent_id?: string;
  status?: string;
  limit?: number;
}) {
  const params = new URLSearchParams();
  if (query?.agent_id) {
    params.set("agent_id", query.agent_id);
  }
  if (query?.status) {
    params.set("status", query.status);
  }
  if (typeof query?.limit === "number") {
    params.set("limit", String(query.limit));
  }
  const suffix = params.toString();
  return fetchJson<PermissionApprovalRecord[]>(
    `/api/permissions/approvals${suffix ? `?${suffix}` : ""}`,
  );
}

export async function resolvePermissionApproval(
  approvalId: string,
  resolution: "allow_once" | "allow_conversation" | "allow_run" | "allow_policy" | "deny",
) {
  return fetchJson<PermissionApprovalRecord>(
    `/api/permissions/approvals/${approvalId}/resolve`,
    {
      method: "POST",
      body: JSON.stringify({ resolution }),
    },
  );
}
