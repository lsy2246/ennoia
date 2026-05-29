import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { describe, expect, test } from "bun:test";

const currentDir = dirname(fileURLToPath(import.meta.url));
const styles = readFileSync(join(currentDir, "../../styles.css"), "utf8");

function cssBlock(selector) {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const match = styles.match(new RegExp(`${escaped}\\s*\\{(?<body>[^}]*)\\}`, "m"));
  return match?.groups?.body ?? "";
}

describe("conversation message accessory layout", () => {
  test("constrains accessory width without setting a vertical flex basis", () => {
    const accessoryBlock = cssBlock(".message-accessory");

    expect(accessoryBlock).toContain("width: min(100%, var(--chat-bubble-max-width));");
    expect(accessoryBlock).not.toMatch(/flex\s*:\s*0\s+1\s+min\(/);
    expect(accessoryBlock).not.toMatch(/flex-basis\s*:\s*min\(/);
  });
});
