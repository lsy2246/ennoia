import { useEffect, useMemo, useState, type FormEvent } from "react";

import {
  createChat,
  deleteChat,
  getChat,
  listAgents,
  listChats,
  sendChatMessage,
  type AgentProfile,
  type ChatThread,
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

export function WorkspacePage() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [sessions, setSessions] = useState<ChatThread[]>([]);
  const [selectedAgentIds, setSelectedAgentIds] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void refresh();
  }, []);

  async function refresh() {
    setError(null);
    try {
      const [nextAgents, nextSessions] = await Promise.all([listAgents(), listChats()]);
      setAgents(nextAgents);
      setSessions(nextSessions);
      setSelectedAgentIds((current) =>
        current.length > 0 ? current : nextAgents.filter((agent) => agent.enabled).slice(0, 1).map((agent) => agent.id),
      );
    } catch (err) {
      setError(String(err));
    }
  }

  function toggleAgent(agentId: string) {
    setSelectedAgentIds((current) =>
      current.includes(agentId)
        ? current.filter((item) => item !== agentId)
        : [...current, agentId],
    );
  }

  async function handleCreate(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    if (selectedAgentIds.length === 0) {
      setError(t("web.workspace.agent_required", "请至少选择 1 个 Agent。1 个 Agent 会创建 direct，2 个及以上会创建 group。"));
      return;
    }
    setBusy(true);
    setError(null);
    try {
      const created = await createChat({
        topology: selectedAgentIds.length === 1 ? "direct" : "group",
        title: title.trim() || undefined,
        agent_ids: selectedAgentIds,
      });
      setTitle("");
      await refresh();
      openView({
        kind: "session",
        entityId: created.conversation.id,
        title: created.conversation.title,
        subtitle: created.conversation.topology,
      });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteSession(id: string) {
    setBusy(true);
    setError(null);
    try {
      await deleteChat(id);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  return (
    <div className="workspace-grid workspace-grid--catalog">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.workspace.hero_eyebrow", "Ennoia Workspace")}</span>
          <h1>{t("web.workspace.hero_title", "统一从这里发起 direct 和 group session。")}</h1>
          <p>{t("web.workspace.hero_body", "选中 1 个 Agent 就是 direct，选中 2 个及以上就是 group。每个 Session 都有唯一 ID。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <form className="stack" onSubmit={handleCreate}>
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder={t("web.workspace.title_placeholder", "可选：Session 标题")}
          />
          <div className="chip-grid">
            {agents.map((agent) => (
              <button
                type="button"
                key={agent.id}
                className={selectedAgentIds.includes(agent.id) ? "chip chip--active" : "chip"}
                onClick={() => toggleAgent(agent.id)}
              >
                {agent.display_name}
              </button>
            ))}
          </div>
          <div className="button-row">
            <button type="submit" disabled={busy}>
              {selectedAgentIds.length <= 1
                ? t("web.workspace.create_direct", "创建 direct")
                : t("web.workspace.create_group", "创建 group")}
            </button>
            <button type="button" className="secondary" onClick={() => void refresh()}>
              {t("web.action.refresh", "刷新")}
            </button>
          </div>
        </form>
      </section>

      <section className="work-panel">
        <div className="panel-title">{t("web.workspace.panel.sessions", "Sessions")}</div>
        <div className="stack">
          {sessions.map((session) => (
            <article key={session.id} className="session-card">
              <div>
                <strong>{session.title}</strong>
                <span>
                  {session.topology === "direct" ? t("web.workspace.kind_direct", "Direct") : t("web.workspace.kind_group", "Group")}
                  {" · "}
                  {session.id}
                </span>
              </div>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "session",
                      entityId: session.id,
                      title: session.title,
                      subtitle: session.topology,
                    })}
                >
                  {t("web.action.open", "打开")}
                </button>
                <button type="button" className="danger" onClick={() => void handleDeleteSession(session.id)}>
                  {t("web.action.delete", "删除")}
                </button>
              </div>
            </article>
          ))}
        </div>
      </section>
    </div>
  );
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
    const ids = new Set(detail?.thread.participants.filter((item) => item !== "operator") ?? []);
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
    const addressed = extractMentionedAgents(draft, agents, detail.thread.participants);
    setBusy(true);
    setError(null);
    try {
      await sendChatMessage(detail.thread.id, {
        lane_id: detail.thread.default_lane_id ?? undefined,
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
              <h1>{detail.thread.title}</h1>
              <p>{detail.thread.id} · {detail.thread.topology}</p>
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
              placeholder={t("web.workspace.composer_placeholder", "输入消息，使用 @agent_id 定向；不写 @ 时默认发给当前 Session 的 Agent 集合。")}
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
