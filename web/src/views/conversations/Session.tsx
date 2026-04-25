import { Fragment, useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent, type ReactNode } from "react";

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
  segments: ComposerSegment[];
  createdAt: string;
  status: LocalMessageStatus;
  error?: string;
};

type PendingReplyMarker = {
  id: string;
  agentId: string;
  createdAt: string;
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

type ComposerSegment =
  | {
      kind: "text";
      value: string;
    }
  | {
      kind: "mention";
      agentId: string;
      label: string;
    };

type ComposerSnapshot = {
  body: string;
  addressedAgents: string[];
  segments: ComposerSegment[];
};

const EMPTY_MENTION_STATE: MentionState = {
  open: false,
  query: "",
  selectedIndex: 0,
};

const OUTBOX_STORAGE_PREFIX = "ennoia.conversation.outbox.v1";
const PENDING_REPLY_STORAGE_PREFIX = "ennoia.conversation.pending-replies.v1";
const RECOVER_SENDING_AFTER_MS = 1500;
const MESSAGE_POLL_INTERVAL_MS = 2500;

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
    segments: snapshot.segments,
    createdAt: nowIso(),
    status: "queued",
  };
}

function createMentionNode(agentId: string, label: string) {
  const node = document.createElement("span");
  node.className = "composer-mention";
  node.contentEditable = "false";
  node.dataset.agentId = agentId;
  node.dataset.agentLabel = label;
  node.textContent = `@${label}`;
  return node;
}

function appendTextSegment(segments: ComposerSegment[], value: string) {
  if (!value) {
    return;
  }
  const normalized = value.replace(/\u00a0/g, " ");
  const last = segments[segments.length - 1];
  if (last?.kind === "text") {
    last.value += normalized;
    return;
  }
  segments.push({ kind: "text", value: normalized });
}

function readComposerSnapshot(root: HTMLElement | null): ComposerSnapshot {
  if (!root) {
    return { body: "", addressedAgents: [], segments: [] };
  }

  const addressedAgents: string[] = [];
  const segments: ComposerSegment[] = [];

  const walk = (node: Node) => {
    if (node.nodeType === Node.TEXT_NODE) {
      appendTextSegment(segments, node.textContent ?? "");
      return;
    }

    if (!(node instanceof HTMLElement)) {
      return;
    }

    if (node.dataset.agentId) {
      addressedAgents.push(node.dataset.agentId);
      segments.push({
        kind: "mention",
        agentId: node.dataset.agentId,
        label: node.dataset.agentLabel ?? node.dataset.agentId,
      });
      return;
    }

    if (node.tagName === "BR") {
      appendTextSegment(segments, "\n");
      return;
    }

    const isBlock = node !== root && ["DIV", "P"].includes(node.tagName);
    const last = segments[segments.length - 1];
    if (isBlock && last?.kind === "text" && !last.value.endsWith("\n")) {
      appendTextSegment(segments, "\n");
    }

    for (const child of [...node.childNodes]) {
      walk(child);
    }

    const tail = segments[segments.length - 1];
    if (isBlock && tail?.kind === "text" && !tail.value.endsWith("\n")) {
      appendTextSegment(segments, "\n");
    }
  };

  for (const child of [...root.childNodes]) {
    walk(child);
  }

  const body = segments
    .map((segment) => segment.kind === "text" ? segment.value : `@${segment.label}`)
    .join("")
    .replace(/\n{3,}/g, "\n\n")
    .trim();

  return {
    body,
    addressedAgents: uniqueStrings(addressedAgents),
    segments,
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

function appendTextNodes(root: HTMLElement, value: string) {
  const parts = value.split("\n");
  parts.forEach((part, index) => {
    if (part.length > 0) {
      root.appendChild(document.createTextNode(part));
    }
    if (index < parts.length - 1) {
      root.appendChild(document.createElement("br"));
    }
  });
}

function writeComposerSnapshot(root: HTMLElement | null, snapshot: ComposerSnapshot) {
  if (!root) {
    return;
  }

  root.innerHTML = "";
  for (const segment of snapshot.segments) {
    if (segment.kind === "text") {
      appendTextNodes(root, segment.value);
      continue;
    }
    root.appendChild(createMentionNode(segment.agentId, segment.label));
  }

  root.dataset.empty = String(snapshot.body.length === 0);
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

  const mentionNode = createMentionNode(agent.id, agent.display_name);
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

function outboxStorageKey(sessionId: string) {
  return `${OUTBOX_STORAGE_PREFIX}:${sessionId}`;
}

function pendingReplyStorageKey(sessionId: string) {
  return `${PENDING_REPLY_STORAGE_PREFIX}:${sessionId}`;
}

function loadPersistedDrafts(sessionId: string): LocalMessageDraft[] {
  if (typeof window === "undefined") {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(outboxStorageKey(sessionId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed as LocalMessageDraft[] : [];
  } catch {
    return [];
  }
}

function loadPersistedPendingReplies(sessionId: string): PendingReplyMarker[] {
  if (typeof window === "undefined") {
    return [];
  }
  try {
    const raw = window.localStorage.getItem(pendingReplyStorageKey(sessionId));
    if (!raw) {
      return [];
    }
    const parsed = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed as PendingReplyMarker[] : [];
  } catch {
    return [];
  }
}

function persistDrafts(sessionId: string, drafts: LocalMessageDraft[]) {
  if (typeof window === "undefined") {
    return;
  }
  try {
    if (drafts.length === 0) {
      window.localStorage.removeItem(outboxStorageKey(sessionId));
      return;
    }
    window.localStorage.setItem(outboxStorageKey(sessionId), JSON.stringify(drafts));
  } catch {
    return;
  }
}

function persistPendingReplies(sessionId: string, pendingReplies: PendingReplyMarker[]) {
  if (typeof window === "undefined") {
    return;
  }
  try {
    if (pendingReplies.length === 0) {
      window.localStorage.removeItem(pendingReplyStorageKey(sessionId));
      return;
    }
    window.localStorage.setItem(
      pendingReplyStorageKey(sessionId),
      JSON.stringify(pendingReplies),
    );
  } catch {
    return;
  }
}

function normalizeDraftBody(body: string) {
  return body.replace(/\s+/g, " ").trim();
}

function matchesRemoteMessage(draft: LocalMessageDraft, message: ChatMessage) {
  if (message.role !== "operator") {
    return false;
  }
  if (normalizeDraftBody(message.body) !== normalizeDraftBody(draft.body)) {
    return false;
  }
  const draftMentions = uniqueStrings(draft.addressedAgents).sort().join("|");
  const remoteMentions = uniqueStrings(message.mentions ?? []).sort().join("|");
  if (draftMentions !== remoteMentions) {
    return false;
  }
  return message.created_at >= draft.createdAt;
}

function reconcileDraftsWithRemote(
  drafts: LocalMessageDraft[],
  messages: ChatMessage[],
) {
  if (drafts.length === 0 || messages.length === 0) {
    return drafts;
  }

  const remainingMessages = [...messages];
  const nextDrafts: LocalMessageDraft[] = [];

  for (const draft of drafts) {
    const matchedIndex = remainingMessages.findIndex((message) =>
      matchesRemoteMessage(draft, message),
    );
    if (matchedIndex >= 0) {
      remainingMessages.splice(matchedIndex, 1);
      continue;
    }
    nextDrafts.push(draft);
  }

  return nextDrafts;
}

function reconcilePendingRepliesWithRemote(
  pendingReplies: PendingReplyMarker[],
  messages: ChatMessage[],
) {
  if (pendingReplies.length === 0 || messages.length === 0) {
    return pendingReplies;
  }

  return pendingReplies.filter((marker) =>
    !messages.some((message) =>
      message.role === "agent"
      && message.sender === marker.agentId
      && message.created_at >= marker.createdAt),
  );
}

function renderMessageBody(body: string, agents: AgentProfile[]): ReactNode {
  const mentionMap = new Map<string, string>();
  for (const agent of agents) {
    mentionMap.set(agent.id.toLowerCase(), agent.display_name);
    mentionMap.set(agent.display_name.toLowerCase(), agent.display_name);
    mentionMap.set(agent.display_name.toLowerCase().replace(/\s+/g, "-"), agent.display_name);
  }

  const lines = body.split("\n");
  return lines.map((line, lineIndex) => {
    const parts = line.split(/(@[\p{L}\p{N}_.-]+)/gu);
    return (
      <Fragment key={`line:${lineIndex}`}>
        {parts.map((part, partIndex) => {
          const match = part.match(/^@([\p{L}\p{N}_.-]+)$/u);
          if (!match) {
            return <Fragment key={`part:${lineIndex}:${partIndex}`}>{part}</Fragment>;
          }
          const label = mentionMap.get(match[1].toLowerCase());
          if (!label) {
            return <Fragment key={`part:${lineIndex}:${partIndex}`}>{part}</Fragment>;
          }
          return (
            <span key={`part:${lineIndex}:${partIndex}`} className="message-inline-mention">
              @{label}
            </span>
          );
        })}
        {lineIndex < lines.length - 1 ? <br /> : null}
      </Fragment>
    );
  });
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
  const [localDrafts, setLocalDrafts] = useState<LocalMessageDraft[]>(() => loadPersistedDrafts(sessionId));
  const [pendingReplies, setPendingReplies] = useState<PendingReplyMarker[]>(() => loadPersistedPendingReplies(sessionId));
  const [mentionState, setMentionState] = useState<MentionState>(EMPTY_MENTION_STATE);
  const [composerSnapshot, setComposerSnapshot] = useState<ComposerSnapshot>({ body: "", addressedAgents: [], segments: [] });
  const [error, setError] = useState<string | null>(null);
  const editorRef = useRef<HTMLDivElement | null>(null);
  const isMountedRef = useRef(true);
  const inFlightDraftIdRef = useRef<string | null>(null);

  const activeAgents = useMemo(() => {
    const ids = new Set(detail?.conversation?.participants?.filter((item) => item !== "operator") ?? []);
    return agents.filter((agent) => ids.has(agent.id));
  }, [agents, detail]);
  const conversation = detail?.conversation ?? null;

  const canMention = activeAgents.length > 1;

  const agentMap = useMemo(
    () => new Map(activeAgents.map((agent) => [agent.id, agent])),
    [activeAgents],
  );

  const refreshThread = useCallback(async () => {
    try {
      const nextDetail = await getChat(sessionId);
      if (!isMountedRef.current) {
        return;
      }
      setDetail(nextDetail);
    } catch (err) {
      if (err instanceof ApiError && err.status === 404 && panelId) {
        closeView(panelId);
        return;
      }
      if (isMountedRef.current) {
        setError(String(err));
      }
    }
  }, [closeView, panelId, sessionId]);

  const hydrate = useCallback(async () => {
    setError(null);
    setDetail(null);
    try {
      const [nextAgents, nextDetail] = await Promise.all([listAgents(), getChat(sessionId)]);
      if (!isMountedRef.current) {
        return;
      }
      setAgents(nextAgents);
      setDetail(nextDetail);
    } catch (err) {
      if (err instanceof ApiError && err.status === 404 && panelId) {
        closeView(panelId);
        return;
      }
      if (isMountedRef.current) {
        setError(String(err));
      }
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

  useEffect(() => {
    isMountedRef.current = true;
    return () => {
      isMountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    setLocalDrafts(loadPersistedDrafts(sessionId));
    setPendingReplies(loadPersistedPendingReplies(sessionId));
  }, [sessionId]);

  useEffect(() => {
    persistDrafts(sessionId, localDrafts);
  }, [localDrafts, sessionId]);

  useEffect(() => {
    persistPendingReplies(sessionId, pendingReplies);
  }, [pendingReplies, sessionId]);

  useEffect(() => {
    if (!detail?.messages) {
      return;
    }
    setLocalDrafts((current) => reconcileDraftsWithRemote(current, detail.messages));
    setPendingReplies((current) => reconcilePendingRepliesWithRemote(current, detail.messages));
  }, [detail?.messages]);

  useEffect(() => {
    if (!conversation) {
      return;
    }
    const timer = window.setInterval(() => {
      void refreshThread();
    }, MESSAGE_POLL_INTERVAL_MS);
    return () => window.clearInterval(timer);
  }, [conversation, refreshThread]);

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
      if (composerSnapshot.addressedAgents.includes(agent.id)) {
        return false;
      }
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
  }, [activeAgents, canMention, composerSnapshot.addressedAgents, mentionState.query]);

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

  const waitingItems = useMemo(
    () => localDrafts.filter((item) => item.status === "queued"),
    [localDrafts],
  );

  const failedItems = useMemo(
    () => localDrafts.filter((item) => item.status === "failed"),
    [localDrafts],
  );

  const sendingItem = useMemo(
    () => localDrafts.find((item) => item.status === "sending") ?? null,
    [localDrafts],
  );

  useEffect(() => {
    if (!sendingItem || inFlightDraftIdRef.current) {
      return;
    }
    const timer = window.setTimeout(() => {
      setLocalDrafts((current) =>
        current.map((item) =>
          item.clientId === sendingItem.clientId && item.status === "sending"
            ? { ...item, status: "queued" }
            : item),
      );
    }, RECOVER_SENDING_AFTER_MS);
    return () => window.clearTimeout(timer);
  }, [sendingItem]);

  const sessionStatus = useMemo(() => {
    if (sendingItem) {
      return {
        tone: "accent",
        label: t("web.conversations.status_sending", "正在发送消息…"),
        detail: t("web.conversations.status_sending_detail", "当前消息已进入处理链路，请稍候。"),
      };
    }
    if (pendingReplies.length > 0) {
      return {
        tone: "accent",
        label: t("web.conversations.status_ai_typing", "AI 正在输入…"),
        detail: t("web.conversations.status_ai_typing_detail", "已发送给 Agent，正在等待回复写回会话。"),
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
  }, [failedItems.length, pendingReplies.length, sendingItem, t, waitingItems.length]);

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

  const typingAgents = useMemo(() => {
    const typingIds = new Set<string>();
    if (sendingItem) {
      for (const agent of resolveRecipients(sendingItem.addressedAgents)) {
        typingIds.add(agent.id);
      }
    }
    for (const marker of pendingReplies) {
      typingIds.add(marker.agentId);
    }
    return [...typingIds]
      .map((agentId) => agentMap.get(agentId))
      .filter((agent): agent is AgentProfile => Boolean(agent));
  }, [agentMap, pendingReplies, resolveRecipients, sendingItem]);

  const resetComposer = useCallback(() => {
    clearComposer(editorRef.current);
    setComposerSnapshot({ body: "", addressedAgents: [], segments: [] });
    setMentionState(EMPTY_MENTION_STATE);
    focusComposerEnd(editorRef.current);
  }, []);

  const restoreDraftToComposer = useCallback((draft: LocalMessageDraft) => {
    writeComposerSnapshot(editorRef.current, {
      body: draft.body,
      addressedAgents: draft.addressedAgents,
      segments: draft.segments,
    });
    syncComposerState();
    focusComposerEnd(editorRef.current);
  }, [syncComposerState]);

  const enqueueCurrentMessage = useCallback(() => {
    if (!conversation) {
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
      segments: snapshot.segments,
    });
    setLocalDrafts((current) => [...current, queued]);
    resetComposer();
  }, [activeAgents, conversation, resetComposer]);

  useEffect(() => {
    if (!conversation || sendingItem || inFlightDraftIdRef.current) {
      return;
    }

    const next = waitingItems[0];
    if (!next) {
      return;
    }

    inFlightDraftIdRef.current = next.clientId;
    setLocalDrafts((current) =>
      current.map((item) => item.clientId === next.clientId ? { ...item, status: "sending", error: undefined } : item),
    );

    void (async () => {
      try {
        const response = await sendChatMessage(conversation.id, {
          lane_id: conversation.default_lane_id ?? undefined,
          body: next.body,
          addressed_agents: next.addressedAgents,
        });
        if (!isMountedRef.current) {
          return;
        }
        setLocalDrafts((current) => current.filter((item) => item.clientId !== next.clientId));
        setPendingReplies((current) => [
          ...current,
          ...next.addressedAgents.map((agentId) => ({
            id: `${response.message.id}:${agentId}`,
            agentId,
            createdAt: response.message.created_at,
          })),
        ]);
        setDetail((current) => {
          if (!current) {
            return current;
          }
          const nextLane = response.lane;
          const nextMessages = current.messages.some((message) => message.id === response.message.id)
            ? current.messages
            : [...current.messages, response.message].sort((left, right) =>
              left.created_at.localeCompare(right.created_at));
          return {
            ...current,
            conversation: response.conversation,
            lanes: current.lanes.some((lane) => lane.id === nextLane.id)
              ? current.lanes.map((lane) => lane.id === nextLane.id ? nextLane : lane)
              : [...current.lanes, nextLane],
            messages: nextMessages,
          };
        });
        notifyChanged();
        void refreshThread();
      } catch (err) {
        if (isMountedRef.current) {
          setLocalDrafts((current) =>
            current.map((item) => item.clientId === next.clientId
              ? { ...item, status: "failed", error: String(err) }
              : item),
          );
          setError(String(err));
        }
      } finally {
        if (inFlightDraftIdRef.current === next.clientId) {
          inFlightDraftIdRef.current = null;
        }
      }
    })();
  }, [
    conversation,
    notifyChanged,
    refreshThread,
    sendingItem,
    waitingItems,
  ]);

  const retryLocalMessage = useCallback((clientId: string) => {
    setLocalDrafts((current) =>
      current.map((item) => item.clientId === clientId ? { ...item, status: "queued", error: undefined } : item),
    );
  }, []);

  const removeLocalMessage = useCallback((clientId: string) => {
    setLocalDrafts((current) => current.filter((item) => item.clientId !== clientId));
  }, []);

  const editQueuedMessage = useCallback((clientId: string) => {
    const target = localDrafts.find((item) => item.clientId === clientId && item.status === "queued");
    if (!target) {
      return;
    }
    restoreDraftToComposer(target);
    setLocalDrafts((current) => current.filter((item) => item.clientId !== clientId));
  }, [localDrafts, restoreDraftToComposer]);

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
  }, [displayMessages.length, typingAgents.length, waitingItems.length]);

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
                    <p>{renderMessageBody(message.body, recipients)}</p>
                    {isOperator ? (
                      <footer className="message-bubble__footer">
                        <div className="message-route">
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
            {typingAgents.map((agent) => (
              <article key={`typing:${agent.id}`} className="message-bubble message-bubble--typing">
                <header className="message-bubble__header">
                  <strong>{agent.display_name}</strong>
                  <small>{t("web.conversations.ai_typing", "输入中")}</small>
                </header>
                <div className="typing-indicator" aria-label={t("web.conversations.ai_typing", "输入中")}>
                  <span />
                  <span />
                  <span />
                </div>
              </article>
            ))}
          </div>

          <div className="composer-shell">
            {waitingItems.length > 0 ? (
              <section className="queue-panel">
                <div className="queue-panel__header">
                  <strong>{t("web.conversations.queue_title", "发送队列")}</strong>
                  <span>{t("web.conversations.queue_summary", "这里只显示还在排队、尚未开始发送的消息。")}</span>
                </div>
                <div className="queue-list">
                  {waitingItems.map((item, index) => {
                    const recipients = resolveRecipients(item.addressedAgents);
                    return (
                      <article key={item.clientId} className="queue-item">
                        <div className="queue-item__copy">
                          <strong>{index + 1}. {summarizeBody(item.body)}</strong>
                          <span>
                            {recipients.map((agent) => `@${agent.display_name}`).join(" ")}
                          </span>
                        </div>
                        <div className="queue-item__actions">
                          <span className="badge badge--warn">{t("web.conversations.message_status_queued", "排队中")}</span>
                          <div className="button-row">
                            <button type="button" className="secondary" onClick={() => editQueuedMessage(item.clientId)}>
                              {t("web.action.edit", "编辑")}
                            </button>
                            <button type="button" className="secondary" onClick={() => removeLocalMessage(item.clientId)}>
                              {t("web.conversations.remove", "移除")}
                            </button>
                          </div>
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
                  {waitingItems.length > 0 || Boolean(sendingItem)
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
