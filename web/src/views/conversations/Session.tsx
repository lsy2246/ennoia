import { useCallback, useEffect, useMemo, useRef, useState, type KeyboardEvent } from "react";

import {
  ApiError,
  getChat,
  listAgents,
  listSkills,
  sendChatMessage,
  type AgentProfile,
  type ChatMessage,
  type ChatThreadDetail,
  type SkillConfig,
} from "@ennoia/api-client";
import { useConversationsStore } from "@/stores/conversations";
import { useUiHelpers } from "@/stores/ui";
import { useWorkbenchStore } from "@/stores/workbench";
import { ChatStream } from "./ChatStream";
import { buildChatEntries, buildStatusEntries } from "./chat-entry-builder";
import type {
  ComposerSegment,
  LocalMessageDraft,
  PendingReplyMarker,
} from "./chat-types";

type ComposerPickerMode = "mention" | "skill";

type ComposerPickerState = {
  open: boolean;
  mode: ComposerPickerMode;
  query: string;
  selectedIndex: number;
};

type ComposerPickerOption = {
  kind: ComposerPickerMode;
  id: string;
  displayLabel: string;
  insertLabel: string;
  secondaryLabel: string;
};

type ComposerSnapshot = {
  body: string;
  addressedAgents: string[];
  explicitMentions?: string[];
  segments: ComposerSegment[];
};

const EMPTY_PICKER_STATE: ComposerPickerState = {
  open: false,
  mode: "mention",
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
    explicitMentions: snapshot.explicitMentions ?? snapshot.addressedAgents,
    segments: snapshot.segments,
    createdAt: nowIso(),
    status: "queued",
  };
}

function createComposerTokenNode(
  kind: ComposerPickerMode,
  id: string,
  displayLabel: string,
  insertLabel: string,
) {
  const node = document.createElement("span");
  node.className = kind === "mention" ? "composer-mention" : "composer-skill";
  node.contentEditable = "false";
  node.dataset.tokenKind = kind;
  node.dataset.tokenId = id;
  node.dataset.tokenLabel = insertLabel;
  node.dataset.tokenDisplayLabel = displayLabel;
  node.textContent = `${kind === "mention" ? "@" : "/"}${displayLabel}`;
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

    if (node.dataset.tokenKind === "mention") {
      const agentId = node.dataset.tokenId ?? "";
      if (!agentId) {
        return;
      }
      addressedAgents.push(agentId);
      segments.push({
        kind: "mention",
        agentId,
        label: node.dataset.tokenDisplayLabel ?? node.dataset.tokenLabel ?? agentId,
      });
      return;
    }

    if (node.dataset.tokenKind === "skill") {
      const skillId = node.dataset.tokenLabel ?? node.dataset.tokenId ?? "";
      if (!skillId) {
        return;
      }
      segments.push({
        kind: "skill",
        skillId,
        label: node.dataset.tokenDisplayLabel ?? skillId,
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
    .map((segment) => {
      if (segment.kind === "text") {
        return segment.value;
      }
      if (segment.kind === "mention") {
        return `@${segment.label}`;
      }
      return `/${segment.skillId}`;
    })
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
    if (segment.kind === "mention") {
      root.appendChild(createComposerTokenNode("mention", segment.agentId, segment.label, segment.label));
      continue;
    }
    root.appendChild(createComposerTokenNode("skill", segment.skillId, segment.label, segment.skillId));
  }

  root.dataset.empty = String(snapshot.body.length === 0);
}

function isSelectionInside(root: HTMLElement | null, node: Node | null) {
  if (!root || !node) {
    return false;
  }
  return root === node || root.contains(node);
}

function extractComposerTrigger(root: HTMLElement | null) {
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
  const match = before.match(/(?:^|\s)([@/])([\p{L}\p{N}_.-]*)$/u);
  if (!match) {
    return null;
  }

  const trigger = match[1] ?? "";
  const query = match[2] ?? "";
  const atIndex = before.lastIndexOf(trigger);
  if (atIndex < 0) {
    return null;
  }

  return {
    textNode: anchorNode,
    kind: trigger === "@" ? "mention" : "skill",
    trigger,
    atIndex,
    offset,
    query,
  };
}

function replaceComposerTriggerAtCaret(root: HTMLElement | null, option: ComposerPickerOption) {
  const context = extractComposerTrigger(root);
  if (!root || !context || typeof window === "undefined") {
    return false;
  }

  const original = context.textNode.textContent ?? "";
  const before = original.slice(0, context.atIndex);
  const after = original.slice(context.offset);
  context.textNode.textContent = before;

  const tokenNode = createComposerTokenNode(
    option.kind,
    option.id,
    option.displayLabel,
    option.insertLabel,
  );
  const trailingText = document.createTextNode(after.startsWith(" ") ? after : ` ${after}`);
  const parent = context.textNode.parentNode;
  if (!parent) {
    return false;
  }

  parent.insertBefore(tokenNode, context.textNode.nextSibling);
  parent.insertBefore(trailingText, tokenNode.nextSibling);

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

function handleComposerTokenBackspace(root: HTMLElement | null) {
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
    if (previous instanceof HTMLElement && previous.dataset.tokenKind) {
      previous.remove();
      root.dataset.empty = String(readComposerSnapshot(root).body.length === 0);
      return true;
    }
  }

  if (anchorNode instanceof HTMLElement && selection.anchorOffset > 0) {
    const previous = anchorNode.childNodes[selection.anchorOffset - 1];
    if (previous instanceof HTMLElement && previous.dataset.tokenKind) {
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
  const draftMentions = uniqueStrings(draft.explicitMentions ?? []).sort().join("|");
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

export function SessionView({ sessionId, panelId }: { sessionId: string; panelId?: string }) {
  const { formatDateTime, t } = useUiHelpers();
  const openView = useWorkbenchStore((state) => state.openView);
  const closeView = useWorkbenchStore((state) => state.closeView);
  const conversationRevision = useConversationsStore((state) => state.revision);
  const deletedSessionMark = useConversationsStore((state) => state.deletedSessionMarks[sessionId]);
  const notifyChanged = useConversationsStore((state) => state.notifyChanged);
  const [agents, setAgents] = useState<AgentProfile[]>([]);
  const [skills, setSkills] = useState<SkillConfig[]>([]);
  const [detail, setDetail] = useState<ChatThreadDetail | null>(null);
  const [localDrafts, setLocalDrafts] = useState<LocalMessageDraft[]>(() => loadPersistedDrafts(sessionId));
  const [pendingReplies, setPendingReplies] = useState<PendingReplyMarker[]>(() => loadPersistedPendingReplies(sessionId));
  const [pickerState, setPickerState] = useState<ComposerPickerState>(EMPTY_PICKER_STATE);
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
  const enabledSkills = useMemo(
    () => skills
      .filter((skill) => skill.enabled)
      .sort((left, right) => left.display_name.localeCompare(right.display_name)),
    [skills],
  );
  const canUseSkills = enabledSkills.length > 0;

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
      const [nextAgents, nextSkills, nextDetail] = await Promise.all([
        listAgents(),
        listSkills(),
        getChat(sessionId),
      ]);
      if (!isMountedRef.current) {
        return;
      }
      setAgents(nextAgents);
      setSkills(nextSkills);
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
    const context = extractComposerTrigger(editorRef.current);
    if (!context) {
      setPickerState(EMPTY_PICKER_STATE);
      return;
    }
    if (context.kind === "mention" && !canMention) {
      setPickerState(EMPTY_PICKER_STATE);
      return;
    }
    if (context.kind === "skill" && !canUseSkills) {
      setPickerState(EMPTY_PICKER_STATE);
      return;
    }
    const mode = context.kind as ComposerPickerMode;
    setPickerState((current) => ({
      open: true,
      mode,
      query: context.query,
      selectedIndex: current.mode === mode ? current.selectedIndex : 0,
    }));
  }, [canMention, canUseSkills]);

  useEffect(() => {
    syncComposerState();
  }, [syncComposerState]);

  const pickerOptions = useMemo<ComposerPickerOption[]>(() => {
    if (!pickerState.open) {
      return [];
    }
    const query = pickerState.query.trim().toLowerCase();
    if (pickerState.mode === "mention") {
      if (!canMention) {
        return [];
      }
      return activeAgents
        .filter((agent) => {
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
        })
        .map<ComposerPickerOption>((agent) => ({
          kind: "mention",
          id: agent.id,
          displayLabel: agent.display_name,
          insertLabel: agent.display_name,
          secondaryLabel: agent.id,
        }));
    }

    if (!canUseSkills) {
      return [];
    }

    const selectedSkillIds = new Set(
      composerSnapshot.segments
        .filter((segment): segment is Extract<ComposerSegment, { kind: "skill" }> => segment.kind === "skill")
        .map((segment) => segment.skillId),
    );

    return enabledSkills
      .filter((skill) => {
        if (selectedSkillIds.has(skill.id)) {
          return false;
        }
        if (!query) {
          return true;
        }
        const haystacks = [
          skill.id.toLowerCase(),
          skill.display_name.toLowerCase(),
          skill.display_name.toLowerCase().replace(/\s+/g, "-"),
          ...skill.tags.map((tag) => tag.toLowerCase()),
        ];
        return haystacks.some((item) => item.includes(query));
      })
      .map<ComposerPickerOption>((skill) => ({
        kind: "skill",
        id: skill.id,
        displayLabel: skill.display_name,
        insertLabel: skill.id,
        secondaryLabel: skill.id,
      }));
  }, [
    activeAgents,
    canMention,
    canUseSkills,
    composerSnapshot.addressedAgents,
    composerSnapshot.segments,
    enabledSkills,
    pickerState.mode,
    pickerState.open,
    pickerState.query,
  ]);

  useEffect(() => {
    if (!pickerState.open) {
      return;
    }
    if (pickerOptions.length === 0) {
      setPickerState((current) => ({ ...current, selectedIndex: 0 }));
      return;
    }
    if (pickerState.selectedIndex >= pickerOptions.length) {
      setPickerState((current) => ({ ...current, selectedIndex: 0 }));
    }
  }, [pickerOptions.length, pickerState.open, pickerState.selectedIndex]);

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
    for (const marker of pendingReplies) {
      typingIds.add(marker.agentId);
    }
    return [...typingIds]
      .map((agentId) => agentMap.get(agentId))
      .filter((agent): agent is AgentProfile => Boolean(agent));
  }, [agentMap, pendingReplies]);

  const chatEntries = useMemo(() => buildChatEntries({
    messages: detail?.messages ?? [],
    localDrafts,
    resolveRecipients,
  }), [detail?.messages, localDrafts, resolveRecipients]);

  const statusEntries = useMemo(() => buildStatusEntries({
    typingAgents,
    pendingCreatedAt: pendingReplies[0]?.createdAt,
    texts: {
      typingLabel: t("web.conversations.status_ai_typing", "AI 正在输入…"),
      typingDetail: t("web.conversations.status_ai_typing_detail", "已发送给 Agent，正在等待回复写回会话。"),
    },
  }), [pendingReplies, t, typingAgents]);

  const streamEntries = useMemo(
    () => [...chatEntries, ...statusEntries],
    [chatEntries, statusEntries],
  );

  const composerStatus = useMemo(() => {
    if (sendingItem) {
      return {
        tone: "accent" as const,
        label: t("web.conversations.status_sending", "正在发送消息…"),
        detail: t("web.conversations.status_sending_detail", "当前消息已进入处理链路，请稍候。"),
      };
    }
    if (waitingItems.length > 0) {
      return {
        tone: "warn" as const,
        label: t("web.conversations.status_queue", "排队中"),
        detail: t("web.conversations.status_queue_detail", "还有 {count} 条消息等待发送。").replace("{count}", String(waitingItems.length)),
      };
    }
    if (failedItems.length > 0) {
      return {
        tone: "danger" as const,
        label: t("web.conversations.status_failed", "有消息发送失败"),
        detail: t("web.conversations.status_failed_detail", "你可以重试失败消息，或从队列中移除它。"),
      };
    }
    return null;
  }, [failedItems.length, sendingItem, t, waitingItems.length]);

  const composerPlaceholder = useMemo(() => {
    if (canMention && canUseSkills) {
      return t(
        "web.conversations.composer_placeholder_with_skill",
        "输入消息。使用 @ 选择 Agent，使用 / 选择技能；不 @ 时默认投递给会话内 Agent。",
      );
    }
    if (canUseSkills) {
      return t(
        "web.conversations.composer_placeholder_skill_only",
        "输入消息。使用 / 选择技能；消息默认投递给当前会话内 Agent。",
      );
    }
    return t(
      "web.conversations.composer_placeholder",
      "输入消息。使用 @coder / @planner 定向提问；不 @ 时默认投递给会话内 Agent。",
    );
  }, [canMention, canUseSkills, t]);

  const composerHint = useMemo(() => {
    if (canMention && canUseSkills) {
      return t(
        "web.conversations.mention_and_skill_hint",
        "输入 @ 可选择会话内 Agent，输入 / 可选择技能。",
      );
    }
    if (canMention) {
      return t("web.conversations.mention_hint", "输入 @ 可选择当前会话中的 Agent。");
    }
    if (canUseSkills) {
      return t("web.conversations.skill_hint", "输入 / 可选择当前可用的技能。");
    }
    return t("web.conversations.skill_disabled", "当前没有可用技能候选。");
  }, [canMention, canUseSkills, t]);

  const resetComposer = useCallback(() => {
    clearComposer(editorRef.current);
    setComposerSnapshot({ body: "", addressedAgents: [], segments: [] });
    setPickerState(EMPTY_PICKER_STATE);
    focusComposerEnd(editorRef.current);
  }, []);

  const restoreDraftToComposer = useCallback((draft: LocalMessageDraft) => {
    writeComposerSnapshot(editorRef.current, {
      body: draft.body,
      addressedAgents: draft.explicitMentions,
      explicitMentions: draft.explicitMentions,
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
      explicitMentions: snapshot.addressedAgents,
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
          mentions: next.explicitMentions,
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

  const choosePickerOption = useCallback((option: ComposerPickerOption) => {
    if (replaceComposerTriggerAtCaret(editorRef.current, option)) {
      syncComposerState();
    }
    setPickerState(EMPTY_PICKER_STATE);
  }, [syncComposerState]);

  const handleComposerKeyDown = useCallback((event: KeyboardEvent<HTMLDivElement>) => {
    if (pickerState.open && pickerOptions.length > 0) {
      if (event.key === "ArrowDown") {
        event.preventDefault();
        setPickerState((current) => ({
          ...current,
          selectedIndex: (current.selectedIndex + 1) % pickerOptions.length,
        }));
        return;
      }
      if (event.key === "ArrowUp") {
        event.preventDefault();
        setPickerState((current) => ({
          ...current,
          selectedIndex: (current.selectedIndex - 1 + pickerOptions.length) % pickerOptions.length,
        }));
        return;
      }
      if (event.key === "Enter" || event.key === "Tab") {
        event.preventDefault();
        choosePickerOption(pickerOptions[pickerState.selectedIndex] ?? pickerOptions[0]);
        return;
      }
    }

    if (pickerState.open && event.key === "Escape") {
      event.preventDefault();
      setPickerState(EMPTY_PICKER_STATE);
      return;
    }

    if (event.key === "Backspace" && handleComposerTokenBackspace(editorRef.current)) {
      event.preventDefault();
      syncComposerState();
      return;
    }

    if (event.key === "Enter" && !event.shiftKey) {
      event.preventDefault();
      enqueueCurrentMessage();
    }
  }, [
    choosePickerOption,
    enqueueCurrentMessage,
    pickerOptions,
    pickerState.open,
    pickerState.selectedIndex,
    syncComposerState,
  ]);

  const scrollRef = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const node = scrollRef.current;
    if (!node) {
      return;
    }
    node.scrollTop = node.scrollHeight;
  }, [chatEntries.length, statusEntries.length]);

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
            <ChatStream
              entries={streamEntries}
              agents={activeAgents}
              skills={skills}
              emptyMessage={t("web.conversations.empty_messages", "还没有消息。在输入框用 @agent_id 指定某个 Agent。")}
              formatDateTime={formatDateTime}
              t={t}
              onRetry={retryLocalMessage}
              onRemove={removeLocalMessage}
            />
          </div>

          <div className="composer-shell">
            {composerStatus ? (
              <section className={`composer-status composer-status--${composerStatus.tone}`}>
                <strong>{composerStatus.label}</strong>
                <span>{composerStatus.detail}</span>
              </section>
            ) : null}

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
                  data-placeholder={composerPlaceholder}
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
                {pickerState.open && pickerOptions.length > 0 ? (
                  <div className="mention-picker">
                    {pickerOptions.map((option, index) => (
                      <button
                        key={`${option.kind}:${option.id}`}
                        type="button"
                        className={index === pickerState.selectedIndex ? "mention-picker__item mention-picker__item--active" : "mention-picker__item"}
                        onMouseDown={(event) => {
                          event.preventDefault();
                          choosePickerOption(option);
                        }}
                      >
                        <strong>{option.kind === "mention" ? "@" : "/"}{option.displayLabel}</strong>
                        <span>{option.secondaryLabel}</span>
                      </button>
                    ))}
                  </div>
                ) : null}
              </div>
              <div className="composer-actions">
                <small>{composerHint}</small>
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
