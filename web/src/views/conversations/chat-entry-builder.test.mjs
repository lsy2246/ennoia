import { describe, expect, test } from "bun:test";

import { buildChatEntries } from "./chat-entry-builder.ts";

function message(overrides) {
  return {
    id: "msg-1",
    conversation_id: "conv-1",
    branch_id: "branch-1",
    lane_id: "lane-1",
    sender: "agent-a",
    role: "agent",
    body: "hello",
    mentions: [],
    created_at: "2026-05-31T00:00:00.000Z",
    ...overrides,
  };
}

describe("chat entry message format", () => {
  test("uses persisted html format before content inference", () => {
    const entries = buildChatEntries({
      messages: [message({
        body: "<section><h2>HTML</h2></section>",
        format: "html",
      })],
      localDrafts: [],
      resolveRecipients: () => [],
    });

    expect(entries[0]?.format).toBe("html");
  });

  test("falls back to content inference for legacy messages without format", () => {
    const entries = buildChatEntries({
      messages: [message({
        body: "{\"ok\":true}",
        format: undefined,
      })],
      localDrafts: [],
      resolveRecipients: () => [],
    });

    expect(entries[0]?.format).toBe("json");
  });
});
