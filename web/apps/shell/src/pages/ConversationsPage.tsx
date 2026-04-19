import { Link, useNavigate } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { createConversation, loadWorkspaceSnapshot, type WorkspaceSnapshot } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ConversationsPage() {
  const navigate = useNavigate();
  const [snapshot, setSnapshot] = useState<WorkspaceSnapshot | null>(null);
  const [selectedAgent, setSelectedAgent] = useState("coder");
  const [selectedSpace, setSelectedSpace] = useState("studio");
  const [selectedGroupAgents, setSelectedGroupAgents] = useState<string[]>(["coder", "planner"]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const { formatDateTime } = useUiHelpers();

  async function refresh() {
    try {
      setSnapshot(await loadWorkspaceSnapshot());
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, []);

  async function handleCreateDirect() {
    setBusy("direct");
    setError(null);
    try {
      const created = await createConversation({
        topology: "direct",
        agent_ids: [selectedAgent],
      });
      navigate({ to: "/conversations/$conversationId", params: { conversationId: created.conversation.id } });
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(null);
    }
  }

  async function handleCreateGroup() {
    setBusy("group");
    setError(null);
    try {
      const created = await createConversation({
        topology: "group",
        space_id: selectedSpace,
        agent_ids: selectedGroupAgents,
      });
      navigate({ to: "/conversations/$conversationId", params: { conversationId: created.conversation.id } });
    } catch (err) {
      setError(String(err));
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

  if (!snapshot) {
    return <div className="page"><p>Loading conversations…</p></div>;
  }

  return (
    <div className="page">
      <h1>Conversations</h1>
      {error && <div className="error">{error}</div>}

      <section className="settings-grid">
        <div className="form-stack">
          <h3>发起一对一会话</h3>
          <label>
            目标 Agent
            <select value={selectedAgent} onChange={(event) => setSelectedAgent(event.target.value)}>
              {snapshot.agents.map((agent) => (
                <option key={agent.id} value={agent.id}>
                  {agent.display_name} ({agent.id})
                </option>
              ))}
            </select>
          </label>
          <button onClick={handleCreateDirect} disabled={busy === "direct"}>
            {busy === "direct" ? "创建中…" : "开始一对一"}
          </button>
        </div>

        <div className="form-stack">
          <h3>发起多 Agent 会话</h3>
          <label>
            归属空间
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
              <label key={agent.id}>
                <input
                  type="checkbox"
                  checked={selectedGroupAgents.includes(agent.id)}
                  onChange={() => toggleGroupAgent(agent.id)}
                />
                {agent.display_name} ({agent.id})
              </label>
            ))}
          </div>
          <button onClick={handleCreateGroup} disabled={busy === "group" || selectedGroupAgents.length === 0}>
            {busy === "group" ? "创建中…" : "开始群聊"}
          </button>
        </div>
      </section>

      <section>
        <h2>已有会话</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Title</th>
              <th>Topology</th>
              <th>Participants</th>
              <th>Updated</th>
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
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
