import type {
  AgentProfile,
  ExecutionRun,
  PermissionApprovalRecord,
  SkillConfig,
} from "@ennoia/api-client";
import { Fragment } from "react";

import { ChatContent } from "./ChatContent";
import type {
  ChatEntryViewModel,
  ChatErrorEntry,
  ChatStatusEntry,
  ChatSystemEntry,
  ChatToolResultEntry,
  ConversationMessageEntry,
} from "./chat-types";

type ChatAccessoryEntry = ChatErrorEntry | ChatStatusEntry | ChatSystemEntry | ChatToolResultEntry;

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
  const groupsByMessageId = new Map<string, Extract<ChatGroup, { anchor: ConversationMessageEntry }>>();
  let order = 0;

  for (const entry of entries) {
    if (entry.kind === "approval") {
      continue;
    }

    if (entry.kind === "message") {
      const current = {
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

    if (entry.kind === "status" || entry.kind === "tool_result") {
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
  if (entry.failureCode === "sandbox_path_restricted") {
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

  const summary = entry.failureSummary?.trim() || entry.body.trim();
  const detail = entry.failureDetail?.trim() || entry.body.trim();
  return {
    kind: "error" as const,
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
  const approval = approvalId ? approvalsById.get(approvalId) : undefined;
  const headline = approval?.status === "approved"
    ? t("web.permissions.status_approved", "已批准")
    : approval?.status === "rejected"
      ? t("web.permissions.status_denied", "已拒绝")
      : approval?.status === "expired"
        ? t("web.permissions.status_expired", "已过期")
        : decision === "ask"
          ? t("web.conversations.permission_approval_title", "等待审批")
          : decision === "deny"
            ? t("web.conversations.permission_denied_title", "权限已拒绝")
            : t("web.action.failed", "失败");
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
  const actorSender = entry.actorSender?.trim();
  const actorLabel = actorSender
    ? resolveSenderLabel({
        role: "agent",
        sender: actorSender,
        agents,
        t,
      })
    : "";
  const toolPresentation = resolveToolResultPresentation(entry, approvalsById, t);

  return (
    <article className="chat-unit chat-unit--agent chat-unit--tool-call">
      <div className="chat-unit__body">
        <div className="chat-unit__meta">
          {actorLabel ? <strong className="chat-unit__sender">{actorLabel}</strong> : null}
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
}: {
  entry: ChatAccessoryEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
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
    if (entry.kind === "status") {
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
            <div className="chat-error-bubble">
              <div className="chat-error-bubble__header">
                <div>
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
              />
            ))}
          </div>
        ) : null}

        <div className="chat-unit__footer">
          {anchor.source === "remote" ? (
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
}: {
  group: Extract<ChatGroup, { anchor: null }>;
  agents: AgentProfile[];
  skills: SkillConfig[];
  approvalsById: Map<string, PermissionApprovalRecord>;
  t: (key: string, fallback: string) => string;
  showThinking: boolean;
  showToolCalls: boolean;
}) {
  const visibleAccessories = group.accessories.filter((entry) => {
    if (entry.kind === "status") {
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
          />
        ))}
    </div>
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
  const approvalsById = new Map(approvals.map((approval) => [approval.approval_id, approval]));
  let previousDate = "";

  return groups.map((group) => {
    const currentDate = dateKey(group.timestamp);
    const showSeparator = currentDate !== previousDate;
    previousDate = currentDate;

    return (
      <Fragment key={group.id}>
        {showSeparator ? (
          <div className="chat-date-separator">
            <span>{currentDate}</span>
          </div>
        ) : null}
        {group.anchor ? (
          <MessageGroup
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
          />
        ) : (
          <StandaloneGroup
            group={group}
            agents={agents}
            skills={skills}
            approvalsById={approvalsById}
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
          />
        )}
      </Fragment>
    );
  });
}
