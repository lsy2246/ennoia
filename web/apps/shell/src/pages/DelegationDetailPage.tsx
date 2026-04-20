import { useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { getDelegation, type DelegationMessage, type DelegationThread } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function DelegationDetailPage() {
  const { chatId, delegationId } = useParams({
    from: "/shell/chat/$chatId/delegations/$delegationId",
  });
  const { t, formatDateTime } = useUiHelpers();
  const [thread, setThread] = useState<DelegationThread | null>(null);
  const [messages, setMessages] = useState<DelegationMessage[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const detail = await getDelegation(chatId, delegationId);
        if (!cancelled) {
          setThread(detail.thread);
          setMessages(detail.messages);
        }
      } catch (err) {
        if (!cancelled) {
          setError(String(err));
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [chatId, delegationId]);

  if (error) {
    return <div className="page error">{error}</div>;
  }

  if (!thread) {
    return <div className="page">{t("shell.action.loading", "加载中…")}</div>;
  }

  return (
    <div className="page delegation-page">
      <header className="page-header">
        <div className="page-header__body">
          <h1>{thread.title}</h1>
          <p className="page-header__description">{thread.summary}</p>
        </div>
      </header>

      <section className="chat-stream chat-stream--single">
        {messages.map((message) => (
          <article key={message.id} className="chat-bubble chat-bubble--agent">
            <header>
              <strong>{message.sender}</strong>
              <span>{formatDateTime(message.created_at)}</span>
            </header>
            <p>{message.body}</p>
          </article>
        ))}
      </section>
    </div>
  );
}
