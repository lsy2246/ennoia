import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  getChat,
  listAgents,
  sendChatMessage,
  type AgentProfile,
  type ChatThreadDetail,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

function extractMentionedAgents(body: string, agents: AgentProfile[], participants: string[]) {
  const mentioned = [...body.matchAll(/@([\p{L}\p{N}_.-]+)/gu)].map((match) => match[1].toLowerCase());
  if (mentioned.length === 0) {
    return [];
  }
  const participantSet = new Set(participants.filter((item) => item !== "operator"));
  const result = new Set<string>();
  for (const agent of agents) {
    const aliases = [
      agent.id.toLowerCase(),
      agent.display_name.toLowerCase(),
      agent.display_name.toLowerCase().replace(/\s+/g, "-"),
    ];
    if (participantSet.has(agent.id) && aliases.some((alias) => mentioned.includes(alias))) {
      result.add(agent.id);
    }
  }
  return [...result];
}


export function SessionView({ sessionId }: { sessionId: string }) {
  const { formatDateTime, t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [detail, setDetail] = useState<ChatThreadDetail | null>(null);
  const [draft, setDraft] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const activeAgents = useMemo(() => {
    const ids = new Set(detail?.conversation.participants.filter((item) => item !== "operator") ?? []);
    return agents.filter((agent) => ids.has(agent.id));
  }, [agents, detail]);

  useEffect(() => {
    void hydrate();
  }, [sessionId]);

  async function hydrate() {
    setError(null);
    try {
      const [nextAgents, nextDetail] = await Promise.all([listAgents(), getChat(sessionId)]);
      setAgents(nextAgents);
      setDetail(nextDetail);
    } catch (err) {
      setError(String(err));
    }
  }

  async function handleSend(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (!detail || !draft.trim()) {
      return;
    }
    const addressed = extractMentionedAgents(draft, agents, detail.conversation.participants);
    setBusy(true);
    setError(null);
    try {
      await sendChatMessage(detail.conversation.id, {
        lane_id: detail.conversation.default_lane_id ?? undefined,
        body: draft.trim(),
        addressed_agents: addressed,
      });
      setDraft("");
      await hydrate();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="session-view">
      {error ? <div className="error">{error}</div> : null}
      {detail ? (
        <>
          <header className="conversation-header">
            <div>
              <h1>{detail.conversation.title}</h1>
              <p>{detail.conversation.id} · {detail.conversation.topology}</p>
            </div>
            <div className="button-row">
              <button type="button" className="secondary" onClick={() => void hydrate()}>
                {t("web.action.refresh", "刷新")}
              </button>
            </div>
          </header>
          <div className="session-view__meta">
            <div className="tag-row">
              {activeAgents.map((agent) => (
                <button
                  key={agent.id}
                  type="button"
                  className="chip"
                  onClick={() =>
                    openView({
                      kind: "agent",
                      entityId: agent.id,
                      title: agent.display_name,
                      subtitle: agent.provider_id,
                    })}
                >
                  {agent.display_name}
                </button>
              ))}
            </div>
          </div>
          <div className="message-stream">
            {detail.messages.length === 0 ? (
              <div className="empty-card">{t("web.workspace.empty_messages", "还没有消息。在输入框里用 @agent_id 定向某个 Agent。")}</div>
            ) : (
              detail.messages.map((message) => (
                <article
                  key={message.id}
                  className={message.role === "operator" ? "message-bubble message-bubble--operator" : "message-bubble"}
                >
                  <header>
                    <strong>{message.sender}</strong>
                    <span>{formatDateTime(message.created_at)}</span>
                  </header>
                  <p>{message.body}</p>
                </article>
              ))
            )}
          </div>
          <form className="composer" onSubmit={handleSend}>
            <textarea
              value={draft}
              onChange={(event) => setDraft(event.target.value)}
              rows={4}
              placeholder={t("web.workspace.composer_placeholder", "输入消息，使用 @agent_id 定向；不写 @ 时默认发给当前 Conversation 的 Agent 集合。")}
            />
            <button type="submit" disabled={busy || !draft.trim()}>
              {t("web.workspace.send", "发送")}
            </button>
          </form>
        </>
      ) : (
        <div className="empty-card">{t("web.common.loading", "加载中…")}</div>
      )}
    </div>
  );
}
