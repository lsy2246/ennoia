import { describe, expect, test } from "bun:test";

import {
  areComposerPickerStatesEqual,
  areComposerSnapshotsEqual,
} from "./session-composer.ts";

describe("conversation composer state", () => {
  test("treats equivalent snapshots as unchanged even when arrays are recreated", () => {
    const current = {
      body: "hello",
      addressedAgents: ["agent-a"],
      explicitMentions: ["agent-a"],
      segments: [
        { kind: "text", value: "hello " },
        { kind: "mention", agentId: "agent-a", label: "Agent A" },
      ],
    };
    const next = {
      body: "hello",
      addressedAgents: ["agent-a"],
      explicitMentions: ["agent-a"],
      segments: [
        { kind: "text", value: "hello " },
        { kind: "mention", agentId: "agent-a", label: "Agent A" },
      ],
    };

    expect(areComposerSnapshotsEqual(current, next)).toBe(true);
  });

  test("detects meaningful picker state changes", () => {
    expect(areComposerPickerStatesEqual(
      { open: false, mode: "mention", query: "", selectedIndex: 0 },
      { open: false, mode: "mention", query: "", selectedIndex: 0 },
    )).toBe(true);
    expect(areComposerPickerStatesEqual(
      { open: true, mode: "mention", query: "a", selectedIndex: 0 },
      { open: true, mode: "mention", query: "a", selectedIndex: 1 },
    )).toBe(false);
  });
});
