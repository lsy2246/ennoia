#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import test from "node:test";

const scriptPath = fileURLToPath(new URL("./browser-open.mjs", import.meta.url));

test("browser-open reports MCP mode as unsupported without loading CloakBrowser", () => {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-browser-open-mcp-"));
  const loaderPath = path.join(tempRoot, "reject-cloakbrowser.mjs");

  try {
    writeFileSync(
      loaderPath,
      `
export async function resolve(specifier, context, nextResolve) {
  if (specifier === "cloakbrowser") {
    throw new Error("cloakbrowser was loaded");
  }
  return nextResolve(specifier, context);
}
`,
    );

    const result = spawnSync(
      process.execPath,
      [
        "--loader",
        pathToFileURL(loaderPath).href,
        scriptPath,
        "https://example.com",
      ],
      {
        encoding: "utf8",
        env: {
          ...process.env,
          ENNOIA_SKILL_CONFIG_JSON: JSON.stringify({
            browser_control: "mcp",
            mcp_transport: "streamable-http",
            mcp_url: "https://browser.example.com/mcp",
          }),
          NODE_PATH: "",
        },
      },
    );

    assert.equal(result.status, 1);
    assert.match(result.stderr, /browser-open 只支持本地自动化浏览器调试/);
    assert.doesNotMatch(result.stderr, /cloakbrowser was loaded/);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});
