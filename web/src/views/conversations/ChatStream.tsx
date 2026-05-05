import type {
  AgentProfile,
  ExtensionRecordEntry,
  ExecutionRun,
  PermissionApprovalRecord,
  SkillConfig,
} from "@ennoia/api-client";
import { apiUrl } from "@ennoia/api-client";
import { Fragment, useEffect, useRef, useState } from "react";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { loadExtensionConversationRecordMount } from "@/views/extensions/registry";

import { ChatContent } from "./ChatContent";
import { classifyConversationFailure } from "./error-classification";
import type {
  ChatEntryViewModel,
  ChatErrorEntry,
  ChatRecordEntry,
  ChatReasoningEntry,
  ChatStatusEntry,
  ChatSystemEntry,
  ChatToolResultEntry,
  ConversationMessageEntry,
} from "./chat-types";

type ChatAccessoryEntry =
  | ChatErrorEntry
  | ChatStatusEntry
  | ChatReasoningEntry
  | ChatSystemEntry
  | ChatToolResultEntry
  | ChatRecordEntry;

type ChatGroup =
  | {
      id: string;
      order: number;
      anchor: ConversationMessageEntry;
      accessories: ChatAccessoryEntry[];
      relatedRuns: ExecutionRun[];
      timestamp: string;
    }
  | {
      id: string;
      order: number;
      anchor: null;
      accessories: ChatAccessoryEntry[];
      relatedRuns: ExecutionRun[];
      timestamp: string;
    };

type ChatMessageGroup = Extract<ChatGroup, { anchor: ConversationMessageEntry }>;
type ChatStandaloneGroup = Extract<ChatGroup, { anchor: null }>;

type ChatTurn = {
  id: string;
  operatorGroup: ChatMessageGroup;
  processGroups: ChatGroup[];
  finalReplyGroup?: ChatMessageGroup;
  timestamp: string;
};

type ChatStreamBlock =
  | {
      kind: "turn";
      id: string;
      timestamp: string;
      turn: ChatTurn;
    }
  | {
      kind: "standalone";
      id: string;
      timestamp: string;
      group: ChatStandaloneGroup;
    };

function resolveSenderLabel(params: {
  role: ChatEntryViewModel["role"];
  sender?: string;
  agents: AgentProfile[];
  t: (key: string, fallback: string) => string;
}) {
  const rawSender = params.sender?.trim();
  const normalizedSender = rawSender?.toLowerCase();
  const isInternalSender = normalizedSender
    ? ["operator", "user", "agent", "system"].includes(normalizedSender)
    : false;

  if (params.role === "operator") {
    return "";
  }

  if (params.role === "agent") {
    if (!rawSender) {
      return params.t("web.conversations.sender_agent", "Agent");
    }
    const matchedAgent = params.agents.find((agent) =>
      agent.id === rawSender || agent.display_name === rawSender);
    if (matchedAgent) {
      return matchedAgent.display_name;
    }
    if (normalizedSender === "operator") {
      return params.t("web.conversations.sender_agent", "Agent");
    }
    return rawSender;
  }

  if (params.role === "system") {
    return params.t("web.conversations.system_label", "系统消息");
  }

  if (isInternalSender) {
    return params.t("web.conversations.tool_label", "工具输出");
  }

  return rawSender || params.t("web.conversations.tool_label", "工具输出");
}

function pad(value: number) {
  return String(value).padStart(2, "0");
}

function formatAbsoluteDateTime(value: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) {
    return value;
  }
  return `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}:${pad(date.getSeconds())}`;
}

function dateKey(value: string) {
  return formatAbsoluteDateTime(value).slice(0, 10);
}

function accessoryRelatedMessageId(entry: ChatAccessoryEntry) {
  if ("relatedMessageId" in entry) {
    return entry.relatedMessageId;
  }
  return undefined;
}

function buildChatGroups(entries: ChatEntryViewModel[], _runs: ExecutionRun[]) {
  const groups: ChatGroup[] = [];
  const groupsByMessageId = new Map<string, ChatMessageGroup>();
  const toolGroupsByOperationKey = new Map<string, ChatStandaloneGroup>();
  let order = 0;

  for (const entry of entries) {
    if (entry.kind === "approval") {
      continue;
    }

    if (entry.kind === "message") {
      const current: ChatMessageGroup = {
        id: `group:${entry.id}`,
        order: order++,
        anchor: entry,
        accessories: [],
        relatedRuns: [],
        timestamp: entry.createdAt,
      };
      groups.push(current);
      groupsByMessageId.set(entry.messageId, current);
      continue;
    }

    if (entry.kind === "status" || entry.kind === "reasoning") {
      groups.push({
        id: `group:standalone:${entry.id}`,
        order: order++,
        anchor: null,
        accessories: [entry],
        relatedRuns: [],
        timestamp: entry.createdAt,
      });
      continue;
    }

    if (entry.kind === "tool_result") {
      const operationKey = buildToolOperationKey(entry);
      const existingGroup = toolGroupsByOperationKey.get(operationKey);
      if (existingGroup) {
        existingGroup.accessories.push(entry);
        continue;
      }

      const current: ChatStandaloneGroup = {
        id: `group:tool:${operationKey}`,
        order: order++,
        anchor: null,
        accessories: [entry],
        relatedRuns: [],
        timestamp: entry.createdAt,
      } satisfies Extract<ChatGroup, { anchor: null }>;
      groups.push(current);
      toolGroupsByOperationKey.set(operationKey, current);
      continue;
    }

    const relatedMessageId = accessoryRelatedMessageId(entry);
    if (relatedMessageId && groupsByMessageId.has(relatedMessageId)) {
      groupsByMessageId.get(relatedMessageId)!.accessories.push(entry);
      continue;
    }

    groups.push({
      id: `group:standalone:${entry.id}`,
      order: order++,
      anchor: null,
      accessories: [entry],
      relatedRuns: [],
      timestamp: entry.createdAt,
    });
  }

  return groups.sort((left, right) => {
    const byTime = left.timestamp.localeCompare(right.timestamp);
    if (byTime !== 0) {
      return byTime;
    }
    return left.order - right.order;
  });
}

function buildChatTurns(groups: ChatGroup[]) {
  const blocks: ChatStreamBlock[] = [];
  let currentOperatorGroup: Extract<ChatGroup, { anchor: ConversationMessageEntry }> | null = null;
  let trailingGroups: ChatGroup[] = [];

  const pushCurrentTurn = () => {
    if (!currentOperatorGroup) {
      return;
    }
    let finalReplyGroup: Extract<ChatGroup, { anchor: ConversationMessageEntry }> | undefined;
    for (let index = trailingGroups.length - 1; index >= 0; index -= 1) {
      const candidate = trailingGroups[index];
      if (candidate.anchor?.role === "agent") {
        finalReplyGroup = candidate as ChatMessageGroup;
        break;
      }
    }
    const processGroups = finalReplyGroup
      ? trailingGroups.filter((group) => group.id !== finalReplyGroup?.id)
      : [...trailingGroups];
    const turn: ChatTurn = {
      id: `turn:${currentOperatorGroup.anchor.messageId}`,
      operatorGroup: currentOperatorGroup,
      processGroups,
      finalReplyGroup,
      timestamp: currentOperatorGroup.timestamp,
    };
    blocks.push({
      kind: "turn",
      id: turn.id,
      timestamp: turn.timestamp,
      turn,
    });
    currentOperatorGroup = null;
    trailingGroups = [];
  };

  for (const group of groups) {
    if (group.anchor?.role === "operator") {
      pushCurrentTurn();
      currentOperatorGroup = group as ChatMessageGroup;
      trailingGroups = [];
      continue;
    }

    if (currentOperatorGroup) {
      trailingGroups.push(group);
      continue;
    }

        blocks.push({
          kind: "standalone",
          id: `standalone:${group.id}`,
          timestamp: group.timestamp,
          group: group as ChatStandaloneGroup,
        });
  }

  pushCurrentTurn();
  return blocks;
}

function TypingGlyph() {
  return (
    <div className="typing-indicator" aria-hidden="true">
      <span />
      <span />
      <span />
    </div>
  );
}

function safeToolPayload(body: string) {
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

function asNonEmptyString(value: unknown) {
  return typeof value === "string" && value.trim().length > 0 ? value.trim() : undefined;
}

function shortenInline(value: string, limit = 96) {
  return value.length > limit ? `${value.slice(0, Math.max(0, limit - 1))}…` : value;
}

function summarizeAccessoryText(value: string, limit = 96) {
  const summary = value
    .split(/\r?\n/)
    .map((line) => line.trim())
    .find(Boolean)
    ?? value.trim();
  return shortenInline(summary, limit);
}

type StructuredToolEnvelope = {
  kind?: unknown;
  tool_call_id?: unknown;
  tool_name?: unknown;
  status?: unknown;
  arguments?: unknown;
  result?: unknown;
  error?: unknown;
};

type StructuredToolError = {
  code?: unknown;
  message?: unknown;
  details?: unknown;
};

function asRecord(value: unknown) {
  if (!value || typeof value !== "object" || Array.isArray(value)) {
    return null;
  }
  return value as Record<string, unknown>;
}

function readStructuredToolEnvelope(body: string) {
  const payload = safeToolPayload(body) as StructuredToolEnvelope | null;
  if (!payload || payload.kind !== "ennoia.tool_call") {
    return null;
  }
  return payload;
}

function readToolApprovalState(entry: ChatToolResultEntry) {
  const envelope = readStructuredToolEnvelope(entry.body);
  const error = asRecord(envelope?.error) as StructuredToolError | null;
  const errorDetails = asRecord(error?.details);
  return {
    envelope,
    decision: asNonEmptyString(errorDetails?.decision),
    approvalId: asNonEmptyString(errorDetails?.approval_id),
  };
}

function detectContentFormat(body: string) {
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

function summarizeToolDescriptor(toolName: string | undefined, value: unknown) {
  const record = asRecord(value);
  if (!record) {
    return undefined;
  }

  if (toolName === "command.exec") {
    const command = asNonEmptyString(record.command);
    const rawArgs = Array.isArray(record.args) ? record.args : [];
    const args = rawArgs
      .map((item) => asNonEmptyString(item))
      .filter((item): item is string => Boolean(item));
    return [command, ...args].filter(Boolean).join(" ").trim() || undefined;
  }

  return (
    asNonEmptyString(record.path)
    ?? asNonEmptyString(record.url)
    ?? asNonEmptyString(record.cwd)
    ?? asNonEmptyString(record.command)
  );
}

function buildToolOperationKey(entry: ChatToolResultEntry) {
  const envelope = readStructuredToolEnvelope(entry.body);
  const payload = safeToolPayload(entry.body);
  const rawToolName =
    asNonEmptyString(envelope?.tool_name)
    ?? asNonEmptyString(payload?.tool)
    ?? entry.title?.trim()
    ?? entry.sender?.trim()
    ?? "tool";
  const descriptor = envelope
    ? summarizeToolDescriptor(rawToolName, envelope.arguments)
    : summarizeToolDescriptor(rawToolName, payload);
  return [
    entry.relatedMessageId ?? "standalone",
    entry.actorSender?.trim() ?? entry.sender?.trim() ?? "",
    rawToolName,
    descriptor ?? "",
  ].join("::");
}

function collapseSupersededAccessories(accessories: ChatAccessoryEntry[]) {
  const next: ChatAccessoryEntry[] = [];
  const seenToolKeys = new Set<string>();

  for (let index = accessories.length - 1; index >= 0; index -= 1) {
    const entry = accessories[index];
    if (entry.kind !== "tool_result") {
      next.push(entry);
      continue;
    }

    const toolKey = buildToolOperationKey(entry);
    const approvalState = readToolApprovalState(entry);
    const isApprovalRequest = approvalState.decision === "ask";

    if (isApprovalRequest && seenToolKeys.has(toolKey)) {
      continue;
    }

    seenToolKeys.add(toolKey);
    next.push(entry);
  }

  return next.reverse();
}

function collectGroupEntries(group: ChatGroup) {
  const entries: ChatEntryViewModel[] = [];
  if (group.anchor) {
    entries.push(group.anchor);
  }
  entries.push(...group.accessories);
  return entries;
}

function resolveTurnAgentSender(turn: ChatTurn) {
  const finalReplySender = turn.finalReplyGroup?.anchor.sender?.trim();
  if (finalReplySender) {
    return finalReplySender;
  }

  for (let index = turn.processGroups.length - 1; index >= 0; index -= 1) {
    const group = turn.processGroups[index];
    const anchorSender = group.anchor?.sender?.trim();
    if (group.anchor?.role === "agent" && anchorSender) {
      return anchorSender;
    }
    for (let accessoryIndex = group.accessories.length - 1; accessoryIndex >= 0; accessoryIndex -= 1) {
      const accessory = group.accessories[accessoryIndex];
      if (accessory.kind === "tool_result" && accessory.actorSender?.trim()) {
        return accessory.actorSender.trim();
      }
      if (accessory.kind === "reasoning" && accessory.actorSender?.trim()) {
        return accessory.actorSender.trim();
      }
    }
  }

  return undefined;
}

function summarizeProcess(groups: ChatGroup[]) {
  let stepCount = 0;
  let toolCallCount = 0;
  let approvalCount = 0;
  let thinkingCount = 0;
  let statusCount = 0;
  const toolOperationKeys = new Set<string>();

  for (const group of groups) {
    for (const entry of collectGroupEntries(group)) {
      stepCount += 1;
      if (entry.kind === "tool_result") {
        const operationKey = buildToolOperationKey(entry);
        if (!toolOperationKeys.has(operationKey)) {
          toolOperationKeys.add(operationKey);
          toolCallCount += 1;
        }
        const approvalState = readToolApprovalState(entry);
        if (approvalState.decision === "ask" || approvalState.approvalId) {
          approvalCount += 1;
        }
      }
      if (entry.kind === "status" || entry.kind === "reasoning") {
        if (entry.kind === "status") {
          statusCount += 1;
        }
        thinkingCount += 1;
      }
    }
  }

  return {
    stepCount,
    toolCallCount,
    approvalCount,
    thinkingCount,
    statusCount,
  };
}

function toolLabel(toolName: string | undefined, t: (key: string, fallback: string) => string) {
  switch (toolName) {
    case "fs.read":
      return t("web.conversations.tool_name_fs_read", "读取文件");
    case "fs.write":
      return t("web.conversations.tool_name_fs_write", "写入文件");
    case "command.exec":
      return t("web.conversations.tool_name_command_exec", "执行命令");
    case "net.fetch":
      return t("web.conversations.tool_name_net_fetch", "网络请求");
    default:
      return t("web.conversations.tool_calls_title", "工具调用");
  }
}

function resolveToolResultPresentation(
  entry: ChatToolResultEntry,
  approvalsById: Map<string, PermissionApprovalRecord>,
  t: (key: string, fallback: string) => string,
) {
  const envelope = readStructuredToolEnvelope(entry.body);
  const payload = safeToolPayload(entry.body);
  const rawToolName =
    asNonEmptyString(envelope?.tool_name)
    ?? asNonEmptyString(payload?.tool)
    ?? entry.title?.trim()
    ?? entry.sender?.trim();
  const title = toolLabel(rawToolName, t);
  const descriptor = envelope
    ? summarizeToolDescriptor(rawToolName, envelope.arguments)
    : summarizeToolDescriptor(rawToolName, payload);

  let summary = t("web.conversations.tool_output_hint", "展开查看工具输出内容");
  let badgeLabel: string | undefined;
  let badgeClassName = "badge--muted";
  let approvalStatus: string | undefined;
  let approvalRecord: PermissionApprovalRecord | undefined;

  if (envelope) {
    const error = asRecord(envelope.error) as StructuredToolError | null;
    const errorDetails = asRecord(error?.details);
    const decision = asNonEmptyString(errorDetails?.decision);
    const approvalId = asNonEmptyString(errorDetails?.approval_id);
    approvalRecord = approvalId ? approvalsById.get(approvalId) : undefined;
    approvalStatus = approvalRecord?.status;
    if (envelope.status === "succeeded") {
      badgeLabel = t("web.common.success", "成功");
      badgeClassName = "badge--accent";
    } else if (approvalStatus === "approved") {
      badgeLabel = t("web.permissions.status_approved", "已批准");
      badgeClassName = "badge--accent";
    } else if (approvalStatus === "rejected") {
      badgeLabel = t("web.permissions.status_denied", "已拒绝");
      badgeClassName = "badge--danger";
    } else if (approvalStatus === "expired") {
      badgeLabel = t("web.permissions.status_expired", "已过期");
      badgeClassName = "badge--warn";
    } else if (decision === "ask") {
      badgeLabel = t("web.conversations.permission_approval_title", "等待审批");
      badgeClassName = "badge--warn";
    } else if (decision === "deny") {
      badgeLabel = t("web.conversations.permission_denied_title", "权限已拒绝");
      badgeClassName = "badge--danger";
    } else {
      badgeLabel = t("web.action.failed", "失败");
      badgeClassName = "badge--danger";
    }

    summary = [badgeLabel, descriptor ? shortenInline(descriptor) : undefined]
      .filter((value): value is string => Boolean(value))
      .join(" · ");
  } else {
    const summaryParts = [
      rawToolName,
      descriptor ? shortenInline(descriptor) : undefined,
    ].filter((value): value is string => Boolean(value));
    summary = summaryParts.join(" · ") || summary;
  }

  return {
    title,
    summary,
    badgeLabel,
    badgeClassName,
    envelope,
    approvalRecord,
  };
}

function resolveFailurePresentation(
  entry: ConversationMessageEntry,
  t: (key: string, fallback: string) => string,
) {
  const failure = classifyConversationFailure(
    entry.failureDetail?.trim() || entry.failureSummary?.trim() || entry.body,
  );
  const source = failure?.source ?? entry.failureSource;
  const summary = failure?.summary ?? entry.failureSummary?.trim() ?? entry.body.trim();
  const detail = failure?.detail ?? entry.failureDetail?.trim() ?? entry.body.trim();

  if (source === "sandbox" || entry.failureCode === "sandbox_path_restricted") {
    return {
      kind: "sandbox" as const,
      title: t("web.conversations.sandbox_path_error_title", "沙盒路径已拦截"),
      summary: t(
        "web.conversations.sandbox_path_error_summary",
        "当前原生沙盒只允许访问 /workspace、/artifacts 和 /tmp，这次路径请求已被拦截。",
      ),
      detail: entry.failureDetail || entry.body,
    };
  }

  if (source === "provider") {
    return {
      kind: "error" as const,
      variant: "provider" as const,
      eyebrow: t("web.conversations.upstream_error_title", "上游模型错误"),
      title: t("web.conversations.upstream_error_title", "上游模型错误"),
      summary,
      detail,
    };
  }

  if (source === "timeout") {
    return {
      kind: "error" as const,
      variant: "system" as const,
      eyebrow: t("web.conversations.timeout_error_title", "请求超时"),
      title: t("web.conversations.timeout_error_title", "请求超时"),
      summary,
      detail,
    };
  }

  if (source === "configuration") {
    return {
      kind: "error" as const,
      variant: "system" as const,
      eyebrow: t("web.conversations.configuration_error_title", "配置错误"),
      title: t("web.conversations.configuration_error_title", "配置错误"),
      summary,
      detail,
    };
  }

  if (source === "extension") {
    return {
      kind: "error" as const,
      variant: "system" as const,
      eyebrow: t("web.conversations.extension_error_title", "扩展运行错误"),
      title: t("web.conversations.extension_error_title", "扩展运行错误"),
      summary,
      detail,
    };
  }

  if (source === "system") {
    return {
      kind: "error" as const,
      variant: "system" as const,
      eyebrow: t("web.conversations.system_error_title", "系统错误"),
      title: t("web.conversations.system_error_title", "系统错误"),
      summary,
      detail,
    };
  }

  if (source === "permission") {
    return {
      kind: "error" as const,
      variant: "permission" as const,
      eyebrow: t("web.conversations.permission_error_title", "权限已拒绝"),
      title: t("web.conversations.permission_error_title", "权限已拒绝"),
      summary,
      detail,
    };
  }

  return {
    kind: "error" as const,
    variant: "default" as const,
    eyebrow: t("web.conversations.error_title", "错误"),
    title: t("web.conversations.error_title", "错误"),
    summary,
    detail,
  };
}

function formatToolResultBody(result: unknown) {
  if (typeof result === "string") {
    return result;
  }
  try {
    return JSON.stringify(result ?? null, null, 2);
  } catch {
    return String(result ?? "");
  }
}

function formatDurationLabel(startedAt: string, endedAt?: string) {
  const start = Date.parse(startedAt);
  const end = endedAt ? Date.parse(endedAt) : Number.NaN;
  if (Number.isNaN(start) || Number.isNaN(end) || end <= start) {
    return "";
  }
  const seconds = Math.max(1, Math.round((end - start) / 1000));
  return `${seconds} 秒`;
}

function renderToolResultBody(
  entry: ChatToolResultEntry,
  agents: AgentProfile[],
  skills: SkillConfig[],
  approvalsById: Map<string, PermissionApprovalRecord>,
  t: (key: string, fallback: string) => string,
) {
  const envelope = readStructuredToolEnvelope(entry.body);
  if (!envelope) {
    return <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />;
  }

  if (envelope.status === "succeeded") {
    const body = formatToolResultBody(envelope.result);
    return (
      <ChatContent
        body={body}
        format={detectContentFormat(body)}
        agents={agents}
        skills={skills}
      />
    );
  }

  const error = asRecord(envelope.error) as StructuredToolError | null;
  const errorMessage = asNonEmptyString(error?.message) ?? t("web.action.failed", "失败");
  const errorDetails = error?.details;
  const errorDetailsRecord = asRecord(errorDetails);
  const decision = asNonEmptyString(errorDetailsRecord?.decision);
  const approvalId = asNonEmptyString(errorDetailsRecord?.approval_id);
  const classifiedFailure = classifyConversationFailure(errorMessage);
  const approval = approvalId ? approvalsById.get(approvalId) : undefined;
  let headline = t("web.action.failed", "失败");
  if (approval?.status === "approved") {
    headline = t("web.permissions.status_approved", "已批准");
  } else if (approval?.status === "rejected") {
    headline = t("web.permissions.status_denied", "已拒绝");
  } else if (approval?.status === "expired") {
    headline = t("web.permissions.status_expired", "已过期");
  } else if (classifiedFailure?.source === "provider") {
    headline = t("web.conversations.upstream_error_title", "上游模型错误");
  } else if (classifiedFailure?.source === "timeout") {
    headline = t("web.conversations.timeout_error_title", "请求超时");
  } else if (classifiedFailure?.source === "configuration") {
    headline = t("web.conversations.configuration_error_title", "配置错误");
  } else if (classifiedFailure?.source === "extension") {
    headline = t("web.conversations.extension_error_title", "扩展运行错误");
  } else if (classifiedFailure?.source === "system") {
    headline = t("web.conversations.system_error_title", "系统错误");
  } else if (classifiedFailure?.source === "sandbox") {
    headline = t("web.conversations.sandbox_path_error_title", "沙盒路径已拦截");
  } else if (classifiedFailure?.source === "permission") {
    headline = t("web.conversations.permission_error_title", "权限已拒绝");
  } else if (decision === "ask") {
    headline = t("web.conversations.permission_approval_title", "等待审批");
  } else if (decision === "deny") {
    headline = t("web.conversations.permission_denied_title", "权限已拒绝");
  }
  const detailsJson = errorDetails && JSON.stringify(errorDetails, null, 2) !== "{}"
    ? JSON.stringify(errorDetails, null, 2)
    : "";

  return (
    <div className="tool-result-error">
      <strong>{headline}</strong>
      <p>{errorMessage}</p>
      {detailsJson ? (
        <details className="tool-result-error__detail">
          <summary>{t("web.conversations.error_detail_toggle", "查看详情")}</summary>
          <pre className="message-pre">
            <code>{detailsJson}</code>
          </pre>
        </details>
      ) : null}
    </div>
  );
}

function SandboxBlockedBubble({
  title,
  t,
}: {
  title: string;
  t: (key: string, fallback: string) => string;
}) {
  return (
    <div className="chat-sandbox-bubble">
      <div className="chat-sandbox-bubble__header">
        <strong>{title}</strong>
      </div>
      <div className="chat-sandbox-bubble__allowlist">
        <span className="chat-sandbox-bubble__allowlist-label">
          {t("web.conversations.sandbox_allowed_paths_label", "仅允许访问这些路径")}
        </span>
        <div className="chat-sandbox-bubble__allowlist-items">
          <code>/workspace</code>
          <code>/artifacts</code>
          <code>/tmp</code>
        </div>
      </div>
    </div>
  );
}

function DefaultRecordBody({
  record,
  agents,
  skills,
}: {
  record: ExtensionRecordEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
}) {
  const payload = (() => {
    try {
      return JSON.stringify(record.payload ?? null, null, 2);
    } catch {
      return String(record.payload ?? "");
    }
  })();
  const body = record.summary?.trim() || record.title?.trim() || record.kind;

  return (
    <div className="message-record-fallback">
      <ChatContent body={body} format="markdown" agents={agents} skills={skills} />
      {payload && payload !== "null" ? (
        <details className="message-accessory__detail">
          <summary>Payload</summary>
          <pre className="message-pre">
            <code>{payload}</code>
          </pre>
        </details>
      ) : null}
    </div>
  );
}

function ConversationRecordMount({
  conversationId,
  record,
  agents,
  skills,
}: {
  conversationId: string;
  record: ExtensionRecordEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
}) {
  const helpers = useUiHelpers();
  const themeId = useUiStore((state) => state.themeId);
  const runtime = useUiStore((state) => state.runtime);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [mounted, setMounted] = useState(false);
  const { formatDate, formatDateTime, formatTime, locale, t } = helpers;
  const generation = runtime?.versions.registry ?? 0;

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void | Promise<void>) | undefined;
    const container = containerRef.current;
    if (!container) {
      return () => {
        cancelled = true;
      };
    }

    container.replaceChildren();
    setMounted(false);
    void loadExtensionConversationRecordMount(record.extension_id, generation, record.kind)
      .then(async (mount) => {
        if (cancelled || !container) {
          return;
        }
        if (!mount) {
          return;
        }
        const handle = await mount(container, {
          kind: "conversation_record",
          extensionId: record.extension_id,
          mount: record.kind,
          conversationId,
          record,
          helpers: {
            locale,
            themeId,
            apiBaseUrl: apiUrl(""),
            t,
            formatDateTime,
            formatDate,
            formatTime,
          },
        });
        if (!cancelled) {
          setMounted(true);
          cleanup = handle?.unmount;
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      void cleanup?.();
    };
  }, [conversationId, formatDate, formatDateTime, formatTime, generation, locale, record, t, themeId]);

  return (
    <div className="message-record-shell">
      <div ref={containerRef} className="message-record-shell__mount" />
      {!mounted ? (
        <DefaultRecordBody record={record} agents={agents} skills={skills} />
      ) : null}
    </div>
  );
}

function StatusBubble({
  entry,
  agents,
  t,
}: {
  entry: ChatStatusEntry;
  agents: AgentProfile[];
  t: (key: string, fallback: string) => string;
}) {
  const absoluteAt = formatAbsoluteDateTime(entry.createdAt);
  const senderLabel = resolveSenderLabel({
    role: entry.role,
    sender: entry.sender,
    agents,
    t,
  });

  return (
    <article className="chat-unit chat-unit--agent chat-unit--status">
      <div className="chat-unit__body">
        <div className="chat-unit__meta">
          {senderLabel ? <strong className="chat-unit__sender">{senderLabel}</strong> : null}
          <span className="chat-unit__time">{absoluteAt}</span>
        </div>
        <div className="message-bubble message-bubble--agent message-bubble--typing">
          <div className="message-bubble__body">
            <div className="message-status-bubble__headline">
              <TypingGlyph />
              <strong>{t("web.conversations.thinking_title", "思考")}</strong>
              <span>{entry.label}</span>
            </div>
            {entry.detail ? <p className="typing-detail">{entry.detail}</p> : null}
          </div>
        </div>
      </div>
    </article>
  );
}

function ToolResultBubble({
  entry,
  agents,
  skills,
  approvalsById,
  t,
}: {
  entry: ChatToolResultEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
}) {
  const absoluteAt = formatAbsoluteDateTime(entry.createdAt);
  const toolPresentation = resolveToolResultPresentation(entry, approvalsById, t);

  return (
    <article className="chat-unit chat-unit--agent chat-unit--tool-call">
      <div className="chat-unit__body">
        <div className="chat-unit__meta">
          <span className="chat-unit__time">{absoluteAt}</span>
        </div>
        <details className="message-accessory message-accessory--tool">
          <summary>
            <span className="message-accessory__summary-main">
              <strong>{toolPresentation.title}</strong>
              {toolPresentation.badgeLabel ? (
                <span className={`badge ${toolPresentation.badgeClassName}`}>{toolPresentation.badgeLabel}</span>
              ) : null}
              <span>{toolPresentation.summary}</span>
            </span>
          </summary>
          <div className="message-accessory__body">
            {renderToolResultBody(entry, agents, skills, approvalsById, t)}
          </div>
        </details>
      </div>
    </article>
  );
}

function AccessoryBlock({
  entry,
  agents,
  skills,
  approvalsById,
  t,
  conversationId,
}: {
  entry: ChatAccessoryEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  conversationId: string;
}) {
  const absoluteAt = formatAbsoluteDateTime(entry.createdAt);
  const senderLabel = resolveSenderLabel({
    role: entry.role,
    sender: entry.kind === "status" ? entry.sender : entry.title ?? entry.sender,
    agents,
    t,
  });

  if (entry.kind === "status") {
    return (
      <details className="message-accessory message-accessory--thinking" open>
        <summary>
          <span className="message-accessory__summary-main">
            <TypingGlyph />
            <strong>{t("web.conversations.thinking_title", "思考")}</strong>
            <span>{senderLabel ? `${senderLabel} · ${entry.label}` : entry.label}</span>
          </span>
          <small>{absoluteAt}</small>
        </summary>
        <div className="message-accessory__body">
          {entry.detail ? <p className="typing-detail">{entry.detail}</p> : null}
        </div>
      </details>
    );
  }

  if (entry.kind === "reasoning") {
    const summary = summarizeAccessoryText(entry.body);
    return (
      <details className="message-accessory message-accessory--reasoning" open>
        <summary>
          <span className="message-accessory__summary-main">
            <strong>{t("web.conversations.thinking_title", "思考")}</strong>
            <span>{senderLabel ? `${senderLabel} · ${summary}` : summary}</span>
          </span>
          <small>{absoluteAt}</small>
        </summary>
        <div className="message-accessory__body">
          <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
        </div>
      </details>
    );
  }

  if (entry.kind === "error") {
    return (
      <details className="message-accessory message-accessory--error" open>
        <summary>
          <span className="message-accessory__summary-main">
            <strong>{senderLabel || t("web.conversations.error_title", "错误")}</strong>
            <span>{entry.summary}</span>
          </span>
          <small>{absoluteAt}</small>
        </summary>
        <div className="message-accessory__body">
          {entry.detail && entry.detail !== entry.summary ? (
            <pre className="message-pre">
              <code>{entry.detail}</code>
            </pre>
          ) : (
            <p>{entry.summary}</p>
          )}
        </div>
      </details>
    );
  }

  if (entry.kind === "system") {
    return (
      <details className="message-accessory message-accessory--system">
        <summary>
          <span className="message-accessory__summary-main">
            <strong>{senderLabel}</strong>
            <span>{entry.body}</span>
          </span>
          <small>{absoluteAt}</small>
        </summary>
        <div className="message-accessory__body">
          <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
        </div>
      </details>
    );
  }

  if (entry.kind === "record") {
    return (
      <details className="message-accessory message-accessory--record" open>
        <summary>
          <span className="message-accessory__summary-main">
            <strong>{entry.record.title?.trim() || entry.record.kind}</strong>
            <span>{entry.record.summary?.trim() || entry.body}</span>
          </span>
          <small>{absoluteAt}</small>
        </summary>
        <div className="message-accessory__body">
          <ConversationRecordMount
            conversationId={conversationId}
            record={entry.record}
            agents={agents}
            skills={skills}
          />
        </div>
      </details>
    );
  }

  const toolPresentation = resolveToolResultPresentation(entry, approvalsById, t);

  return (
    <details className="message-accessory message-accessory--tool">
      <summary>
        <span className="message-accessory__summary-main">
          <strong>{toolPresentation.title}</strong>
          {toolPresentation.badgeLabel ? (
            <span className={`badge ${toolPresentation.badgeClassName}`}>{toolPresentation.badgeLabel}</span>
          ) : null}
          <span>{toolPresentation.summary}</span>
        </span>
        <small>{absoluteAt}</small>
      </summary>
      <div className="message-accessory__body">
        {renderToolResultBody(entry, agents, skills, approvalsById, t)}
      </div>
    </details>
  );
}

function MessageGroup({
  group,
  agents,
  skills,
  approvalsById,
  t,
  showThinking,
  showToolCalls,
  onCopy,
  onBranchFrom,
  onEditAndResend,
  onRetry,
  onRemove,
  showActions = true,
  conversationId,
}: {
  group: Extract<ChatGroup, { anchor: ConversationMessageEntry }>;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  showActions?: boolean;
  conversationId: string;
}) {
  const { anchor } = group;
  const isOperator = anchor.role === "operator";
  const senderLabel = resolveSenderLabel({
    role: anchor.role,
    sender: anchor.sender,
    agents,
    t,
  });
  const bubbleClassNames = [
    "message-bubble",
    isOperator ? "message-bubble--operator" : "message-bubble--agent",
    anchor.state === "pending" ? "message-bubble--pending" : "",
    anchor.state === "failed" ? "message-bubble--failed" : "",
  ]
    .filter(Boolean)
    .join(" ");
  const absoluteAt = formatAbsoluteDateTime(anchor.createdAt);
  const showSenderLabel = !isOperator && Boolean(senderLabel);
  const visibleAccessories = group.accessories.filter((entry) => {
    if (entry.kind === "status" || entry.kind === "reasoning") {
      return showThinking;
    }
    if (entry.kind === "tool_result") {
      return showToolCalls;
    }
    return true;
  });
  const mergedAccessories = collapseSupersededAccessories(visibleAccessories);
  const failurePresentation = !isOperator && anchor.state === "failed"
    ? resolveFailurePresentation(anchor, t)
    : null;

  return (
    <article className={isOperator ? "chat-unit chat-unit--operator" : "chat-unit chat-unit--agent"}>
      <div className="chat-unit__body">
        <div className="chat-unit__meta">
          {showSenderLabel ? (
            <strong className="chat-unit__sender">{senderLabel}</strong>
          ) : null}
          {anchor.rewriteFromMessageId ? (
            <span className="chat-unit__tag">{t("web.conversations.rewrite_badge", "改写分支")}</span>
          ) : null}
          {anchor.replyToMessageId && !anchor.rewriteFromMessageId ? (
            <span className="chat-unit__tag">{t("web.conversations.branch_badge", "分支消息")}</span>
          ) : null}
          {anchor.source === "local" && anchor.localStatus ? (
            <span className={`badge ${
              anchor.localStatus === "failed"
                ? "badge--danger"
                : anchor.localStatus === "sending"
                  ? "badge--accent"
                  : "badge--warn"
            }`}>
              {anchor.localStatus === "sending"
                ? t("web.conversations.message_status_sending", "发送中")
                : anchor.localStatus === "queued"
                  ? t("web.conversations.message_status_queued", "排队中")
                  : t("web.conversations.message_status_failed", "发送失败")}
            </span>
          ) : null}
          {anchor.source === "local" && anchor.dispatchMode === "insert" ? (
            <span className="chat-unit__tag">{t("web.conversations.insert_badge", "插入")}</span>
          ) : null}
          <span className="chat-unit__time">{absoluteAt}</span>
          {isOperator
            ? anchor.recipients.map((agent) => (
              <span key={agent.id} className="chat-unit__tag">@{agent.label}</span>
            ))
            : null}
        </div>

        {failurePresentation ? (
          failurePresentation.kind === "sandbox" ? (
            <SandboxBlockedBubble
              title={failurePresentation.title}
              t={t}
            />
          ) : (
            <div className={`chat-error-bubble chat-error-bubble--${failurePresentation.variant}`}>
              <div className="chat-error-bubble__header">
                <div>
                  <span className="chat-error-bubble__eyebrow">{failurePresentation.eyebrow}</span>
                  <strong>{failurePresentation.title}</strong>
                </div>
              </div>
              <p className="chat-error-bubble__summary">{failurePresentation.summary}</p>
              {failurePresentation.detail && failurePresentation.detail !== failurePresentation.summary ? (
                <details className="chat-error-bubble__detail">
                  <summary>{t("web.conversations.error_detail_toggle", "查看详情")}</summary>
                  <pre className="message-pre">
                    <code>{failurePresentation.detail}</code>
                  </pre>
                </details>
              ) : null}
            </div>
          )
        ) : (
          <div className={bubbleClassNames}>
            <div className="message-bubble__body">
              <ChatContent
                body={anchor.body}
                format={anchor.format}
                agents={agents}
                skills={skills}
                mentionAgentIds={anchor.mentions}
              />
            </div>
          </div>
        )}

        {anchor.localError ? (
          <div className="message-inline-error">
            <strong>{t("web.conversations.error_title", "错误")}</strong>
            <span>{anchor.localError}</span>
          </div>
        ) : null}

        {mergedAccessories.length > 0 ? (
          <div className="message-accessory-stack">
            {mergedAccessories.map((entry) => (
              <AccessoryBlock
                key={entry.id}
                entry={entry}
                agents={agents}
                skills={skills}
                approvalsById={approvalsById}
                t={t}
                conversationId={conversationId}
              />
            ))}
          </div>
        ) : null}

        <div className="chat-unit__footer">
          {anchor.source === "remote" && showActions ? (
            <div className="message-actions message-actions--inline">
              <button
                type="button"
                className="message-action-button message-action-button--copy"
                onClick={() => onCopy(anchor.messageId, anchor.body)}
              >
                {t("web.conversations.copy", "复制")}
              </button>
              <button
                type="button"
                className="message-action-button message-action-button--branch"
                onClick={() => onBranchFrom(anchor.messageId)}
              >
                {t("web.conversations.branch", "分支")}
              </button>
              {isOperator ? (
                <button
                  type="button"
                  className="message-action-button message-action-button--edit"
                  onClick={() => onEditAndResend(anchor.messageId)}
                >
                  {t("web.action.edit", "编辑")}
                </button>
              ) : null}
            </div>
          ) : null}

          {anchor.source === "local" && anchor.localStatus === "failed" ? (
            <div className="message-actions message-actions--inline">
              <button
                type="button"
                className="message-action-button message-action-button--retry"
                onClick={() => onRetry(anchor.id)}
              >
                {t("web.conversations.retry", "重试")}
              </button>
              <button
                type="button"
                className="message-action-button message-action-button--remove"
                onClick={() => onRemove(anchor.id)}
              >
                {t("web.conversations.remove", "移除")}
              </button>
            </div>
          ) : null}
        </div>
      </div>
    </article>
  );
}

function StandaloneGroup({
  group,
  agents,
  skills,
  approvalsById,
  t,
  showThinking,
  showToolCalls,
  conversationId,
}: {
  group: Extract<ChatGroup, { anchor: null }>;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  conversationId: string;
}) {
  const visibleAccessories = group.accessories.filter((entry) => {
    if (entry.kind === "status" || entry.kind === "reasoning") {
      return showThinking;
    }
    if (entry.kind === "tool_result") {
      return showToolCalls;
    }
    return true;
  });
  const mergedAccessories = collapseSupersededAccessories(visibleAccessories);
  if (mergedAccessories.length === 0) {
    return null;
  }

  return (
    <div className="chat-standalone-stack">
      {mergedAccessories.map((entry) =>
        entry.kind === "status" ? (
          <StatusBubble key={entry.id} entry={entry} agents={agents} t={t} />
        ) : entry.kind === "tool_result" ? (
          <ToolResultBubble
            key={entry.id}
            entry={entry}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
          />
        ) : (
          <AccessoryBlock
            key={entry.id}
            entry={entry}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            conversationId={conversationId}
          />
        ))}
    </div>
  );
}

function ProcessSummary({
  turn,
  agents,
  t,
}: {
  turn: ChatTurn;
  agents: AgentProfile[];
  t: (key: string, fallback: string) => string;
}) {
  const summary = summarizeProcess(turn.processGroups);
  const agentLabel = resolveSenderLabel({
    role: "agent",
    sender: resolveTurnAgentSender(turn),
    agents,
    t,
  });
  const endedAt =
    turn.finalReplyGroup?.timestamp
    ?? turn.processGroups[turn.processGroups.length - 1]?.timestamp;
  const duration = formatDurationLabel(turn.operatorGroup.timestamp, endedAt);
  const parts = [
    t("web.conversations.process_steps", "过程 {count} 步").replace("{count}", String(summary.stepCount)),
    summary.toolCallCount > 0
      ? t("web.conversations.process_tool_calls", "{count} 次工具调用").replace("{count}", String(summary.toolCallCount))
      : "",
    summary.approvalCount > 0
      ? t("web.conversations.process_approvals", "{count} 次审批").replace("{count}", String(summary.approvalCount))
      : "",
    duration,
  ].filter(Boolean);

  return <span>{[agentLabel, ...parts].filter(Boolean).join(" · ")}</span>;
}

function hasCollapsibleProcess(turn: ChatTurn) {
  if (!turn.finalReplyGroup || turn.processGroups.length === 0) {
    return false;
  }
  return turn.processGroups.some((group) =>
    collectGroupEntries(group).some((entry) => entry.kind !== "status"));
}

function ProcessGroupList({
  groups,
  agents,
  skills,
  approvalsById,
  t,
  showThinking,
  showToolCalls,
  onCopy,
  onBranchFrom,
  onEditAndResend,
  onRetry,
  onRemove,
  conversationId,
}: {
  groups: ChatGroup[];
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  conversationId: string;
}) {
  return (
    <>
      {groups.map((group) =>
        group.anchor ? (
          <MessageGroup
            key={group.id}
            group={group}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
            showActions={false}
            conversationId={conversationId}
          />
        ) : (
          <StandaloneGroup
            key={group.id}
            group={group}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            conversationId={conversationId}
          />
        ))}
    </>
  );
}

function TurnProcessPanel({
  turn,
  agents,
  skills,
  approvalsById,
  t,
  showThinking,
  showToolCalls,
  onCopy,
  onBranchFrom,
  onEditAndResend,
  onRetry,
  onRemove,
  conversationId,
}: {
  turn: ChatTurn;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  conversationId: string;
}) {
  if (!hasCollapsibleProcess(turn)) {
    return null;
  }

  return (
    <details className="chat-turn-process">
      <summary>
        <span className="chat-turn-process__summary-main">
          <strong>{t("web.conversations.process_title", "执行过程")}</strong>
          <ProcessSummary turn={turn} agents={agents} t={t} />
        </span>
      </summary>
      <div className="chat-turn-process__body">
        <ProcessGroupList
          groups={turn.processGroups}
          agents={agents}
          skills={skills}
          approvalsById={approvalsById}
          t={t}
          showThinking={showThinking}
          showToolCalls={showToolCalls}
          onCopy={onCopy}
          onBranchFrom={onBranchFrom}
          onEditAndResend={onEditAndResend}
          onRetry={onRetry}
          onRemove={onRemove}
          conversationId={conversationId}
        />
      </div>
    </details>
  );
}

function TurnBlock({
  turn,
  agents,
  skills,
  approvalsById,
  t,
  showThinking,
  showToolCalls,
  onCopy,
  onBranchFrom,
  onEditAndResend,
  onRetry,
  onRemove,
  conversationId,
}: {
  turn: ChatTurn;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  conversationId: string;
}) {
  return (
    <section className="chat-turn">
      <MessageGroup
        group={turn.operatorGroup}
        agents={agents}
        skills={skills}
        approvalsById={approvalsById}
        t={t}
        showThinking={false}
        showToolCalls={false}
        onCopy={onCopy}
        onBranchFrom={onBranchFrom}
        onEditAndResend={onEditAndResend}
        onRetry={onRetry}
        onRemove={onRemove}
        conversationId={conversationId}
      />
      <div className="chat-turn__agent-side">
        {turn.finalReplyGroup ? (
          <TurnProcessPanel
            turn={turn}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
            conversationId={conversationId}
          />
        ) : (
          <ProcessGroupList
            groups={turn.processGroups}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
            conversationId={conversationId}
          />
        )}
        {turn.finalReplyGroup ? (
          <MessageGroup
            group={turn.finalReplyGroup}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={false}
            showToolCalls={false}
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
            conversationId={conversationId}
          />
        ) : null}
      </div>
    </section>
  );
}

export function ChatStream({
  entries,
  runs,
  agents,
  skills,
  approvals,
  t,
  showThinking,
  showToolCalls,
  onCopy,
  onBranchFrom,
  onEditAndResend,
  onRetry,
  onRemove,
  conversationId,
}: {
  entries: ChatEntryViewModel[];
  runs: ExecutionRun[];
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvals: PermissionApprovalRecord[];
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
  conversationId: string;
}) {
  if (entries.length === 0) {
    return null;
  }

  const groups = buildChatGroups(entries, runs).filter((group) => {
    if (group.anchor) {
      return true;
    }
    const hasThinking = showThinking && group.accessories.some((entry) => entry.kind === "status");
    const hasToolCalls = showToolCalls && group.accessories.some((entry) => entry.kind === "tool_result");
    const hasOtherAccessories = group.accessories.some((entry) => entry.kind !== "status" && entry.kind !== "tool_result");
    return hasThinking || hasToolCalls || hasOtherAccessories;
  });
  const blocks = buildChatTurns(groups);
  const approvalsById = new Map(approvals.map((approval) => [approval.approval_id, approval]));
  let previousDate = "";

  return blocks.map((block) => {
    const currentDate = dateKey(block.timestamp);
    const showSeparator = currentDate !== previousDate;
    previousDate = currentDate;

    return (
      <Fragment key={block.id}>
        {showSeparator ? (
          <div className="chat-date-separator">
            <span>{currentDate}</span>
          </div>
        ) : null}
        {block.kind === "turn" ? (
          <TurnBlock
            turn={block.turn}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
            conversationId={conversationId}
          />
        ) : (
          <StandaloneGroup
            group={block.group}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
            conversationId={conversationId}
          />
        )}
      </Fragment>
    );
  });
}
