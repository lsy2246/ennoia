import { fetchJson } from "./core";
import type {
  PermissionApprovalRecord,
  PermissionEventRecord,
  PermissionGrantRecord,
  PermissionPolicySummary,
} from "./types";

export async function listPermissionPolicySummaries() {
  return fetchJson<PermissionPolicySummary[]>("/api/permissions/policies");
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
  conversation_id?: string;
  status?: string;
  limit?: number;
}) {
  const params = new URLSearchParams();
  if (query?.agent_id) {
    params.set("agent_id", query.agent_id);
  }
  if (query?.conversation_id) {
    params.set("conversation_id", query.conversation_id);
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

export async function listConversationPermissionApprovals(
  conversationId: string,
  query?: {
    status?: string;
    limit?: number;
  },
) {
  return listPermissionApprovals({
    conversation_id: conversationId,
    status: query?.status,
    limit: query?.limit,
  });
}

export async function resolvePermissionApproval(
  approvalId: string,
  resolution: "allow_once" | "allow_reply_action" | "allow_conversation_all" | "deny",
) {
  return fetchJson<PermissionApprovalRecord>(
    `/api/permissions/approvals/${approvalId}/resolve`,
    {
      method: "POST",
      body: JSON.stringify({ resolution }),
    },
  );
}

export async function listPermissionGrants(query?: {
  agent_id?: string;
  conversation_id?: string;
  limit?: number;
}) {
  const params = new URLSearchParams();
  if (query?.agent_id) {
    params.set("agent_id", query.agent_id);
  }
  if (query?.conversation_id) {
    params.set("conversation_id", query.conversation_id);
  }
  if (typeof query?.limit === "number") {
    params.set("limit", String(query.limit));
  }
  const suffix = params.toString();
  return fetchJson<PermissionGrantRecord[]>(
    `/api/permissions/grants${suffix ? `?${suffix}` : ""}`,
  );
}

export async function listConversationPermissionGrants(
  conversationId: string,
  query?: {
    agent_id?: string;
    limit?: number;
  },
) {
  return listPermissionGrants({
    agent_id: query?.agent_id,
    conversation_id: conversationId,
    limit: query?.limit,
  });
}

export async function revokePermissionGrant(grantId: string) {
  return fetchJson<PermissionGrantRecord>(
    `/api/permissions/grants/${grantId}/revoke`,
    {
      method: "POST",
    },
  );
}
