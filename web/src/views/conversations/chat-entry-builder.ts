import type { AgentProfile, ConversationMessage, OperationRecord } from "@ennoia/api-client";

import type {
  ChatEntryRecipient,
  ChatEntryViewModel,
  LocalMessageDraft,
} from "./chat-types";
import { classifyConversationFailure, isLikelyFailureMessage } from "./error-classification";

type StatusTexts = {
  typingLabel: string;
  typingDetail: string;
};

function describeOperationStep(operation: OperationRecord) {
  if (operation.kind === "provider" && operation.name === "generate") {
    return "正在生成回复。";
  }
  if (operation.kind === "runtime" && operation.name === "command.exec") {
    return "正在执行命令。";
  }
  if (operation.kind === "runtime" && operation.name === "fs.write") {
    return "正在写入文件。";
  }
  if (operation.kind === "runtime" && operation.name === "fs.read") {
    return "正在读取文件。";
  }
  if (operation.kind === "runtime" && operation.name === "net.fetch") {
    return "正在请求网络资源。";
  }
  return undefined;
}

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
  if (role === "tool") {
    const payload = parseToolPayload(body);
    if (payload?.kind === "ennoia.tool_call") {
      return false;
    }
  }
  return isLikelyFailureMessage(body);
}

function summarizeError(message: string) {
  const lines = message
    .split(/\r?\n/)
    .map((line) => line.trim())
    .filter(Boolean);
  return lines[0] ?? message.trim();
}

function detectFailureCode(message: string) {
  return classifyConversationFailure(message)?.code;
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

function readReasoningContent(body: string) {
  const payload = parseToolPayload(body);
  if (payload?.kind !== "ennoia.reasoning") {
    return null;
  }
  const content = typeof payload.content === "string" ? payload.content.trim() : "";
  if (!content) {
    return null;
  }
  return {
    content,
    format: typeof payload.format === "string" ? payload.format.trim() : "",
  };
}

function readToolName(body: string) {
  const payload = parseToolPayload(body);
  const tool = payload?.tool_name ?? payload?.tool;
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
  records?: Array<import("@ennoia/api-client").ExtensionRecordEntry>;
  localDrafts: LocalMessageDraft[];
  operatorDisplayName?: string;
  resolveRecipients: (mentions: string[]) => AgentProfile[];
}): ChatEntryViewModel[] {
  const operatorDisplayName = params.operatorDisplayName?.trim() || "Operator";
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
    if (message.role === "operator" && (!message.sender.trim() || message.sender === "operator")) {
      base.sender = operatorDisplayName;
    }

    if (isLikelyErrorMessage(message.role, message.body)) {
      if (message.role === "agent") {
        const failure = classifyConversationFailure(message.body);
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
            failureCode: failure?.code ?? detectFailureCode(message.body),
            failureSource: failure?.source,
            failureSummary: failure?.summary ?? summarizeError(message.body),
            failureDetail: failure?.detail ?? message.body.trim(),
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
      const reasoning = readReasoningContent(message.body);
      if (reasoning) {
        entries.push({
          order: order++,
          entry: {
            ...base,
            kind: "reasoning",
            role: "agent",
            sender: message.sender,
            body: reasoning.content,
            format: reasoning.format === "plain" ? "plain" : detectMessageFormat(reasoning.content),
            relatedMessageId: message.parent_message_id ?? undefined,
            actorSender: message.sender,
          },
        });
        continue;
      }

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

  for (const record of params.records ?? []) {
    entries.push({
      order: order++,
      entry: {
        id: `record:${record.id}`,
        role: "agent",
        kind: "record",
        format: "plain",
        state: record.closed_at ? "done" : "streaming",
        sender: record.extension_id,
        body: record.summary?.trim() || record.title?.trim() || record.kind,
        createdAt: record.created_at,
        relatedMessageId: record.related_message_id ?? undefined,
        record,
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
      sender: operatorDisplayName,
      body: draft.body,
      createdAt: draft.createdAt,
      branchId: draft.branchId,
      replyToMessageId: draft.forkFromMessageId,
      rewriteFromMessageId: draft.rewriteFromMessageId,
      recipients,
      mentions: draft.explicitMentions ?? [],
      source: "local",
      localStatus: draft.status,
      dispatchMode: draft.dispatchMode,
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
  operations: OperationRecord[];
  resolveAgent: (agentId: string) => AgentProfile | undefined;
  texts: StatusTexts;
}): ChatEntryViewModel[] {
  const entries: ChatEntryViewModel[] = [];

  for (const operation of params.operations) {
    if (!["queued", "running"].includes(operation.status)) {
      continue;
    }
    const agent = params.resolveAgent(operation.agent_id);
    entries.push({
      id: `status:${operation.id}`,
      role: "agent",
      kind: "status",
      format: "plain",
      state: "streaming",
      sender: agent?.display_name ?? operation.agent_id,
      title: params.texts.typingLabel,
      label: params.texts.typingLabel,
      branchId: operation.branch_id ?? operation.lane_id ?? undefined,
      detail: describeOperationStep(operation) ?? params.texts.typingDetail,
      animation: "typing",
      body: params.texts.typingDetail,
      createdAt: operation.updated_at,
      sourceMessageId: operation.message_id ?? undefined,
      live: true,
      operationId: operation.id,
    });
  }

  return entries;
}
