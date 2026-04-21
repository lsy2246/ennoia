import { useEffect, useState, type FormEvent } from "react";

import {
  createChat,
  deleteChat,
  listAgents,
  listChats,
  type AgentProfile,
  type ChatThread,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

export function Conversations() {
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
      setError(t("web.conversations.agent_required", "请至少选择 1 个 Agent。1 个 Agent 会创建 direct，2 个及以上会创建 group。"));
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
    <div className="conversation-grid conversation-grid--catalog">
      <section className="work-panel">
        <div className="page-heading">
          <span>{t("web.conversations.hero_eyebrow", "Conversations")}</span>
          <h1>{t("web.conversations.hero_title", "统一从这里发起 direct 和 group conversation。")}</h1>
          <p>{t("web.conversations.hero_body", "选中 1 个 Agent 就是 direct，选中 2 个及以上就是 group。每个 Conversation 都有唯一 ID。")}</p>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <form className="stack" onSubmit={handleCreate}>
          <input
            value={title}
            onChange={(event) => setTitle(event.target.value)}
            placeholder={t("web.conversations.title_placeholder", "可选：Conversation 标题")}
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
                ? t("web.conversations.create_direct", "创建 direct")
                : t("web.conversations.create_group", "创建 group")}
            </button>
            <button type="button" className="secondary" onClick={() => void refresh()}>
              {t("web.action.refresh", "刷新")}
            </button>
          </div>
        </form>
      </section>

      <section className="work-panel">
        <div className="panel-title">{t("web.conversations.panel.conversations", "Conversations")}</div>
        <div className="stack">
          {sessions.map((conversation) => (
            <article key={conversation.id} className="session-card">
              <div>
                <strong>{conversation.title}</strong>
                <span>
                  {conversation.topology === "direct" ? t("web.conversations.kind_direct", "Direct") : t("web.conversations.kind_group", "Group")}
                  {" · "}
                  {conversation.id}
                </span>
              </div>
              <div className="button-row">
                <button
                  type="button"
                  className="secondary"
                  onClick={() =>
                    openView({
                      kind: "session",
                      entityId: conversation.id,
                      title: conversation.title,
                      subtitle: conversation.topology,
                    })}
                >
                  {t("web.action.open", "打开")}
                </button>
                <button type="button" className="danger" onClick={() => void handleDeleteSession(conversation.id)}>
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


