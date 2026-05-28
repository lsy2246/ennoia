import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const currentDir = dirname(fileURLToPath(import.meta.url));
const entrySource = readFileSync(join(currentDir, "entry.tsx"), "utf8");

describe("html reply message renderer", () => {
  test("renders HTML replies as message content without exposing source controls", () => {
    expect(entrySource).toContain("HtmlReplyCard");
    expect(entrySource).toContain('sandbox=""');
    expect(entrySource).not.toContain("html-reply-source");
    expect(entrySource).not.toContain("<summary>源码</summary>");
  });
});
