import type { AgentProfile, SkillConfig } from "@ennoia/api-client";

import { ChatEntry } from "./ChatEntry";
import type { ChatEntryViewModel } from "./chat-types";

export function ChatStream({
  entries,
  agents,
  skills,
  emptyMessage,
  formatDateTime,
  t,
  onRetry,
  onRemove,
}: {
  entries: ChatEntryViewModel[];
  agents: AgentProfile[];
  skills: SkillConfig[];
  emptyMessage: string;
  formatDateTime: (value: string) => string;
  t: (key: string, fallback: string) => string;
  onRetry: (id: string) => void;
  onRemove: (id: string) => void;
}) {
  if (entries.length === 0) {
    return <div className="empty-card conversation-empty-card">{emptyMessage}</div>;
  }

  return entries.map((entry) => (
    <ChatEntry
      key={entry.id}
      entry={entry}
      agents={agents}
      skills={skills}
      formatDateTime={formatDateTime}
      t={t}
      onRetry={onRetry}
      onRemove={onRemove}
    />
  ));
}
