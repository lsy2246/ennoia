import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { createChat, deleteChat, listAgents, listChats, type AgentProfile, type ChatThread } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

export function ChatPage() {
  const navigate = useNavigate();
  const { t, formatDateTime } = useUiHelpers();
  const [threads, setThreads] = useState<ChatThread[]>([]);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [selectedAgent, setSelectedAgent] = useState("");
  const [groupAgents, setGroupAgents] = useState<string>("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      const [nextThreads, nextAgents] = await Promise.all([listChats(), listAgents()]);
      setThreads(nextThreads);
      setAgents(nextAgents);
      setSelectedAgent((current) => current || nextAgents[0]?.id || "");
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, []);

  async function handleCreateDirect() {
    if (!selectedAgent) {
      return;
    }
    setBusy(true);
    try {
      const created = await createChat({
        topology: "direct",
        agent_ids: [selectedAgent],
      });
      await navigate({ to: "/chat/$chatId", params: { chatId: created.conversation.id } });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleCreateGroup() {
    const agentIds = groupAgents
      .split(",")
      .map((item) => item.trim())
      .filter(Boolean);
    if (agentIds.length === 0) {
      return;
    }
    setBusy(true);
    try {
      const created = await createChat({
        topology: "group",
        agent_ids: agentIds,
      });
      await navigate({ to: "/chat/$chatId", params: { chatId: created.conversation.id } });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDelete(chatId: string) {
    if (!window.confirm(t("shell.chat.delete_confirm", "删除这个聊天及其执行记录？"))) {
      return;
    }
    try {
      await deleteChat(chatId);
      await refresh();
    } catch (err) {
      setError(String(err));
    }
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.chat.title", "聊天")}
        description={t(
          "shell.chat.description",
          "把私聊和群聊统一成聊天线程。主视图是聊天盒子，执行过程、子 Agent、工具与输出都附着在消息流上下文中。",
        )}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "刷新")}
          </button>
        }
      />

      {error ? <div className="error">{error}</div> : null}

      <div className="surface-grid">
        <section className="surface-panel">
          <h2>{t("shell.chat.start_direct", "发起私聊")}</h2>
          <div className="form-stack">
            <label>
              {t("shell.chat.select_agent", "选择 Agent")}
              <select value={selectedAgent} onChange={(event) => setSelectedAgent(event.target.value)}>
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.id}>
                    {agent.display_name} · {agent.default_model}
                  </option>
                ))}
              </select>
            </label>
            <button onClick={() => void handleCreateDirect()} disabled={busy || !selectedAgent}>
              {t("shell.chat.create_direct", "创建私聊")}
            </button>
          </div>
        </section>

        <section className="surface-panel">
          <h2>{t("shell.chat.start_group", "发起群聊")}</h2>
          <div className="form-stack">
            <label>
              {t("shell.chat.group_agents", "参与 Agent")}
              <input
                value={groupAgents}
                onChange={(event) => setGroupAgents(event.target.value)}
                placeholder="coder,planner"
              />
            </label>
            <button className="secondary" onClick={() => void handleCreateGroup()} disabled={busy}>
              {t("shell.chat.create_group", "创建群聊")}
            </button>
          </div>
        </section>
      </div>

      <section className="surface-panel">
        <div className="section-heading">
          <h2>{t("shell.chat.thread_list", "聊天列表")}</h2>
          <span className="muted">
            {loading ? t("shell.action.loading", "加载中…") : `${threads.length} ${t("shell.common.items", "项")}`}
          </span>
        </div>

        <div className="stack-list">
          {threads.map((thread) => (
            <article key={thread.id} className="thread-card">
              <div>
                <div className="thread-card__title">
                  <Link to="/chat/$chatId" params={{ chatId: thread.id }}>
                    {thread.title}
                  </Link>
                </div>
                <p>
                  {thread.topology === "direct"
                    ? t("shell.chat.kind.direct", "私聊")
                    : t("shell.chat.kind.group", "群聊")}
                  {" · "}
                  {thread.participants.join(", ")}
                </p>
                <span>{formatDateTime(thread.updated_at)}</span>
              </div>
              <button className="danger" onClick={() => void handleDelete(thread.id)}>
                {t("shell.action.delete", "删除")}
              </button>
            </article>
          ))}
          {!loading && threads.length === 0 ? (
            <p className="muted">{t("shell.chat.empty", "还没有聊天线程。")}</p>
          ) : null}
        </div>
      </section>
    </div>
  );
}
