import type { AgentProfile, ConversationMessage } from "@ennoia/api-client";

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

function isLikelyErrorMessage(role: ConversationMessage["role"], body: string) {
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
    || normalized.includes(" provider returned empty")
    || normalized.includes("native sandbox only accepts")
    || normalized.includes("path cannot escape the selected execution root")
    || normalized.includes("path must stay inside the selected execution root");
}

function summarizeError(message: string) {
  const lines = message
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines[0] ?? message.trim();
}

function detectFailureCode(message: string) {
  const normalized = message.trim().toLowerCase();
  if (
    normalized.includes("native sandbox only accepts /workspace, /artifacts and /tmp paths")
    || normalized.includes("native sandbox only accepts")
    || normalized.includes("path cannot escape the selected execution root")
    || normalized.includes("path must stay inside the selected execution root")
  ) {
    return "sandbox_path_restricted";
  }
  return undefined;
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

function parseToolPayload(body: string) {
  try {
    const parsed = JSON.parse(body) as unknown;
    if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
      return null;
    }
    return parsed as Record<string, unknown>;
  } catch {
    return null;
  }
}

function readToolName(body: string) {
  const payload = parseToolPayload(body);
  const tool = payload?.tool;
  return typeof tool === "string" && tool.trim().length > 0 ? tool.trim() : undefined;
}

function normalizeToolIdentifier(value: string | undefined) {
  return value?.trim().toLowerCase().replace(/[_\s]+/g, ".").replace(/\.+/g, ".");
}

function looksLikeToolSender(sender: string | undefined, parsedToolName: string | undefined) {
  const normalizedSender = normalizeToolIdentifier(sender);
  const normalizedParsedToolName = normalizeToolIdentifier(parsedToolName);

  if (!normalizedSender) {
    return false;
  }

  if (normalizedParsedToolName && normalizedSender === normalizedParsedToolName) {
    return true;
  }

  return ["fs.read", "fs.write", "command.exec", "net.fetch"].includes(normalizedSender);
}

function findLegacyToolActor(messages: ConversationMessage[], startIndex: number) {
  for (let index = startIndex + 1; index < messages.length; index += 1) {
    const candidate = messages[index];
    if (candidate.role === "operator") {
      break;
    }
    if (candidate.role === "agent" && candidate.sender.trim().length > 0) {
      return candidate.sender;
    }
  }

  for (let index = startIndex - 1; index >= 0; index -= 1) {
    const candidate = messages[index];
    if (candidate.role === "operator") {
      break;
    }
    if (candidate.role === "agent" && candidate.sender.trim().length > 0) {
      return candidate.sender;
    }
  }

  return undefined;
}

export function buildChatEntries(params: {
  messages: ConversationMessage[];
  localDrafts: LocalMessageDraft[];
  resolveRecipients: (mentions: string[]) => AgentProfile[];
}): ChatEntryViewModel[] {
  const entries: Array<{ order: number; entry: ChatEntryViewModel }> = [];
  const messageContextById = new Map(
    params.messages.map((message) => [
      message.id,
      {
        role: message.role,
        sender: message.sender,
      },
    ]),
  );
  let order = 0;

  for (const [messageIndex, message] of params.messages.entries()) {
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
      if (message.role === "agent") {
        entries.push({
          order: order++,
          entry: {
            ...base,
            kind: "message",
            state: "failed",
            messageId: message.id,
            branchId: message.branch_id ?? message.lane_id ?? undefined,
            parentMessageId: message.parent_message_id ?? undefined,
            replyToMessageId: message.reply_to_message_id ?? undefined,
            rewriteFromMessageId: message.rewrite_from_message_id ?? undefined,
            recipients,
            mentions: message.mentions,
            source: "remote",
            failureCode: detectFailureCode(message.body),
            failureSummary: summarizeError(message.body),
            failureDetail: message.body.trim(),
          },
        });
        continue;
      }

      entries.push({
        order: order++,
        entry: {
          ...base,
          kind: "error",
          title: message.sender,
          summary: summarizeError(message.body),
          detail: createErrorDetail(message.body),
          tone: "danger",
          relatedMessageId: message.parent_message_id ?? undefined,
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
          relatedMessageId: message.parent_message_id ?? undefined,
        },
      });
      continue;
    }

    if (message.role === "tool") {
      const parsedToolName = readToolName(message.body);
      const parentMessage = message.parent_message_id
        ? messageContextById.get(message.parent_message_id)
        : undefined;
      const senderValue = message.sender?.trim();
      const senderIsToolName = looksLikeToolSender(senderValue, parsedToolName);
      const legacyActorSender = senderIsToolName
        ? findLegacyToolActor(params.messages, messageIndex)
        : undefined;
      entries.push({
        order: order++,
        entry: {
          ...base,
          kind: "tool_result",
          role: "tool",
          title: parsedToolName ?? message.sender,
          relatedMessageId: message.parent_message_id ?? undefined,
          actorSender: senderIsToolName
            ? (parentMessage?.role === "agent" ? parentMessage.sender : legacyActorSender)
            : message.sender,
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
        parentMessageId: message.parent_message_id ?? undefined,
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
