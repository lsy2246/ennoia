import type { AgentProfile, ExecutionRun, SkillConfig } from "@ennoia/api-client";
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
  t: (key: string, fallback: string) => string,
) {
  const payload = safeToolPayload(entry.body);
  const rawToolName = asNonEmptyString(payload?.tool) ?? entry.title?.trim() ?? entry.sender?.trim();
  const title = toolLabel(rawToolName, t);

  let descriptor: string | undefined;
  if (rawToolName === "command.exec") {
    const command = asNonEmptyString(payload?.command);
    const rawArgs = Array.isArray(payload?.args) ? payload.args : [];
    const args = rawArgs
      .map((value) => asNonEmptyString(value))
      .filter((value): value is string => Boolean(value));
    descriptor = [command, ...args].filter(Boolean).join(" ").trim() || undefined;
  } else {
    descriptor =
      asNonEmptyString(payload?.path) ??
      asNonEmptyString(payload?.url) ??
      asNonEmptyString(payload?.cwd) ??
      asNonEmptyString(payload?.command);
  }

  const summaryParts = [
    rawToolName,
    descriptor ? shortenInline(descriptor) : undefined,
  ].filter((value): value is string => Boolean(value));

  return {
    title,
    summary:
      summaryParts.join(" · ") ||
      t("web.conversations.tool_output_hint", "展开查看工具输出内容"),
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
  t,
}: {
  entry: ChatToolResultEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
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
  const toolPresentation = resolveToolResultPresentation(entry, t);

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
              <span>{toolPresentation.summary}</span>
            </span>
          </summary>
          <div className="message-accessory__body">
            <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
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
  t,
}: {
  entry: ChatAccessoryEntry;
  agents: AgentProfile[];
  skills: SkillConfig[];
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

  const toolPresentation = resolveToolResultPresentation(entry, t);

  return (
    <details className="message-accessory message-accessory--tool">
      <summary>
        <span className="message-accessory__summary-main">
          <strong>{toolPresentation.title}</strong>
          <span>{toolPresentation.summary}</span>
        </span>
        <small>{absoluteAt}</small>
      </summary>
      <div className="message-accessory__body">
        <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
      </div>
    </details>
  );
}

function MessageGroup({
  group,
  agents,
  skills,
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

        {visibleAccessories.length > 0 ? (
          <div className="message-accessory-stack">
            {visibleAccessories.map((entry) => (
              <AccessoryBlock key={entry.id} entry={entry} agents={agents} skills={skills} t={t} />
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
  t,
  showThinking,
  showToolCalls,
}: {
  group: Extract<ChatGroup, { anchor: null }>;
  agents: AgentProfile[];
  skills: SkillConfig[];
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
  if (visibleAccessories.length === 0) {
    return null;
  }

  return (
    <div className="chat-standalone-stack">
      {visibleAccessories.map((entry) =>
        entry.kind === "status" ? (
          <StatusBubble key={entry.id} entry={entry} agents={agents} t={t} />
        ) : entry.kind === "tool_result" ? (
          <ToolResultBubble key={entry.id} entry={entry} agents={agents} skills={skills} t={t} />
        ) : (
          <AccessoryBlock key={entry.id} entry={entry} agents={agents} skills={skills} t={t} />
        ))}
    </div>
  );
}

export function ChatStream({
  entries,
  runs,
  agents,
  skills,
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
            t={t}
            showThinking={showThinking}
            showToolCalls={showToolCalls}
          />
        )}
      </Fragment>
    );
  });
}
