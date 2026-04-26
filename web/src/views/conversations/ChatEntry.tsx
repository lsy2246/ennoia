import type { AgentProfile, SkillConfig } from "@ennoia/api-client";

import { ChatContent } from "./ChatContent";
import type { ChatEntryViewModel } from "./chat-types";

function TypingGlyph() {
  return (
    <div className="typing-indicator" aria-hidden="true">
      <span />
      <span />
      <span />
    </div>
  );
}

export function ChatEntry({
  entry,
  agents,
  skills,
  formatDateTime,
  t,
  onRetry,
  onRemove,
}: {
  entry: ChatEntryViewModel;
  agents: AgentProfile[];
  skills: SkillConfig[];
  formatDateTime: (value: string) => string;
  t: (key: string, fallback: string) => string;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  if (entry.kind === "system") {
    return (
      <div className="chat-system-entry">
        <span className="chat-system-entry__line" />
        <div className="chat-system-entry__body">
          <strong>{entry.title ?? t("web.conversations.system_label", "系统消息")}</strong>
          <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
        </div>
        <span className="chat-system-entry__line" />
      </div>
    );
  }

  if (entry.kind === "status") {
    return (
      <article className="message-bubble message-bubble--typing">
        <header className="message-bubble__header">
          <strong>{entry.sender ?? entry.label}</strong>
          <small>{entry.label}</small>
        </header>
        <div className="message-bubble__body">
          <TypingGlyph />
          {entry.detail ? <p className="typing-detail">{entry.detail}</p> : null}
        </div>
      </article>
    );
  }

  if (entry.kind === "error") {
    return (
      <article className={`chat-error-bubble chat-error-bubble--${entry.tone}`}>
        <header className="chat-error-bubble__header">
          <div>
            <strong>{entry.title || t("web.conversations.error_title", "错误")}</strong>
            <small>{formatDateTime(entry.createdAt)}</small>
          </div>
        </header>
        <p className="chat-error-bubble__summary">{entry.summary}</p>
        {entry.detail && entry.detail !== entry.summary ? (
          <details className="chat-error-bubble__detail">
            <summary>{t("web.conversations.error_detail_toggle", "查看详情")}</summary>
            <pre className="message-pre">
              <code>{entry.detail}</code>
            </pre>
          </details>
        ) : null}
      </article>
    );
  }

  if (entry.kind === "tool_result") {
    return (
      <article className="message-bubble message-bubble--tool">
        <header className="message-bubble__header">
          <strong>{entry.title ?? entry.sender ?? t("web.conversations.tool_label", "工具结果")}</strong>
          <small>{formatDateTime(entry.createdAt)}</small>
        </header>
        <div className="message-bubble__body">
          <ChatContent body={entry.body} format={entry.format} agents={agents} skills={skills} />
        </div>
      </article>
    );
  }

  const isOperator = entry.role === "operator";
  const bubbleClassNames = [
    "message-bubble",
    isOperator ? "message-bubble--operator" : "message-bubble--agent",
    entry.state === "pending" ? "message-bubble--pending" : "",
    entry.state === "failed" ? "message-bubble--failed" : "",
  ]
    .filter(Boolean)
    .join(" ");

  return (
    <article className={bubbleClassNames}>
      <header className="message-bubble__header">
        <strong>{entry.sender}</strong>
        <small>{formatDateTime(entry.createdAt)}</small>
      </header>
      <div className="message-bubble__body">
        <ChatContent
          body={entry.body}
          format={entry.format}
          agents={agents}
          skills={skills}
          mentionAgentIds={entry.mentions}
        />
      </div>
      {isOperator ? (
        <footer className="message-bubble__footer">
          <div className="message-route">
            <div className="message-route__agents">
              {entry.recipients.map((agent) => (
                <span key={agent.id} className="badge badge--muted">@{agent.label}</span>
              ))}
            </div>
          </div>
          <div className="message-state">
            {entry.source === "local" ? (
              <>
                <span className={`badge ${
                  entry.localStatus === "failed"
                    ? "badge--danger"
                    : entry.localStatus === "sending"
                      ? "badge--accent"
                      : "badge--warn"
                }`}>
                  {entry.localStatus === "sending"
                    ? t("web.conversations.message_status_sending", "发送中")
                    : entry.localStatus === "queued"
                      ? t("web.conversations.message_status_queued", "排队中")
                      : t("web.conversations.message_status_failed", "发送失败")}
                </span>
                {entry.localStatus === "failed" ? (
                  <div className="button-row">
                    <button type="button" className="secondary" onClick={() => onRetry(entry.id)}>
                      {t("web.conversations.retry", "重试")}
                    </button>
                    <button type="button" className="secondary" onClick={() => onRemove(entry.id)}>
                      {t("web.conversations.remove", "移除")}
                    </button>
                  </div>
                ) : null}
              </>
            ) : (
              <span className="message-delivery">
                {t("web.conversations.message_status_delivered", "已送达")}
              </span>
            )}
          </div>
        </footer>
      ) : null}
      {entry.localError ? (
        <div className="message-inline-error">
          <strong>{t("web.conversations.error_title", "错误")}</strong>
          <span>{entry.localError}</span>
        </div>
      ) : null}
    </article>
  );
}
