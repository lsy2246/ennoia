import { readFileSync } from "node:fs";
import { join } from "node:path";
import test from "node:test";
import assert from "node:assert/strict";

const entrySource = readFileSync(join(import.meta.dirname, "entry.tsx"), "utf8");

test("markdown renderer UI exports markdown message renderer mount", () => {
  assert.match(entrySource, /ReactMarkdown/);
  assert.match(entrySource, /remarkGfm/);
  assert.match(entrySource, /messageRenderers/);
  assert.match(entrySource, /markdown-renderer\.markdown/);
});
