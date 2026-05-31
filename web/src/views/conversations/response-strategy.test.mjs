import { describe, expect, test } from "bun:test";

import {
  normalizeConversationResponseStrategy,
  responseStrategyUsesPipeline,
} from "./response-strategy.ts";

describe("conversation response strategy", () => {
  test("keeps normal response outside the workflow pipeline", () => {
    expect(responseStrategyUsesPipeline("normal")).toBe(false);
    expect(responseStrategyUsesPipeline("clarify_first")).toBe(true);
    expect(responseStrategyUsesPipeline("acceptance_first")).toBe(true);
  });

  test("normalizes unknown stored values to normal response", () => {
    expect(normalizeConversationResponseStrategy("clarify_first")).toBe("clarify_first");
    expect(normalizeConversationResponseStrategy("acceptance_first")).toBe("acceptance_first");
    expect(normalizeConversationResponseStrategy("task_mode")).toBe("normal");
    expect(normalizeConversationResponseStrategy(null)).toBe("normal");
  });
});
