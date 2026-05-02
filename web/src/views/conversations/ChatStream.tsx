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
      anchor: ConversationMessageEntry;
      accessories: ChatAccessoryEntry[];
      relatedRuns: ExecutionRun[];
      timestamp: string;
    }
  | {
      id: string;
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

function buildChatGroups(entries: ChatEntryViewModel[], runs: ExecutionRun[]) {
  const groups: ChatGroup[] = [];
  const groupsByMessageId = new Map<string, Extract<ChatGroup, { anchor: ConversationMessageEntry }>>();

  for (const entry of entries) {
    if (entry.kind === "approval") {
      continue;
    }

    if (entry.kind === "message") {
      const current = {
        id: `group:${entry.id}`,
        anchor: entry,
        accessories: [],
        relatedRuns: [],
        timestamp: entry.createdAt,
      };
      groups.push(current);
      groupsByMessageId.set(entry.messageId, current);
      continue;
    }

    const relatedMessageId = accessoryRelatedMessageId(entry);
    if (relatedMessageId && groupsByMessageId.has(relatedMessageId)) {
      groupsByMessageId.get(relatedMessageId)!.accessories.push(entry);
      continue;
    }

    groups.push({
      id: `group:standalone:${entry.id}`,
      anchor: null,
      accessories: [entry],
      relatedRuns: [],
      timestamp: entry.createdAt,
    });
  }

  for (const run of runs) {
    const sourceMessageId = run.source_message_id ?? undefined;
    if (!sourceMessageId) {
      continue;
    }
    const target = groupsByMessageId.get(sourceMessageId);
    if (target) {
      target.relatedRuns.push(run);
    }
  }

  for (const group of groups) {
    group.relatedRuns.sort((left, right) => right.updated_at.localeCompare(left.updated_at));
  }

  return groups;
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

function stageTone(stage: string) {
  switch (stage) {
    case "completed":
      return "success";
    case "failed":
    case "blocked":
    case "cancelled":
      return "danger";
    case "pending":
    case "planning":
    case "dispatched":
    case "running":
    case "reviewing":
      return "warn";
    default:
      return "muted";
  }
}

function summarizeRunGoal(goal: string) {
  const normalized = goal.trim().replace(/\s+/g, " ");
  if (normalized.length <= 72) {
    return normalized;
  }
  return `${normalized.slice(0, 72)}...`;
}

function ProcessAccessoryBlock({
  runs,
  t,
}: {
  runs: ExecutionRun[];
  t: (key: string, fallback: string) => string;
}) {
  const latest = runs[0];
  const label = latest
    ? `${runs.length} ${t("web.conversations.process_runs_suffix", "个运行")} · ${latest.stage}`
    : t("web.conversations.process_empty", "没有过程信息");

  return (
    <details className="message-accessory message-accessory--run">
      <summary>
        <span className="message-accessory__summary-main">
          <strong>{t("web.conversations.process_title", "思考与执行过程")}</strong>
          <span>{label}</span>
        </span>
        {latest ? <small>{formatAbsoluteDateTime(latest.updated_at)}</small> : null}
      </summary>
      <div className="message-accessory__body">
        <div className="process-run-list">
          {runs.map((run) => (
            <article key={run.id} className="process-run-card">
              <div className="process-run-card__header">
                <strong>{summarizeRunGoal(run.goal)}</strong>
                <span className={`badge badge--${stageTone(run.stage)}`}>{run.stage}</span>
              </div>
              <div className="process-run-card__meta">
                <span>{run.id}</span>
                <span>{run.trigger}</span>
                <span>{formatAbsoluteDateTime(run.updated_at)}</span>
              </div>
            </article>
          ))}
        </div>
      </div>
    </details>
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
    sender: entry.title ?? entry.sender,
    agents,
    t,
  });

  if (entry.kind === "status") {
    return (
      <details className="message-accessory message-accessory--thinking" open>
        <summary>
          <span className="message-accessory__summary-main">
            <TypingGlyph />
            <strong>{senderLabel}</strong>
            <span>{entry.label}</span>
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

  return (
    <details className="message-accessory message-accessory--tool">
      <summary>
        <span className="message-accessory__summary-main">
          <strong>{senderLabel}</strong>
          <span>{t("web.conversations.tool_output_hint", "展开查看工具输出内容")}</span>
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

        {anchor.localError ? (
          <div className="message-inline-error">
            <strong>{t("web.conversations.error_title", "错误")}</strong>
            <span>{anchor.localError}</span>
          </div>
        ) : null}

        {(group.relatedRuns.length > 0 || group.accessories.length > 0) ? (
          <div className="message-accessory-stack">
            {group.relatedRuns.length > 0 ? (
              <ProcessAccessoryBlock runs={group.relatedRuns} t={t} />
            ) : null}
            {group.accessories.map((entry) => (
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
}: {
  group: Extract<ChatGroup, { anchor: null }>;
  agents: AgentProfile[];
  skills: SkillConfig[];
  t: (key: string, fallback: string) => string;
}) {
  return (
    <div className="chat-standalone-stack">
      {group.accessories.map((entry) => (
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
  onCopy: (entryId: string, body: string) => void;
  onBranchFrom: (messageId: string) => void;
  onEditAndResend: (messageId: string) => void;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  if (entries.length === 0) {
    return null;
  }

  const groups = buildChatGroups(entries, runs);
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
            onCopy={onCopy}
            onBranchFrom={onBranchFrom}
            onEditAndResend={onEditAndResend}
            onRetry={onRetry}
            onRemove={onRemove}
          />
        ) : (
          <StandaloneGroup group={group} agents={agents} skills={skills} t={t} />
        )}
      </Fragment>
    );
  });
}
