import type { AgentProfile, ChatMessage } from "@ennoia/api-client";

import type {
  ChatEntryRecipient,
  ChatEntryViewModel,
  LocalMessageDraft,
} from "./chat-types";

type StatusTexts = {
  typingLabel: string;
  typingDetail: string;
};

function detectMessageFormat(body: string) {
  const trimmed = body.trim();
  if (trimmed.startsWith("```mermaid") && trimmed.endsWith("```")) {
    return "diagram" as const;
  }
  if (trimmed.startsWith("```") && trimmed.endsWith("```")) {
    return "code" as const;
  }
  if ((trimmed.startsWith("{") && trimmed.endsWith("}")) || (trimmed.startsWith("[") && trimmed.endsWith("]"))) {
    try {
      JSON.parse(trimmed);
      return "json" as const;
    } catch {
      return "markdown" as const;
    }
  }
  return "markdown" as const;
}

function isLikelyErrorMessage(role: ChatMessage["role"], body: string) {
  if (role === "operator") {
    return false;
  }
  const normalized = body.trim().toLowerCase();
  if (!normalized) {
    return false;
  }
  return normalized.startsWith("error:")
    || normalized.startsWith("exception:")
    || normalized.startsWith("panic:")
    || normalized.includes(" request failed:")
    || normalized.endsWith(" failed")
    || normalized.includes(" upstream call failed")
    || normalized.includes(" provider returned empty");
}

function summarizeError(message: string) {
  const lines = message
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines[0] ?? message.trim();
}

function createErrorDetail(message: string) {
  const trimmed = message.trim();
  const lines = trimmed
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  if (lines.length <= 1) {
    return undefined;
  }
  return trimmed;
}

export function buildChatEntries(params: {
  messages: ChatMessage[];
  localDrafts: LocalMessageDraft[];
  resolveRecipients: (mentions: string[]) => AgentProfile[];
}): ChatEntryViewModel[] {
  const entries: Array<{ order: number; entry: ChatEntryViewModel }> = [];
  let order = 0;

  for (const message of params.messages) {
    const recipients = params.resolveRecipients(message.mentions).map<ChatEntryRecipient>((agent) => ({
      id: agent.id,
      label: agent.display_name,
    }));
    const base = {
      id: message.id,
      role: message.role,
      sender: message.sender,
      body: message.body,
      createdAt: message.created_at,
      state: "done" as const,
      format: detectMessageFormat(message.body),
    };

    if (isLikelyErrorMessage(message.role, message.body)) {
      entries.push({
        order: order++,
        entry: {
          ...base,
          kind: "error",
          title: message.sender,
          summary: summarizeError(message.body),
          detail: createErrorDetail(message.body),
          tone: "danger",
        },
      });
      continue;
    }

    if (message.role === "system") {
      entries.push({
        order: order++,
        entry: {
          ...base,
          kind: "system",
          role: "system",
        },
      });
      continue;
    }

    if (message.role === "tool") {
      entries.push({
        order: order++,
        entry: {
          ...base,
          kind: "tool_result",
          role: "tool",
          title: message.sender,
        },
      });
      continue;
    }

    entries.push({
      order: order++,
      entry: {
        ...base,
        kind: "message",
        messageId: message.id,
        branchId: message.branch_id ?? message.lane_id ?? undefined,
        replyToMessageId: message.reply_to_message_id ?? undefined,
        rewriteFromMessageId: message.rewrite_from_message_id ?? undefined,
        recipients,
        mentions: message.mentions,
        source: "remote",
      },
    });
  }

  for (const draft of params.localDrafts) {
    const recipients = params.resolveRecipients(draft.addressedAgents).map<ChatEntryRecipient>((agent) => ({
      id: agent.id,
      label: agent.display_name,
    }));
    const messageEntry: ChatEntryViewModel = {
      id: draft.clientId,
      messageId: draft.clientId,
      role: "operator",
      kind: "message",
      format: detectMessageFormat(draft.body),
      state: draft.status === "failed" ? "failed" : draft.status === "sending" ? "streaming" : "pending",
      sender: "Operator",
      body: draft.body,
      createdAt: draft.createdAt,
      branchId: draft.branchId,
      replyToMessageId: draft.forkFromMessageId,
      rewriteFromMessageId: draft.rewriteFromMessageId,
      recipients,
      mentions: draft.explicitMentions ?? [],
      source: "local",
      localStatus: draft.status,
      localError: draft.error,
    };
    entries.push({ order: order++, entry: messageEntry });
  }

  return entries
    .sort((left, right) => {
      const time = left.entry.createdAt.localeCompare(right.entry.createdAt);
      if (time !== 0) {
        return time;
      }
      return left.order - right.order;
    })
    .map((item) => item.entry);
}

export function buildStatusEntries(params: {
  typingAgents: AgentProfile[];
  pendingCreatedAt?: string;
  texts: StatusTexts;
}): ChatEntryViewModel[] {
  const entries: ChatEntryViewModel[] = [];

  if (params.typingAgents.length > 0) {
    for (const agent of params.typingAgents) {
      entries.push({
        id: `typing:${agent.id}`,
        role: "agent",
        kind: "status",
        format: "plain",
        state: "streaming",
        sender: agent.display_name,
        title: params.texts.typingLabel,
        label: params.texts.typingLabel,
        detail: params.texts.typingDetail,
        animation: "typing",
        body: params.texts.typingDetail,
        createdAt: params.pendingCreatedAt ?? "",
      });
    }
  }

  return entries;
}
