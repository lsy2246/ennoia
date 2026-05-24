import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const currentDir = dirname(fileURLToPath(import.meta.url));
const entrySource = readFileSync(join(currentDir, "entry.tsx"), "utf8");

describe("artifact runner HTML preview", () => {
  test("allows scripts in the sandboxed iframe so canvas previews can draw", () => {
    expect(entrySource).toContain('sandbox="allow-scripts"');
  });
});
