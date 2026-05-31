export type ConversationResponseStrategy = "normal" | "clarify_first" | "acceptance_first";

export const PIPELINE_ACTIVATION_NAMESPACE = "pipeline.activation";
export const RESPONSE_STRATEGY_NAMESPACE = "workflow.response";
export const RESPONSE_STRATEGY_KEY = "strategy";

export const RESPONSE_STRATEGY_OPTIONS: ConversationResponseStrategy[] = [
  "normal",
  "clarify_first",
  "acceptance_first",
];

export function normalizeConversationResponseStrategy(value: unknown): ConversationResponseStrategy {
  return RESPONSE_STRATEGY_OPTIONS.includes(value as ConversationResponseStrategy)
    ? value as ConversationResponseStrategy
    : "normal";
}

export function responseStrategyUsesPipeline(strategy: ConversationResponseStrategy) {
  return strategy !== "normal";
}
