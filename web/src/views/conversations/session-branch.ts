import type { ConversationBranch } from "@ennoia/api-client";

function normalizeConversationBranchId(...candidates: Array<string | null | undefined>) {
  for (const candidate of candidates) {
    const normalized = typeof candidate === "string" ? candidate.trim() : "";
    if (normalized) {
      return normalized;
    }
  }
  return undefined;
}

export function resolveActiveConversationBranch(params: {
  conversationActiveBranchId?: string | null;
  branches: ConversationBranch[];
}) {
  const activeBranchId = normalizeConversationBranchId(params.conversationActiveBranchId);
  const branch = activeBranchId
    ? params.branches.find((item) => item.id === activeBranchId) ?? null
    : null;
  return {
    branch: branch ?? params.branches[0] ?? null,
    branchId: activeBranchId ?? normalizeConversationBranchId(branch?.id, params.branches[0]?.id),
  };
}
