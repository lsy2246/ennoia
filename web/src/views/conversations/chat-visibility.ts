import type { ChatEntryViewModel } from "./chat-types";

type VisibilityEntry = Pick<ChatEntryViewModel, "kind">;

export function isReasoningEntry(entry: VisibilityEntry) {
  return entry.kind === "reasoning";
}

export function shouldKeepStandaloneAccessoryGroup(params: {
  accessories: VisibilityEntry[];
  showThinking: boolean;
  showToolCalls: boolean;
}) {
  const hasStatus = params.accessories.some((entry) => entry.kind === "status");
  const hasThinking = params.showThinking && params.accessories.some(isReasoningEntry);
  const hasToolCalls = params.showToolCalls && params.accessories.some((entry) => entry.kind === "tool_result");
  const hasOtherAccessories = params.accessories.some((entry) =>
    !isReasoningEntry(entry) && entry.kind !== "status" && entry.kind !== "tool_result");
  return hasStatus || hasThinking || hasToolCalls || hasOtherAccessories;
}
