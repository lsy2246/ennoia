import { describe, expect, test } from "bun:test";

import {
  normalizeCodeBlockText,
  normalizeFencedCodeBody,
} from "./code-block-content.ts";

describe("conversation code block content", () => {
  test("removes trailing blank lines after stripping a fenced code block", () => {
    expect(normalizeFencedCodeBody("```ts\nconst value = 1;\n\n\n```")).toBe("const value = 1;");
  });

  test("keeps intentional blank lines inside code blocks", () => {
    expect(normalizeCodeBlockText("const a = 1;\n\nconst b = 2;\n\n")).toBe("const a = 1;\n\nconst b = 2;");
  });
});
