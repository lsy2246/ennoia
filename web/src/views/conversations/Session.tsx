import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import {
  ApiError,
  getChat,
  listAgents,
  sendChatMessage,
  type AgentProfile,
  type ChatMessage,
  type ChatThreadDetail,
} from "@ennoia/api-client";
import { useConversationsStore } from "@/stores/conversations";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";

type LocalMessageStatus = "queued" | "sending" | "failed";

type LocalMessageDraft = {
  clientId: string;
  body: string;
  addressedAgents: string[];
  createdAt: string;
  status: LocalMessageStatus;
  error?: string;
};

type DisplayMessage =
  | {
      kind: "remote";
      id: string;
      role: ChatMessage["role"];
      sender: string;
      body: string;
      mentions: string[];
      createdAt: string;
    }
  | {
      kind: "local";
      id: string;
      role: "operator";
      sender: string;
      body: string;
      mentions: string[];
      createdAt: string;
      status: LocalMessageStatus;
      error?: string;
    };

type MentionState = {
  open: boolean;
  query: string;
  selectedIndex: number;
};

type ComposerSnapshot = {
  body: string;
  addressedAgents: string[];
};

const EMPTY_MENTION_STATE: MentionState = {
  open: false,
  query: "",
  selectedIndex: 0,
};

function uniqueStrings(values: string[]) {
  return [...new Set(values.map((item) => item.trim()).filter(Boolean))];
}

function nowIso() {
  return new Date().toISOString();
}

function createLocalDraft(snapshot: ComposerSnapshot): LocalMessageDraft {
  return {
    clientId: `local-${Math.random().toString(36).slice(2, 10)}`,
    body: snapshot.body,
    addressedAgents: snapshot.addressedAgents,
    createdAt: nowIso(),
    status: "queued",
  };
}

function createMentionNode(agent: AgentProfile) {
  const node = document.createElement("span");
  node.className = "composer-mention";
  node.contentEditable = "false";
  node.dataset.agentId = agent.id;
  node.dataset.agentLabel = agent.display_name;
  node.textContent = `@${agent.display_name}`;
  return node;
}

function readComposerSnapshot(root: HTMLElement | null): ComposerSnapshot {
  if (!root) {
    return { body: "", addressedAgents: [] };
  }

  const addressedAgents: string[] = [];
  const parts: string[] = [];

  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      parts.push(node.textContent ?? "");
      return;
    }

    if (!(node instanceof HTMLElement)) {
      return;
    }

    if (node.dataset.agentId) {
      addressedAgents.push(node.dataset.agentId);
      parts.push(`@${node.dataset.agentLabel ?? node.dataset.agentId}`);
      return;
    }

    if (node.tagName === "BR") {
      parts.push("\n");
      return;
    }

    const isBlock = node !== root && ["DIV", "P"].includes(node.tagName);
    if (isBlock && parts.length > 0 && !parts[parts.length - 1].endsWith("\n")) {
      parts.push("\n");
    }

    for (const child of [...node.childNodes]) {
      walk(child);
    }

    if (isBlock && parts.length > 0 && !parts[parts.length - 1].endsWith("\n")) {
      parts.push("\n");
    }
  };

  for (const child of [...root.childNodes]) {
    walk(child);
  }

  return {
    body: parts.join("").replace(/\u00a0/g, " ").replace(/\n{3,}/g, "\n\n").trim(),
    addressedAgents: uniqueStrings(addressedAgents),
  };
}

function focusComposerEnd(root: HTMLElement | null) {
  if (!root || typeof window === "undefined") {
    return;
  }
  root.focus();
  const selection = window.getSelection();
  if (!selection) {
    return;
  }
  const range = document.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  selection.removeAllRanges();
  selection.addRange(range);
}

function clearComposer(root: HTMLElement | null) {
  if (!root) {
    return;
  }
  root.innerHTML = "";
  root.dataset.empty = "true";
}

function isSelectionInside(root: HTMLElement | null, node: Node | null) {
  if (!root || !node) {
    return false;
  }
  return root === node || root.contains(node);
}

function extractMentionQuery(root: HTMLElement | null) {
  if (!root || typeof window === "undefined") {
    return null;
  }

  const selection = window.getSelection();
  if (!selection || !selection.isCollapsed) {
    return null;
  }

  const anchorNode = selection.anchorNode;
  if (!isSelectionInside(root, anchorNode)) {
    return null;
  }

  if (!(anchorNode instanceof Text)) {
    return null;
  }

  const offset = selection.anchorOffset;
  const text = anchorNode.textContent ?? "";
  const before = text.slice(0, offset);
  const match = before.match(/(?:^|\s)@([\p{L}\p{N}_.-]*)$/u);
  if (!match) {
    return null;
  }

  const query = match[1] ?? "";
  const atIndex = before.lastIndexOf("@");
  if (atIndex < 0) {
    return null;
  }

  return {
    textNode: anchorNode,
    atIndex,
    offset,
    query,
  };
}

function replaceMentionAtCaret(root: HTMLElement | null, agent: AgentProfile) {
  const context = extractMentionQuery(root);
  if (!root || !context || typeof window === "undefined") {
    return false;
  }

  const original = context.textNode.textContent ?? "";
  const before = original.slice(0, context.atIndex);
  const after = original.slice(context.offset);
  context.textNode.textContent = before;

  const mentionNode = createMentionNode(agent);
  const trailingText = document.createTextNode(after.startsWith(" ") ? after : ` ${after}`);
  const parent = context.textNode.parentNode;
  if (!parent) {
    return false;
  }

  parent.insertBefore(mentionNode, context.textNode.nextSibling);
  parent.insertBefore(trailingText, mentionNode.nextSibling);

  const selection = window.getSelection();
  if (selection) {
    const range = document.createRange();
    range.setStart(trailingText, 1);
    range.collapse(true);
    selection.removeAllRanges();
    selection.addRange(range);
  }

  if ((context.textNode.textContent ?? "").length === 0) {
    parent.removeChild(context.textNode);
  }

  root.dataset.empty = "false";
  return true;
}

function handleMentionBackspace(root: HTMLElement | null) {
  if (!root || typeof window === "undefined") {
    return false;
  }
  const selection = window.getSelection();
  if (!selection || !selection.isCollapsed) {
    return false;
  }

  const anchorNode = selection.anchorNode;
  if (!isSelectionInside(root, anchorNode)) {
    return false;
  }

  if (anchorNode instanceof Text && selection.anchorOffset === 0) {
    const previous = anchorNode.previousSibling;
    if (previous instanceof HTMLElement && previous.dataset.agentId) {
      previous.remove();
      root.dataset.empty = String(readComposerSnapshot(root).body.length === 0);
      return true;
    }
  }

  if (anchorNode instanceof HTMLElement && selection.anchorOffset > 0) {
    const previous = anchorNode.childNodes[selection.anchorOffset - 1];
    if (previous instanceof HTMLElement && previous.dataset.agentId) {
      previous.remove();
      root.dataset.empty = String(readComposerSnapshot(root).body.length === 0);
      return true;
    }
  }

  return false;
}

function summarizeBody(body: string) {
  const normalized = body.replace(/\s+/g, " ").trim();
  if (normalized.length <= 56) {
    return normalized;
  }
  return `${normalized.slice(0, 56)}…`;
}

export function SessionView({ sessionId, panelId }: { sessionId: string; panelId?: string }) {
  const { formatDateTime, t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const closeView = useWorkbenchStore((state) => state.closeView);
  const conversationRevision = useConversationsStore((state) => state.revision);
  const deletedSessionMark = useConversationsStore((state) => state.deletedSessionMarks[sessionId]);
  const notifyChanged = useConversationsStore((state) => state.notifyChanged);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [detail, setDetail] = useState<ChatThreadDetail | null>(null);
  const [localDrafts, setLocalDrafts] = useState<LocalMessageDraft[]>([]);
  const [mentionState, setMentionState] = useState<MentionState>(EMPTY_MENTION_STATE);
  const [composerSnapshot, setComposerSnapshot] = useState<ComposerSnapshot>({ body: "", addressedAgents: [] });
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<HTMLDivElement | null>(null);

  const activeAgents = useMemo(() => {
    const ids = new Set(detail?.conversation?.participants?.filter((item) => item !== "operator") ?? []);
    return agents.filter((agent) => ids.has(agent.id));
  }, [agents, detail]);

  const canMention = activeAgents.length > 1;

  const agentMap = useMemo(
    () => new Map(activeAgents.map((agent) => [agent.id, agent])),
    [activeAgents],
  );

  const hydrate = useCallback(async () => {
    setError(null);
    setDetail(null);
    try {
      const [nextAgents, nextDetail] = await Promise.all([listAgents(), getChat(sessionId)]);
      setAgents(nextAgents);
      setDetail(nextDetail);
    } catch (err) {
      if (err instanceof ApiError && err.status === 404 && panelId) {
        closeView(panelId);
        return;
      }
      setError(String(err));
    }
  }, [closeView, panelId, sessionId]);

  useEffect(() => {
    void hydrate();
  }, [conversationRevision, hydrate]);

  useEffect(() => {
    if (deletedSessionMark && panelId) {
      closeView(panelId);
    }
  }, [closeView, deletedSessionMark, panelId]);

  const syncComposerState = useCallback(() => {
    const snapshot = readComposerSnapshot(editorRef.current);
    setComposerSnapshot(snapshot);
    editorRef.current?.setAttribute("data-empty", String(snapshot.body.length === 0));
    if (!canMention) {
      setMentionState(EMPTY_MENTION_STATE);
      return;
    }
    const context = extractMentionQuery(editorRef.current);
    if (!context) {
      setMentionState(EMPTY_MENTION_STATE);
      return;
    }
    setMentionState((current) => ({
      open: true,
      query: context.query,
      selectedIndex: current.selectedIndex,
    }));
  }, [canMention]);

  useEffect(() => {
    syncComposerState();
  }, [syncComposerState]);

  const mentionOptions = useMemo(() => {
    if (!canMention) {
      return [];
    }
    const query = mentionState.query.trim().toLowerCase();
    const options = activeAgents.filter((agent) => {
      if (!query) {
        return true;
      }
      const haystacks = [
        agent.id.toLowerCase(),
        agent.display_name.toLowerCase(),
        agent.display_name.toLowerCase().replace(/\s+/g, "-"),
      ];
      return haystacks.some((item) => item.includes(query));
    });
    return options;
  }, [activeAgents, canMention, mentionState.query]);

  useEffect(() => {
    if (!mentionState.open) {
      return;
    }
    if (mentionOptions.length === 0) {
      setMentionState((current) => ({ ...current, selectedIndex: 0 }));
      return;
    }
    if (mentionState.selectedIndex >= mentionOptions.length) {
      setMentionState((current) => ({ ...current, selectedIndex: 0 }));
    }
  }, [mentionOptions.length, mentionState.open, mentionState.selectedIndex]);

  const queueItems = useMemo(
    () => localDrafts.filter((item) => item.status === "sending" || item.status === "queued"),
    [localDrafts],
  );

  const failedItems = useMemo(
    () => localDrafts.filter((item) => item.status === "failed"),
    [localDrafts],
  );

  const sendingItem = queueItems.find((item) => item.status === "sending") ?? null;
  const waitingItems = queueItems.filter((item) => item.status === "queued");

  const sessionStatus = useMemo(() => {
    if (sendingItem) {
      return {
        tone: "accent",
        label: t("web.conversations.status_sending", "正在发送消息…"),
        detail: t("web.conversations.status_sending_detail", "当前消息已进入处理链路，请稍候。"),
      };
    }
    if (waitingItems.length > 0) {
      return {
        tone: "warn",
        label: t("web.conversations.status_queue", "排队中"),
        detail: t("web.conversations.status_queue_detail", "还有 {count} 条消息等待发送。").replace("{count}", String(waitingItems.length)),
      };
    }
    if (failedItems.length > 0) {
      return {
        tone: "danger",
        label: t("web.conversations.status_failed", "有消息发送失败"),
        detail: t("web.conversations.status_failed_detail", "你可以重试失败消息，或从队列中移除它。"),
      };
    }
    return {
      tone: "muted",
      label: t("web.conversations.status_idle", "会话空闲"),
      detail: t("web.conversations.status_idle_detail", "消息会串行发送；如果连续提交，会进入可见队列。"),
    };
  }, [failedItems.length, sendingItem, t, waitingItems.length]);

  const displayMessages = useMemo<DisplayMessage[]>(() => {
    const remoteMessages = (detail?.messages ?? []).map<DisplayMessage>((message) => ({
      kind: "remote",
      id: message.id,
      role: message.role,
      sender: message.sender,
      body: message.body,
      mentions: message.mentions,
      createdAt: message.created_at,
    }));
    const localMessages = localDrafts.map<DisplayMessage>((item) => ({
      kind: "local",
      id: item.clientId,
      role: "operator",
      sender: "Operator",
      body: item.body,
      mentions: item.addressedAgents,
      createdAt: item.createdAt,
      status: item.status,
      error: item.error,
    }));
    return [...remoteMessages, ...localMessages].sort((left, right) => left.createdAt.localeCompare(right.createdAt));
  }, [detail?.messages, localDrafts]);

  const resolveRecipients = useCallback((mentions: string[]) => {
    const resolved = uniqueStrings(mentions)
      .map((agentId) => agentMap.get(agentId))
      .filter((item): item is AgentProfile => Boolean(item));
    if (resolved.length > 0) {
      return resolved;
    }
    return activeAgents;
  }, [activeAgents, agentMap]);

  const resetComposer = useCallback(() => {
    clearComposer(editorRef.current);
    setComposerSnapshot({ body: "", addressedAgents: [] });
    setMentionState(EMPTY_MENTION_STATE);
    focusComposerEnd(editorRef.current);
  }, []);

  const enqueueCurrentMessage = useCallback(() => {
    if (!detail?.conversation) {
      return;
    }
    const snapshot = readComposerSnapshot(editorRef.current);
    if (!snapshot.body.trim()) {
      return;
    }
    const recipients = snapshot.addressedAgents.length > 0
      ? snapshot.addressedAgents
      : activeAgents.map((agent) => agent.id);
    const queued = createLocalDraft({
      body: snapshot.body.trim(),
      addressedAgents: recipients,
    });
    setLocalDrafts((current) => [...current, queued]);
    resetComposer();
  }, [activeAgents, detail?.conversation, resetComposer]);

  useEffect(() => {
    if (!detail?.conversation || sendingItem) {
      return;
    }

    const next = localDrafts.find((item) => item.status === "queued");
    if (!next) {
      return;
    }

    let cancelled = false;
    setLocalDrafts((current) =>
      current.map((item) => item.clientId === next.clientId ? { ...item, status: "sending", error: undefined } : item),
    );

    void (async () => {
      try {
        await sendChatMessage(detail.conversation.id, {
          lane_id: detail.conversation.default_lane_id ?? undefined,
          body: next.body,
          addressed_agents: next.addressedAgents,
        });
        notifyChanged();
        await hydrate();
        if (!cancelled) {
          setLocalDrafts((current) => current.filter((item) => item.clientId !== next.clientId));
        }
      } catch (err) {
        if (!cancelled) {
          setLocalDrafts((current) =>
            current.map((item) => item.clientId === next.clientId
              ? { ...item, status: "failed", error: String(err) }
              : item),
          );
          setError(String(err));
        }
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [detail?.conversation, hydrate, localDrafts, notifyChanged, sendingItem]);

  const retryLocalMessage = useCallback((clientId: string) => {
    setLocalDrafts((current) =>
      current.map((item) => item.clientId === clientId ? { ...item, status: "queued", error: undefined } : item),
    );
  }, []);

  const removeLocalMessage = useCallback((clientId: string) => {
    setLocalDrafts((current) => current.filter((item) => item.clientId !== clientId));
  }, []);

  const chooseMention = useCallback((agent: AgentProfile) => {
    if (replaceMentionAtCaret(editorRef.current, agent)) {
      syncComposerState();
    }
    setMentionState(EMPTY_MENTION_STATE);
  }, [syncComposerState]);

  const handleComposerKeyDown = useCallback((event: KeyboardEvent<HTMLDivElement>) => {
    if (mentionState.open && mentionOptions.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setMentionState((current) => ({
          ...current,
          selectedIndex: (current.selectedIndex + 1) % mentionOptions.length,
        }));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setMentionState((current) => ({
          ...current,
          selectedIndex: (current.selectedIndex - 1 + mentionOptions.length) % mentionOptions.length,
        }));
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        chooseMention(mentionOptions[mentionState.selectedIndex] ?? mentionOptions[0]);
        return;
      }
      if (event.key === "Escape") {
        event.preventDefault();
        setMentionState(EMPTY_MENTION_STATE);
        return;
      }
    }

    if (event.key === "Backspace" && handleMentionBackspace(editorRef.current)) {
      event.preventDefault();
      syncComposerState();
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      enqueueCurrentMessage();
    }
  }, [chooseMention, enqueueCurrentMessage, mentionOptions, mentionState.open, mentionState.selectedIndex, syncComposerState]);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const node = scrollRef.current;
    if (!node) {
      return;
    }
    node.scrollTop = node.scrollHeight;
  }, [displayMessages.length, queueItems.length]);

  return (
    <div className="session-view session-view--chat">
      {error ? <div className="error">{error}</div> : null}
      {detail?.conversation ? (
        <>
          <header className="conversation-header conversation-header--chat">
            <div className="conversation-header__main">
              <span className="conversation-header__eyebrow">{t("web.conversations.hero_eyebrow", "Conversations")}</span>
              <h1>{detail.conversation.title}</h1>
              <p>
                <span className="badge badge--accent">{detail.conversation.topology}</span>
                {" "}
                {detail.conversation.id}
              </p>
            </div>
            <div className="button-row">
              <button type="button" className="secondary" onClick={() => void hydrate()}>
                {t("web.action.refresh", "刷新")}
              </button>
            </div>
          </header>

          <section className={`session-status session-status--${sessionStatus.tone}`}>
            <strong>{sessionStatus.label}</strong>
            <span>{sessionStatus.detail}</span>
          </section>

          <div className="session-view__meta session-view__meta--chat">
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

          <div ref={scrollRef} className="message-stream message-stream--chat">
            {displayMessages.length === 0 ? (
              <div className="empty-card conversation-empty-card">
                {t("web.conversations.empty_messages", "还没有消息。在输入框用 @agent_id 指定某个 Agent。")}
              </div>
            ) : (
              displayMessages.map((message) => {
                const recipients = resolveRecipients(message.mentions);
                const isOperator = message.role === "operator";
                return (
                  <article
                    key={message.id}
                    className={isOperator ? "message-bubble message-bubble--operator" : "message-bubble"}
                  >
                    <header className="message-bubble__header">
                      <strong>{message.sender}</strong>
                      <small>{formatDateTime(message.createdAt)}</small>
                    </header>
                    <p>{message.body}</p>
                    {isOperator ? (
                      <footer className="message-bubble__footer">
                        <div className="message-route">
                          <span>{t("web.conversations.sent_to", "已发送给")}</span>
                          <div className="message-route__agents">
                            {recipients.map((agent) => (
                              <span key={agent.id} className="badge badge--muted">@{agent.display_name}</span>
                            ))}
                          </div>
                        </div>
                        {message.kind === "local" ? (
                          <div className="message-state">
                            <span className={`badge ${
                              message.status === "failed"
                                ? "badge--danger"
                                : message.status === "sending"
                                  ? "badge--accent"
                                  : "badge--warn"
                            }`}>
                              {message.status === "sending"
                                ? t("web.conversations.message_status_sending", "发送中")
                                : message.status === "queued"
                                  ? t("web.conversations.message_status_queued", "排队中")
                                  : t("web.conversations.message_status_failed", "发送失败")}
                            </span>
                            {message.status === "failed" ? (
                              <div className="button-row">
                                <button type="button" className="secondary" onClick={() => retryLocalMessage(message.id)}>
                                  {t("web.conversations.retry", "重试")}
                                </button>
                                <button type="button" className="secondary" onClick={() => removeLocalMessage(message.id)}>
                                  {t("web.conversations.remove", "移除")}
                                </button>
                              </div>
                            ) : null}
                          </div>
                        ) : (
                          <span className="message-delivery">
                            {t("web.conversations.message_status_delivered", "已送达")}
                          </span>
                        )}
                      </footer>
                    ) : null}
                    {message.kind === "local" && message.error ? (
                      <small className="message-error">{message.error}</small>
                    ) : null}
                  </article>
                );
              })
            )}
          </div>

          <div className="composer-shell">
            {queueItems.length > 0 ? (
              <section className="queue-panel">
                <div className="queue-panel__header">
                  <strong>{t("web.conversations.queue_title", "发送队列")}</strong>
                  <span>{t("web.conversations.queue_summary", "当前显示正在发送和排队中的消息。")}</span>
                </div>
                <div className="queue-list">
                  {queueItems.map((item, index) => {
                    const recipients = resolveRecipients(item.addressedAgents);
                    return (
                      <article key={item.clientId} className="queue-item">
                        <div className="queue-item__copy">
                          <strong>{index + 1}. {summarizeBody(item.body)}</strong>
                          <span>
                            {t("web.conversations.sent_to", "已发送给")}
                            {" "}
                            {recipients.map((agent) => `@${agent.display_name}`).join(" ")}
                          </span>
                        </div>
                        <div className="queue-item__actions">
                          <span className={`badge ${item.status === "sending" ? "badge--accent" : "badge--warn"}`}>
                            {item.status === "sending"
                              ? t("web.conversations.message_status_sending", "发送中")
                              : t("web.conversations.message_status_queued", "排队中")}
                          </span>
                          {item.status === "queued" ? (
                            <button type="button" className="secondary" onClick={() => removeLocalMessage(item.clientId)}>
                              {t("web.conversations.remove", "移除")}
                            </button>
                          ) : null}
                        </div>
                      </article>
                    );
                  })}
                </div>
              </section>
            ) : null}

            <form className="composer composer--chat" onSubmit={(event) => {
              event.preventDefault();
              enqueueCurrentMessage();
            }}>
              <div className="composer-editor-wrap">
                <div
                  ref={editorRef}
                  className="composer-editor"
                  contentEditable
                  suppressContentEditableWarning
                  role="textbox"
                  aria-multiline="true"
                  data-empty="true"
                  data-placeholder={t("web.conversations.composer_placeholder", "输入消息。使用 @coder / @planner 定向提问；不 @ 时默认投递给会话内 Agent。")}
                  onInput={syncComposerState}
                  onClick={syncComposerState}
                  onKeyUp={syncComposerState}
                  onKeyDown={handleComposerKeyDown}
                  onPaste={(event) => {
                    event.preventDefault();
                    const text = event.clipboardData.getData("text/plain");
                    document.execCommand("insertText", false, text);
                    syncComposerState();
                  }}
                />
                {mentionState.open && mentionOptions.length > 0 ? (
                  <div className="mention-picker">
                    {mentionOptions.map((agent, index) => (
                      <button
                        key={agent.id}
                        type="button"
                        className={index === mentionState.selectedIndex ? "mention-picker__item mention-picker__item--active" : "mention-picker__item"}
                        onMouseDown={(event) => {
                          event.preventDefault();
                          chooseMention(agent);
                        }}
                      >
                        <strong>@{agent.display_name}</strong>
                        <span>{agent.id}</span>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
              <div className="composer-actions">
                <small>
                  {canMention
                    ? t("web.conversations.mention_hint", "输入 @ 可选择当前会话中的 Agent。")
                    : t("web.conversations.mention_disabled", "当前只有 1 个 Agent，不显示 @ 候选。")}
                </small>
                <button type="submit" disabled={!composerSnapshot.body.trim()}>
                  {queueItems.length > 0
                    ? t("web.conversations.enqueue", "加入队列")
                    : t("web.conversations.send", "发送")}
                </button>
              </div>
            </form>
          </div>
        </>
      ) : (
        <div className="empty-card">{t("web.common.loading", "加载中…")}</div>
      )}
    </div>
  );
}
