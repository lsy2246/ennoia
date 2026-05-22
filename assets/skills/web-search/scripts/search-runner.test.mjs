#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { pathToFileURL, fileURLToPath } from "node:url";
import test from "node:test";

const scriptPath = fileURLToPath(new URL("./search-runner.mjs", import.meta.url));

test("MCP browser control does not load local browser automation dependencies", () => {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-runner-mcp-"));
  const loaderPath = path.join(tempRoot, "reject-local-browser-deps.mjs");

  try {
    writeFileSync(
      loaderPath,
      `
const blocked = new Set([
  "cloakbrowser",
  "cheerio",
  "linkedom",
  "@mozilla/readability",
]);

export async function resolve(specifier, context, nextResolve) {
  if (blocked.has(specifier)) {
    throw new Error("local browser dependency was loaded: " + specifier);
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
        "cloakbrowser mcp",
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

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const payload = JSON.parse(result.stdout);

    assert.equal(payload.browser_control, "mcp");
    assert.equal(payload.available, false);
    assert.equal(payload.status, "mcp_provider_unavailable");
    assert.equal(payload.mcp_transport, "streamable-http");
    assert.equal(payload.mcp_url, "https://browser.example.com/mcp");
    assert.equal(Object.hasOwn(payload, ["mcp", "server", "id"].join("_")), false);
    assert.deepEqual(payload.results, []);
    assert.deepEqual(payload.pages, []);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});
