import { Link, useNavigate } from "@tanstack/react-router";
import { useState } from "react";

import { createConversation, deleteConversation } from "@ennoia/api-client";
import { PageHeader } from "@/components/PageHeader";
import { useWorkspaceSnapshot } from "@/hooks/useWorkspaceSnapshot";
import { useUiHelpers } from "@/stores/ui";

export function ConversationsPage() {
  const navigate = useNavigate();
  const { snapshot, loading, error, refresh } = useWorkspaceSnapshot();
  const [selectedAgent, setSelectedAgent] = useState("coder");
  const [selectedSpace, setSelectedSpace] = useState("studio");
  const [selectedGroupAgents, setSelectedGroupAgents] = useState<string[]>(["coder", "planner"]);
  const [busy, setBusy] = useState<string | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);
  const { t, formatDateTime } = useUiHelpers();

  async function handleCreateDirect() {
    setBusy("direct");
    setActionError(null);
    try {
      const created = await createConversation({
        topology: "direct",
        agent_ids: [selectedAgent],
      });
      await refresh();
      navigate({ to: "/conversations/$conversationId", params: { conversationId: created.conversation.id } });
    } catch (err) {
      setActionError(String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleCreateGroup() {
    setBusy("group");
    setActionError(null);
    try {
      const created = await createConversation({
        topology: "group",
        space_id: selectedSpace,
        agent_ids: selectedGroupAgents,
      });
      await refresh();
      navigate({ to: "/conversations/$conversationId", params: { conversationId: created.conversation.id } });
    } catch (err) {
      setActionError(String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleDeleteConversation(conversationId: string) {
    if (!window.confirm(t("shell.conversations.confirm_delete", "Delete this conversation and its related runtime data?"))) {
      return;
    }
    setBusy(conversationId);
    setActionError(null);
    try {
      await deleteConversation(conversationId);
      await refresh();
    } catch (err) {
      setActionError(String(err));
    } finally {
      setBusy(null);
    }
  }

  function toggleGroupAgent(agentId: string) {
    setSelectedGroupAgents((current) =>
      current.includes(agentId)
        ? current.filter((item) => item !== agentId)
        : [...current, agentId],
    );
  }

  if (loading || !snapshot) {
    return <div className="page"><p>{t("shell.loading.conversations", "Loading conversations…")}</p></div>;
  }

  return (
    <div className="page">
      <PageHeader
        title={t("shell.page.conversations.title", "Conversations")}
        description={t(
          "shell.page.conversations.description",
          "Create, inspect and clean up direct or group conversations from the main control surface.",
        )}
        meta={[
          `${snapshot.conversations.length} ${t("shell.meta.total", "total")}`,
          `${snapshot.agents.length} ${t("shell.nav.agents", "Agents")}`,
          `${snapshot.spaces.length} ${t("shell.nav.spaces", "Spaces")}`,
        ]}
        actions={
          <button className="secondary" onClick={() => void refresh()}>
            {t("shell.action.refresh", "Refresh")}
          </button>
        }
      />

      {(error || actionError) && <div className="error">{actionError ?? error}</div>}

      <section className="split-grid">
        <div className="surface-card">
          <h2>{t("shell.conversations.direct", "Direct conversation")}</h2>
          <label>
            {t("shell.conversations.target_agent", "Target agent")}
            <select value={selectedAgent} onChange={(event) => setSelectedAgent(event.target.value)}>
              {snapshot.agents.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.display_name} ({agent.id})
                </option>
              ))}
            </select>
          </label>
          <div className="actions">
            <button onClick={handleCreateDirect} disabled={busy === "direct"}>
              {busy === "direct" ? t("shell.action.creating", "Creating…") : t("shell.conversations.start_direct", "Start direct")}
            </button>
          </div>
        </div>

        <div className="surface-card">
          <h2>{t("shell.conversations.group", "Group conversation")}</h2>
          <label>
            {t("shell.conversations.space", "Space")}
            <select value={selectedSpace} onChange={(event) => setSelectedSpace(event.target.value)}>
              {snapshot.spaces.map((space) => (
                <option key={space.id} value={space.id}>
                  {space.display_name} ({space.id})
                </option>
              ))}
            </select>
          </label>
          <div className="simple-list">
            {snapshot.agents.map((agent) => (
              <label key={agent.id} className="check-row">
                <input
                  type="checkbox"
                  checked={selectedGroupAgents.includes(agent.id)}
                  onChange={() => toggleGroupAgent(agent.id)}
                />
                <span>{agent.display_name} ({agent.id})</span>
              </label>
            ))}
          </div>
          <div className="actions">
            <button onClick={handleCreateGroup} disabled={busy === "group" || selectedGroupAgents.length === 0}>
              {busy === "group" ? t("shell.action.creating", "Creating…") : t("shell.conversations.start_group", "Start group")}
            </button>
          </div>
        </div>
      </section>

      <section>
        <h2>{t("shell.conversations.existing", "Existing conversations")}</h2>
        <table className="table">
          <thead>
            <tr>
              <th>{t("shell.conversations.title", "Title")}</th>
              <th>{t("shell.conversations.topology", "Topology")}</th>
              <th>{t("shell.conversations.participants", "Participants")}</th>
              <th>{t("shell.conversations.updated_at", "Updated")}</th>
              <th>{t("shell.conversations.actions", "Actions")}</th>
            </tr>
          </thead>
          <tbody>
            {snapshot.conversations.map((conversation) => (
              <tr key={conversation.id}>
                <td>
                  <Link to="/conversations/$conversationId" params={{ conversationId: conversation.id }}>
                    {conversation.title}
                  </Link>
                </td>
                <td>{conversation.topology}</td>
                <td>{conversation.participants.join(", ")}</td>
                <td>{formatDateTime(conversation.updated_at)}</td>
                <td>
                  <div className="row-actions">
                    <button
                      className="danger"
                      onClick={() => void handleDeleteConversation(conversation.id)}
                      disabled={busy === conversation.id}
                    >
                      {busy === conversation.id ? t("shell.action.deleting", "Deleting…") : t("shell.action.delete", "Delete")}
                    </button>
                  </div>
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
