import { dispatchAction } from "./actions";
import type {
  ConversationBranch,
  ConversationLane,
  ConversationMessage,
  ConversationMessageAppendResponse,
  ConversationSummary,
  ConversationDetail,
  ConversationStreamSnapshot,
  ExecutionRun,
  ExecutionStep,
  PermissionApprovalRecord,
  RunOutput,
} from "./types";
import { listConversationPermissionApprovals } from "./permissions";

const CONVERSATION_STREAM_POLL_MS = 1000;

type ConversationDetailPayload = {
  conversation: ConversationSummary;
  lanes?: ConversationLane[];
  branches?: ConversationBranch[];
  messages?: ConversationMessage[];
  runs?: ExecutionRun[];
  tasks?: ExecutionStep[];
  outputs?: RunOutput[];
};

type ConversationStreamListener = (event: Event) => void;

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null;
}

function isConversationSummary(value: unknown): value is ConversationSummary {
  return isRecord(value)
    && typeof value.id === "string"
    && (value.topology === "direct" || value.topology === "group")
    && typeof value.title === "string";
}

function normalizeConversationDetailPayload(payload: unknown): ConversationDetailPayload {
  if (isConversationSummary(payload)) {
    return { conversation: payload };
  }

  if (!isRecord(payload) || !isConversationSummary(payload.conversation)) {
    throw new Error("invalid conversation detail payload");
  }

  return {
    conversation: payload.conversation,
    lanes: Array.isArray(payload.lanes) ? payload.lanes as ConversationLane[] : undefined,
    branches: Array.isArray(payload.branches) ? payload.branches as ConversationBranch[] : undefined,
    messages: Array.isArray(payload.messages) ? payload.messages as ConversationMessage[] : undefined,
    runs: Array.isArray(payload.runs) ? payload.runs as ExecutionRun[] : undefined,
    tasks: Array.isArray(payload.tasks) ? payload.tasks as ExecutionStep[] : undefined,
    outputs: Array.isArray(payload.outputs) ? payload.outputs as RunOutput[] : undefined,
  };
}

async function listConversationRuns(conversationId: string) {
  try {
    return await dispatchAction<ExecutionRun[]>("run.list", {
      conversation_id: conversationId,
      limit: 24,
    });
  } catch {
    return [];
  }
}

async function hydrateConversationDetail(
  conversationId: string,
  payload: unknown,
): Promise<ConversationDetail> {
  const normalized = normalizeConversationDetailPayload(payload);
  const [lanes, branches, messages, runs] = await Promise.all([
    normalized.lanes
      ? Promise.resolve(normalized.lanes)
      : dispatchAction<ConversationLane[]>("lane.list", { conversation_id: conversationId }),
    normalized.branches
      ? Promise.resolve(normalized.branches)
      : dispatchAction<ConversationBranch[]>("branch.list", { conversation_id: conversationId }),
    normalized.messages
      ? Promise.resolve(normalized.messages)
      : dispatchAction<ConversationMessage[]>("message.list", { conversation_id: conversationId }),
    normalized.runs ? Promise.resolve(normalized.runs) : listConversationRuns(conversationId),
  ]);

  return {
    conversation: normalized.conversation,
    lanes,
    branches,
    messages,
    runs,
    tasks: normalized.tasks ?? [],
    outputs: normalized.outputs ?? [],
  };
}

async function loadConversationSnapshot(
  conversationId: string,
): Promise<ConversationStreamSnapshot> {
  const [detail, approvals] = await Promise.all([
    getConversation(conversationId),
    listConversationPermissionApprovals(conversationId, { limit: 80 }),
  ]);
  return { detail, approvals };
}

function createMessageEvent(type: string, data: string) {
  if (typeof MessageEvent !== "undefined") {
    return new MessageEvent(type, { data });
  }
  return new Event(type);
}

class PollingConversationStream {
  private readonly listeners = new Map<string, Set<ConversationStreamListener>>();
  private closed = false;
  private timer: number | null = null;
  private opened = false;

  onopen: ((event: Event) => void) | null = null;
  onerror: ((event: Event) => void) | null = null;

  constructor(private readonly conversationId: string) {
    void this.poll();
  }

  addEventListener(type: string, listener: ConversationStreamListener) {
    const bucket = this.listeners.get(type) ?? new Set<ConversationStreamListener>();
    bucket.add(listener);
    this.listeners.set(type, bucket);
  }

  removeEventListener(type: string, listener: ConversationStreamListener) {
    const bucket = this.listeners.get(type);
    if (!bucket) {
      return;
    }
    bucket.delete(listener);
    if (bucket.size === 0) {
      this.listeners.delete(type);
    }
  }

  close() {
    this.closed = true;
    if (this.timer !== null && typeof window !== "undefined") {
      window.clearTimeout(this.timer);
      this.timer = null;
    }
  }

  private emit(type: string, event: Event) {
    const bucket = this.listeners.get(type);
    if (!bucket) {
      return;
    }
    for (const listener of bucket) {
      listener(event);
    }
  }

  private scheduleNextPoll() {
    if (this.closed || typeof window === "undefined") {
      return;
    }
    this.timer = window.setTimeout(() => {
      void this.poll();
    }, CONVERSATION_STREAM_POLL_MS);
  }

  private async poll() {
    if (this.closed) {
      return;
    }

    try {
      const snapshot = await loadConversationSnapshot(this.conversationId);
      const payload = JSON.stringify(snapshot);
      if (!this.opened) {
        this.opened = true;
        this.onopen?.(new Event("open"));
      }
      this.emit(
        "conversation.snapshot",
        createMessageEvent("conversation.snapshot", payload),
      );
    } catch (error) {
      const payload = JSON.stringify({
        message: String(error),
      });
      const event = createMessageEvent("conversation.error", payload);
      this.emit("conversation.error", event);
      this.onerror?.(event);
    } finally {
      this.scheduleNextPoll();
    }
  }
}

export async function listConversations() {
  return dispatchAction<ConversationSummary[]>("conversation.list");
}

export async function createConversation(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return dispatchAction<{ conversation: ConversationSummary; default_lane: ConversationLane }>(
    "conversation.create",
    payload,
  );
}

export async function deleteConversation(conversationId: string) {
  await dispatchAction("conversation.delete", { conversation_id: conversationId });
}

export async function getConversation(conversationId: string): Promise<ConversationDetail> {
  const detail = await dispatchAction<unknown>("conversation.get", {
    conversation_id: conversationId,
  });
  return hydrateConversationDetail(conversationId, detail);
}

export async function appendConversationMessage(
  conversationId: string,
  payload: {
    lane_id?: string;
    branch_id?: string;
    body: string;
    goal?: string;
    addressed_agents?: string[];
    mentions?: string[];
    parent_message_id?: string;
    fork_from_message_id?: string;
    rewrite_from_message_id?: string;
    branch_name?: string;
  },
) {
  return dispatchAction<ConversationMessageAppendResponse>("message.append", {
    conversation_id: conversationId,
    message: {
      ...payload,
      role: "operator",
      sender: "operator",
    },
  });
}

export async function listConversationLanes(conversationId: string) {
  return dispatchAction<ConversationLane[]>("lane.list", {
    conversation_id: conversationId,
  });
}

export async function listConversationBranches(conversationId: string) {
  return dispatchAction<ConversationBranch[]>("branch.list", {
    conversation_id: conversationId,
  });
}

export async function createConversationBranch(
  conversationId: string,
  payload: {
    from_branch_id?: string;
    source_message_id?: string;
    name?: string;
    mode?: "fork" | "rewrite";
    activate?: boolean;
  },
) {
  return dispatchAction<ConversationBranch>("branch.create", {
    conversation_id: conversationId,
    ...payload,
  });
}

export async function switchConversationBranch(conversationId: string, branchId: string) {
  await dispatchAction("branch.switch", {
    conversation_id: conversationId,
    branch_id: branchId,
  });
  return getConversation(conversationId);
}

export async function updateConversationBranch(
  conversationId: string,
  branchId: string,
  payload: {
    name: string;
  },
) {
  return dispatchAction<ConversationBranch>("branch.update", {
    conversation_id: conversationId,
    branch_id: branchId,
    ...payload,
  });
}

export async function deleteConversationBranch(
  conversationId: string,
  branchId: string,
  payload: {
    mode: "detach_children" | "delete_tree";
  },
) {
  await dispatchAction("branch.delete", {
    conversation_id: conversationId,
    branch_id: branchId,
    ...payload,
  });
  return getConversation(conversationId);
}

export function createConversationStream(conversationId: string) {
  return new PollingConversationStream(conversationId);
}

export function parseConversationStreamPayload(value: string): ConversationStreamSnapshot {
  const parsed = JSON.parse(value) as {
    detail?: unknown;
    approvals?: unknown;
  };
  const detailValue = parsed.detail;
  const normalized = normalizeConversationDetailPayload(detailValue);
  const detailRecord = isRecord(detailValue) ? detailValue : null;

  return {
    detail: {
      conversation: normalized.conversation,
      lanes: normalized.lanes ?? [],
      branches: normalized.branches ?? [],
      messages: normalized.messages ?? [],
      runs: Array.isArray(detailRecord?.runs) ? detailRecord.runs as ConversationDetail["runs"] : [],
      tasks: Array.isArray(detailRecord?.tasks) ? detailRecord.tasks as ConversationDetail["tasks"] : [],
      outputs: Array.isArray(detailRecord?.outputs) ? detailRecord.outputs as ConversationDetail["outputs"] : [],
    },
    approvals: Array.isArray(parsed.approvals)
      ? parsed.approvals as PermissionApprovalRecord[]
      : [],
  };
}
