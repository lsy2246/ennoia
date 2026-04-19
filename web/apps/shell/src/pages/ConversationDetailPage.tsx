import { useParams } from "@tanstack/react-router";
import { useEffect, useState, type FormEvent } from "react";

import {
  getConversation,
  loadConversationMessages,
  loadConversationRuns,
  sendConversationMessage,
  type ConversationDetailResponse,
  type Message,
  type Run,
} from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ConversationDetailPage() {
  const { conversationId } = useParams({ from: "/shell/conversations/$conversationId" });
  const [detail, setDetail] = useState<ConversationDetailResponse | null>(null);
  const [messages, setMessages] = useState<Message[]>([]);
  const [runs, setRuns] = useState<Run[]>([]);
  const [laneId, setLaneId] = useState<string>("");
  const [body, setBody] = useState("");
  const [goal, setGoal] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const { formatDateTime } = useUiHelpers();

  async function refresh() {
    try {
      const [nextDetail, nextMessages, nextRuns] = await Promise.all([
        getConversation(conversationId),
        loadConversationMessages(conversationId),
        loadConversationRuns(conversationId),
      ]);
      setDetail(nextDetail);
      setMessages(nextMessages);
      setRuns(nextRuns);
      setLaneId((current) => current || nextDetail.conversation.default_lane_id || nextDetail.lanes[0]?.id || "");
    } catch (err) {
      setError(String(err));
    }
  }

  useEffect(() => {
    refresh();
  }, [conversationId]);

  async function handleSubmit(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setBusy(true);
    setError(null);
    try {
      await sendConversationMessage(conversationId, {
        lane_id: laneId || undefined,
        body,
        goal: goal || undefined,
      });
      setBody("");
      setGoal("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setBusy(false);
    }
  }

  if (!detail) {
    return <div className="page"><p>Loading conversation…</p></div>;
  }

  return (
    <div className="page">
      <h1>{detail.conversation.title}</h1>
      {error && <div className="error">{error}</div>}

      <section className="settings-grid">
        <div className="form-stack">
          <h3>基本信息</h3>
          <p>Topology: <code>{detail.conversation.topology}</code></p>
          <p>Participants: <code>{detail.conversation.participants.join(", ")}</code></p>
          <p>Owner: <code>{detail.conversation.owner.kind}/{detail.conversation.owner.id}</code></p>
        </div>

        <form className="form-stack" onSubmit={handleSubmit}>
          <h3>发送消息</h3>
          <label>
            当前线
            <select value={laneId} onChange={(event) => setLaneId(event.target.value)}>
              {detail.lanes.map((lane) => (
                <option key={lane.id} value={lane.id}>
                  {lane.name} ({lane.participants.join(", ")})
                </option>
              ))}
            </select>
          </label>
          <label>
            Goal
            <input value={goal} onChange={(event) => setGoal(event.target.value)} />
          </label>
          <label>
            Message
            <textarea
              value={body}
              onChange={(event) => setBody(event.target.value)}
              rows={5}
              required
            />
          </label>
          <button type="submit" disabled={busy}>
            {busy ? "发送中…" : "发送并触发运行"}
          </button>
        </form>
      </section>

      <section>
        <h2>Lanes</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Name</th>
              <th>Type</th>
              <th>Status</th>
              <th>Goal</th>
            </tr>
          </thead>
          <tbody>
            {detail.lanes.map((lane) => (
              <tr key={lane.id}>
                <td>{lane.name}</td>
                <td>{lane.lane_type}</td>
                <td>{lane.status}</td>
                <td>{lane.goal}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Messages</h2>
        <table className="table">
          <thead>
            <tr>
              <th>Sender</th>
              <th>Role</th>
              <th>Body</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {messages.map((message) => (
              <tr key={message.id}>
                <td>{message.sender}</td>
                <td>{message.role}</td>
                <td>{message.body}</td>
                <td>{formatDateTime(message.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>

      <section>
        <h2>Recent runs</h2>
        <table className="table">
          <thead>
            <tr>
              <th>ID</th>
              <th>Stage</th>
              <th>Goal</th>
              <th>Created</th>
            </tr>
          </thead>
          <tbody>
            {runs.map((run) => (
              <tr key={run.id}>
                <td><code>{run.id.slice(0, 12)}</code></td>
                <td>{run.stage}</td>
                <td>{run.goal}</td>
                <td>{formatDateTime(run.created_at)}</td>
              </tr>
            ))}
          </tbody>
        </table>
      </section>
    </div>
  );
}
