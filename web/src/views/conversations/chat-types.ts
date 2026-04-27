import type { PermissionApprovalRecord } from "@ennoia/api-client";

export type LocalMessageStatus = "queued" | "sending" | "failed";

export type ComposerSegment =
  | {
      kind: "text";
      value: string;
    }
  | {
      kind: "mention";
      agentId: string;
      label: string;
    }
  | {
      kind: "skill";
      skillId: string;
      label: string;
    };

export type LocalMessageDraft = {
  clientId: string;
  body: string;
  addressedAgents: string[];
  explicitMentions: string[];
  segments: ComposerSegment[];
  createdAt: string;
  status: LocalMessageStatus;
  branchId?: string;
  forkFromMessageId?: string;
  rewriteFromMessageId?: string;
  resetContext?: boolean;
  branchName?: string;
  error?: string;
};

export type PendingReplyMarker = {
  id: string;
  agentId: string;
  createdAt: string;
};

export type ChatEntryFormat = "plain" | "markdown" | "code" | "json" | "diagram";
export type ChatEntryState = "pending" | "streaming" | "done" | "failed";
export type ChatEntryTone = "accent" | "warn" | "danger" | "muted";

export type ChatEntryRecipient = {
  id: string;
  label: string;
};

type ChatEntryBase = {
  id: string;
  role: "operator" | "agent" | "system" | "tool";
  kind: "message" | "error" | "system" | "status" | "tool_result" | "approval";
  format: ChatEntryFormat;
  state: ChatEntryState;
  sender?: string;
  title?: string;
  body: string;
  createdAt: string;
};

export type ChatMessageEntry = ChatEntryBase & {
  kind: "message";
  messageId: string;
  branchId?: string;
  replyToMessageId?: string;
  rewriteFromMessageId?: string;
  recipients: ChatEntryRecipient[];
  mentions: string[];
  source: "remote" | "local";
  localStatus?: LocalMessageStatus;
  localError?: string;
};

export type ChatErrorEntry = ChatEntryBase & {
  kind: "error";
  title: string;
  summary: string;
  detail?: string;
  tone: "danger" | "warn";
  relatedEntryId?: string;
};

export type ChatSystemEntry = ChatEntryBase & {
  kind: "system";
};

export type ChatStatusEntry = ChatEntryBase & {
  kind: "status";
  label: string;
  detail?: string;
  animation: "typing";
};

export type ChatToolResultEntry = ChatEntryBase & {
  kind: "tool_result";
};

export type ChatApprovalEntry = ChatEntryBase & {
  kind: "approval";
  approval: PermissionApprovalRecord;
  agentLabel: string;
};

export type ChatEntryViewModel =
  | ChatMessageEntry
  | ChatErrorEntry
  | ChatSystemEntry
  | ChatStatusEntry
  | ChatToolResultEntry
  | ChatApprovalEntry;
