import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "bun:test";

const currentDir = dirname(fileURLToPath(import.meta.url));
const sessionSource = readFileSync(join(currentDir, "Session.tsx"), "utf8");

describe("conversation response strategy UI", () => {
  test("renders a processing strategy selector instead of the old checkbox", () => {
    expect(sessionSource).toContain("composer-response-strategy");
    expect(sessionSource).toContain("处理策略");
    expect(sessionSource).toContain("常规响应");
    expect(sessionSource).toContain("澄清优先");
    expect(sessionSource).toContain("验收先行");
    expect(sessionSource).not.toContain("composer-pipeline-toggle");
  });
});
