import type { ExtensionRecordEntry, PermissionApprovalRecord } from "@ennoia/api-client";
import type { ConversationFailureSource } from "./error-classification";

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
  branchName?: string;
  error?: string;
};

export type PendingReplyMarker = {
  id: string;
  agentId: string;
  createdAt: string;
  sourceMessageId: string;
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
  kind: "message" | "error" | "system" | "status" | "tool_result" | "approval" | "record";
  format: ChatEntryFormat;
  state: ChatEntryState;
  sender?: string;
  title?: string;
  body: string;
  createdAt: string;
};

export type ConversationMessageEntry = ChatEntryBase & {
  kind: "message";
  messageId: string;
  branchId?: string;
  parentMessageId?: string;
  replyToMessageId?: string;
  rewriteFromMessageId?: string;
  recipients: ChatEntryRecipient[];
  mentions: string[];
  source: "remote" | "local";
  localStatus?: LocalMessageStatus;
  localError?: string;
  failureCode?: string;
  failureSource?: ConversationFailureSource;
  failureSummary?: string;
  failureDetail?: string;
};

export type ChatErrorEntry = ChatEntryBase & {
  kind: "error";
  title: string;
  summary: string;
  detail?: string;
  tone: "danger" | "warn";
  relatedMessageId?: string;
};

export type ChatSystemEntry = ChatEntryBase & {
  kind: "system";
  relatedMessageId?: string;
};

export type ChatStatusEntry = ChatEntryBase & {
  kind: "status";
  label: string;
  detail?: string;
  animation: "typing";
  relatedMessageId?: string;
  sourceMessageId?: string;
  live?: boolean;
};

export type ChatToolResultEntry = ChatEntryBase & {
  kind: "tool_result";
  relatedMessageId?: string;
  actorSender?: string;
};

export type ChatApprovalEntry = ChatEntryBase & {
  kind: "approval";
  approval: PermissionApprovalRecord;
  agentLabel: string;
};

export type ChatRecordEntry = ChatEntryBase & {
  kind: "record";
  record: ExtensionRecordEntry;
  relatedMessageId?: string;
};

export type ChatEntryViewModel =
  | ConversationMessageEntry
  | ChatErrorEntry
  | ChatSystemEntry
  | ChatStatusEntry
  | ChatToolResultEntry
  | ChatApprovalEntry
  | ChatRecordEntry;
