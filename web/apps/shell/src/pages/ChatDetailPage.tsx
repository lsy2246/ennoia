import { Link, useParams } from "@tanstack/react-router";
import { useEffect, useState } from "react";

import { getChat, sendChatMessage, type ChatThreadDetail } from "@ennoia/api-client";
import { useUiHelpers } from "@/stores/ui";

export function ChatDetailPage() {
  const { chatId } = useParams({ from: "/shell/chat/$chatId" });
  const { t, formatDateTime } = useUiHelpers();
  const [detail, setDetail] = useState<ChatThreadDetail | null>(null);
  const [draft, setDraft] = useState("");
  const [goal, setGoal] = useState("");
  const [showThinking, setShowThinking] = useState(true);
  const [showTools, setShowTools] = useState(true);
  const [loading, setLoading] = useState(true);
  const [sending, setSending] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function refresh() {
    setLoading(true);
    setError(null);
    try {
      setDetail(await getChat(chatId));
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    void refresh();
  }, [chatId]);

  async function handleSend() {
    if (!draft.trim()) {
      return;
    }
    setSending(true);
    setError(null);
    try {
      await sendChatMessage(chatId, {
        lane_id: detail?.lanes[0]?.id,
        body: draft,
        goal: goal || draft,
      });
      setDraft("");
      setGoal("");
      await refresh();
    } catch (err) {
      setError(String(err));
    } finally {
      setSending(false);
    }
  }

  if (loading) {
    return <div className="page">{t("shell.loading.conversation", "正在加载聊天详情…")}</div>;
  }

  if (error || !detail) {
    return <div className="page error">{error ?? t("shell.common.failed", "失败")}</div>;
  }

  const latestRun = detail.runs[0];

  return (
    <div className="chat-workbench">
      <aside className="chat-pane chat-pane--sidebar">
        <div className="chat-pane__header">
          <h2>{detail.thread.title}</h2>
          <p>{detail.thread.participants.join(", ")}</p>
        </div>

        <div className="stack-list">
          {detail.delegations.map((delegation) => (
            <Link
              key={delegation.id}
              className="delegation-chip"
              to="/chat/$chatId/delegations/$delegationId"
              params={{ chatId, delegationId: delegation.id }}
            >
              <strong>{delegation.title}</strong>
              <span>{delegation.status}</span>
            </Link>
          ))}
        </div>
      </aside>

      <section className="chat-pane chat-pane--center">
        <div className="chat-stream">
          {detail.messages.map((message) => (
            <article
              key={message.id}
              className={`chat-bubble chat-bubble--${message.role === "operator" ? "operator" : "agent"}`}
            >
              <header>
                <strong>{message.sender}</strong>
                <span>{formatDateTime(message.created_at)}</span>
              </header>
              <p>{message.body}</p>
            </article>
          ))}

          {showThinking && latestRun ? (
            <article className="event-card">
              <header>
                <strong>{t("shell.chat.event.thinking", "思考过程")}</strong>
                <span>{latestRun.stage}</span>
              </header>
              <p>{latestRun.goal}</p>
            </article>
          ) : null}

          {showTools
            ? detail.outputs.map((output) => (
                <article key={output.id} className="event-card">
                  <header>
                    <strong>{t("shell.chat.event.output", "输出产物")}</strong>
                    <span>{output.kind}</span>
                  </header>
                  <p>{output.relative_path}</p>
                </article>
              ))
            : null}
        </div>

        <div className="chat-composer">
          <label>
            {t("shell.chat.goal", "目标")}
            <input value={goal} onChange={(event) => setGoal(event.target.value)} />
          </label>
          <label>
            {t("shell.chat.message", "消息")}
            <textarea rows={5} value={draft} onChange={(event) => setDraft(event.target.value)} />
          </label>
          <div className="chat-composer__actions">
            <button onClick={() => void handleSend()} disabled={sending}>
              {sending ? t("shell.conversation_detail.sending", "发送中…") : t("shell.chat.send", "发送")}
            </button>
          </div>
        </div>
      </section>

      <aside className="chat-pane chat-pane--inspector">
        <section className="inspector-card">
          <h3>{t("shell.chat.inspector", "Inspector")}</h3>
          <dl className="data-pairs">
            <dt>ID</dt>
            <dd>{detail.thread.id}</dd>
            <dt>{t("shell.chat.thread_kind", "类型")}</dt>
            <dd>{detail.thread.topology}</dd>
            <dt>{t("shell.chat.updated_at", "更新时间")}</dt>
            <dd>{formatDateTime(detail.thread.updated_at)}</dd>
          </dl>
        </section>

        <section className="inspector-card">
          <h3>{t("shell.chat.stream_options", "消息流选项")}</h3>
          <label className="check-row">
            <input
              type="checkbox"
              checked={showThinking}
              onChange={(event) => setShowThinking(event.target.checked)}
            />
            <span>{t("shell.chat.toggle_thinking", "显示思考过程")}</span>
          </label>
          <label className="check-row">
            <input
              type="checkbox"
              checked={showTools}
              onChange={(event) => setShowTools(event.target.checked)}
            />
            <span>{t("shell.chat.toggle_tools", "显示工具与输出")}</span>
          </label>
        </section>
      </aside>

      <section className="chat-bottom">
        <div className="bottom-panel">
          <h3>{t("shell.chat.execution", "执行过程")}</h3>
          <div className="stack-list stack-list--compact">
            {detail.tasks.map((task) => (
              <div key={task.id} className="execution-row">
                <strong>{task.title}</strong>
                <span>
                  {task.assigned_agent_id} · {task.status}
                </span>
              </div>
            ))}
          </div>
        </div>

        <div className="bottom-panel">
          <h3>{t("shell.chat.logs", "运行日志")}</h3>
          <div className="stack-list stack-list--compact">
            {detail.runs.map((run) => (
              <div key={run.id} className="execution-row">
                <strong>{run.goal}</strong>
                <span>{run.stage}</span>
              </div>
            ))}
          </div>
        </div>
      </section>
    </div>
  );
}
