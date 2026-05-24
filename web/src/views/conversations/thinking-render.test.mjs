import { describe, expect, test } from "bun:test";

import { shouldKeepStandaloneAccessoryGroup } from "./chat-visibility.ts";

describe("conversation thinking rendering", () => {
  test("hides standalone reasoning entries when thinking visibility is off", () => {
    const accessories = [{
      id: "reasoning-1",
      role: "agent",
      kind: "reasoning",
      format: "plain",
      state: "done",
      sender: "lsy",
      body: "hidden internal reasoning",
      createdAt: "2026-05-24T11:57:10.000Z",
    }];

    expect(shouldKeepStandaloneAccessoryGroup({
      accessories,
      showThinking: false,
      showToolCalls: true,
    })).toBe(false);
  });

  test("keeps standalone status entries when thinking visibility is off", () => {
    const accessories = [{
      id: "status-1",
      role: "agent",
      kind: "status",
      format: "plain",
      state: "streaming",
      sender: "lsy",
      label: "思考中",
      animation: "typing",
      body: "Agent 已接到消息，正在组织回复与处理工具步骤。",
      createdAt: "2026-05-24T11:57:10.000Z",
    }];

    expect(shouldKeepStandaloneAccessoryGroup({
      accessories,
      showThinking: false,
      showToolCalls: true,
    })).toBe(true);
  });
});
