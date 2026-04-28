import { useEffect, useState, type FormEvent } from "react";

import {
  createChat,
  deleteChat,
  listAgents,
  listChats,
  type AgentProfile,
  type ChatThread,
} from "@ennoia/api-client";
import { useConversationsStore } from "@/stores/conversations";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

function formatConversationTime(value: string) {
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
}

export function Conversations() {
  const { t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const openViews = useWorkbenchStore((state) => state.openViews);
  const closeView = useWorkbenchStore((state) => state.closeView);
  const conversationRevision = useConversationsStore((state) => state.revision);
  const notifyChanged = useConversationsStore((state) => state.notifyChanged);
  const notifyDeleted = useConversationsStore((state) => state.notifyDeleted);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [sessions, setSessions] = useState<ChatThread[]>([]);
  const [selectedAgentIds, setSelectedAgentIds] = useState<string[]>([]);
  const [title, setTitle] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    void refresh();
  }, [conversationRevision]);

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
      notifyChanged();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  async function handleDeleteSession(id: string) {
    const confirmed = window.confirm(
      t("web.conversations.delete_confirm", "确认删除这个会话吗？删除后无法恢复，并且已打开的相关会话窗口会被关闭。"),
    );
    if (!confirmed) {
      return;
    }

    setBusy(true);
    setError(null);
    try {
      await deleteChat(id);
      for (const view of openViews) {
        if (view.kind === "session" && view.entityId === id) {
          closeView(view.panelId);
        }
      }
      notifyDeleted(id);
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  const directCount = sessions.filter((item) => item.topology === "direct").length;
  const groupCount = sessions.filter((item) => item.topology === "group").length;
  const enabledAgents = agents.filter((item) => item.enabled).length;
  const selectedAgents = agents.filter((agent) => selectedAgentIds.includes(agent.id));

  return (
    <div className="conversations-page">
      <section className="work-panel conversations-toolbar">
        <div className="conversations-toolbar__row">
          <div className="page-heading">
            <span>{t("web.conversations.hero_eyebrow", "Conversations")}</span>
            <h1>{t("web.conversations.hero_title", "统一从这里发起 direct 和 group conversation。")}</h1>
            <p>{t("web.conversations.hero_body", "选中 1 个 Agent 就是 direct，选中 2 个及以上就是 group。每个 Conversation 都有唯一 ID。")}</p>
          </div>
          <div className="conversations-toolbar__actions">
            <button type="button" className="secondary" onClick={() => void refresh()} disabled={busy}>
              {busy ? t("web.common.loading", "加载中…") : t("web.action.refresh", "刷新")}
            </button>
          </div>
        </div>
        {error ? <div className="error">{error}</div> : null}
        <div className="conversations-overview-grid">
          <article className="metric-card conversations-metric-card">
            <span>{t("web.conversations.summary_total", "会话总数")}</span>
            <strong>{sessions.length}</strong>
            <small>{t("web.conversations.panel.conversations", "会话")}</small>
          </article>
          <article className="metric-card conversations-metric-card">
            <span>{t("web.conversations.summary_direct", "私聊")}</span>
            <strong>{directCount}</strong>
            <small>{t("web.conversations.kind_direct", "Direct")}</small>
          </article>
          <article className="metric-card conversations-metric-card">
            <span>{t("web.conversations.summary_group", "群聊")}</span>
            <strong>{groupCount}</strong>
            <small>{t("web.conversations.kind_group", "Group")}</small>
          </article>
          <article className="metric-card conversations-metric-card">
            <span>{t("web.conversations.summary_agents", "可用 Agent")}</span>
            <strong>{enabledAgents}</strong>
            <small>{t("web.conversations.summary_selected", "当前已选")} {selectedAgentIds.length}</small>
          </article>
        </div>
      </section>

      <div className="conversations-shell">
        <section className="work-panel conversations-creator-panel">
          <div className="conversations-section__header">
            <div className="page-heading">
              <span>{t("web.conversations.create_eyebrow", "Create")}</span>
              <h1>{t("web.conversations.create_title", "发起新会话")}</h1>
              <p>{t("web.conversations.create_description", "先选 Agent，再决定是直接协作还是多人群聊。")}</p>
            </div>
            <span className="badge badge--muted">{`${selectedAgentIds.length} ${t("web.conversations.summary_selected", "已选")}`}</span>
          </div>

          <form className="conversations-create-form" onSubmit={handleCreate}>
            <label>
              {t("web.conversations.title_placeholder", "可选：会话标题")}
              <input
                value={title}
                onChange={(event) => setTitle(event.target.value)}
                placeholder={t("web.conversations.title_placeholder", "可选：Conversation 标题")}
              />
            </label>

            <div className="conversations-agent-picker">
              {agents.map((agent) => (
                <button
                  type="button"
                  key={agent.id}
                  className={`chip conversations-agent-chip ${selectedAgentIds.includes(agent.id) ? "chip--active" : ""}`}
                  onClick={() => toggleAgent(agent.id)}
                >
                  {agent.display_name}
                </button>
              ))}
            </div>

            <div className="conversations-selected-meta">
              {selectedAgents.length === 0 ? (
                <span className="badge badge--muted">{t("web.conversations.none_selected", "还没有选中 Agent")}</span>
              ) : (
                selectedAgents.map((agent) => (
                  <span key={agent.id} className="badge badge--muted">{agent.display_name}</span>
                ))
              )}
            </div>

            <div className="button-row">
              <button type="submit" disabled={busy}>
                {selectedAgentIds.length <= 1
                  ? t("web.conversations.create_direct", "创建 direct")
                  : t("web.conversations.create_group", "创建 group")}
              </button>
            </div>
          </form>
        </section>

        <section className="work-panel conversations-catalog-panel">
          <div className="conversations-section__header">
            <div className="page-heading">
              <span>{t("web.conversations.panel.conversations", "Conversations")}</span>
              <h1>{t("web.conversations.catalog_title", "会话目录")}</h1>
              <p>{t("web.conversations.catalog_description", "这里保留最近的 direct 和 group 会话，便于继续打开、切换和删除。")}</p>
            </div>
            <span className="conversations-catalog-count">{`${sessions.length} ${t("web.conversations.catalog_count", "条")}`}</span>
          </div>

          <div className="conversations-catalog-list">
            {sessions.length === 0 ? (
              <div className="empty-card conversations-empty-state">
                <strong>{t("web.conversations.empty_title", "还没有会话")}</strong>
                <p>{t("web.conversations.empty_body", "先在左侧选择 Agent，然后创建一个 direct 或 group 会话。")}</p>
              </div>
            ) : (
              sessions.map((conversation) => (
                <article key={conversation.id} className="session-card conversations-session-card">
                  <div className="conversations-session-card__header">
                    <div className="stack conversations-session-card__title">
                      <strong>{conversation.title}</strong>
                      <small>{conversation.id}</small>
                    </div>
                    <span className={`badge ${conversation.topology === "direct" ? "badge--accent" : "badge--muted"}`}>
                      {conversation.topology === "direct" ? t("web.conversations.kind_direct", "Direct") : t("web.conversations.kind_group", "Group")}
                    </span>
                  </div>
                  <div className="conversations-session-meta">
                    <span>{`${t("web.conversations.participants", "参与 Agent")} ${conversation.participants.length}`}</span>
                    <span>{`${t("web.conversations.updated_at", "最近更新")} ${formatConversationTime(conversation.updated_at)}`}</span>
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
              ))
            )}
          </div>
        </section>
      </div>
    </div>
  );
}
