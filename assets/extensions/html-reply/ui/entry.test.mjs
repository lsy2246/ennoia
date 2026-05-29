import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const currentDir = dirname(fileURLToPath(import.meta.url));
const entrySource = readFileSync(join(currentDir, "entry.tsx"), "utf8");
const styleSource = readFileSync(join(currentDir, "html-reply.css"), "utf8");

describe("html reply message renderer", () => {
  test("renders HTML replies as message content without exposing source controls", () => {
    expect(entrySource).toContain("HtmlReplyCard");
    expect(entrySource).toContain('sandbox="allow-same-origin"');
    expect(entrySource).not.toContain("allow-scripts");
    expect(entrySource).not.toContain("html-reply-source");
    expect(entrySource).not.toContain("<summary>源码</summary>");
  });

  test("sizes short message iframes to their content instead of reserving viewport height", () => {
    expect(entrySource).toContain("html-reply-frame--auto");
    expect(entrySource).toContain("handleFrameLoad");
    expect(entrySource).toContain("setFrameHeight");
    expect(entrySource).toContain("scrollHeight");
    expect(entrySource).toContain("offsetHeight");
    expect(styleSource).toContain(".html-reply-frame--auto");
    expect(styleSource).toContain("height: var(--html-reply-frame-height, 64px);");
    expect(styleSource).toContain("max-height: min(70vh, 420px);");
    expect(styleSource).not.toContain("height: clamp(180px, 34vh, 420px);");
  });
});
