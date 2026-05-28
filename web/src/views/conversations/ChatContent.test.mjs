import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "bun:test";

const currentDir = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(currentDir, "ChatContent.tsx"), "utf8");

describe("ChatContent markdown delegation", () => {
  test("delegates markdown rendering to message renderer extensions", () => {
    expect(source).toContain("loadExtensionMessageRendererMount");
    expect(source).toContain("selectMessageRenderer");
    expect(source).not.toContain("react-markdown");
    expect(source).not.toContain("remark-gfm");
  });
});
