#!/usr/bin/env node

import assert from "node:assert/strict";
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";
import { fileURLToPath } from "node:url";

const scriptPath = fileURLToPath(new URL("./doctor.mjs", import.meta.url));

test("doctor reports missing dependencies as not installed", () => {
  const home = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-doctor-"));
  const skillRoot = path.join(home, "skill");
  mkdirSync(skillRoot, { recursive: true });
  writeFileSync(path.join(skillRoot, "package.json"), JSON.stringify({ type: "module" }));

  const env = { ...process.env, ENNOIA_HOME: home, ENNOIA_SKILL_ROOT: skillRoot };
  delete env.ENNOIA_SKILL_DATA_DIR;
  delete env.ENNOIA_SKILL_CONFIG_JSON;

  try {
    const result = spawnSync(process.execPath, [scriptPath], {
      encoding: "utf8",
      env,
    });

    assert.equal(result.status, 1);
    const payload = JSON.parse(result.stdout);
    const cloakbrowser = payload.items.find((item) => item.key === "cloakbrowser");

    assert.equal(cloakbrowser.status, "missing");
    assert.equal(cloakbrowser.message, "cloakbrowser 未安装");
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});

test("doctor reports MCP browser control with direct service configuration", () => {
  const home = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-doctor-mcp-"));
  const skillRoot = path.join(home, "skill");
  mkdirSync(skillRoot, { recursive: true });
  writeFileSync(path.join(skillRoot, "package.json"), JSON.stringify({ type: "module" }));

  const env = {
    ...process.env,
    ENNOIA_HOME: home,
    ENNOIA_SKILL_ROOT: skillRoot,
    ENNOIA_SKILL_CONFIG_JSON: JSON.stringify({
      browser_control: "mcp",
      mcp_transport: "streamable-http",
      mcp_url: "https://browser.example.com/mcp",
    }),
  };
  delete env.ENNOIA_SKILL_DATA_DIR;

  try {
    const result = spawnSync(process.execPath, [scriptPath], {
      encoding: "utf8",
      env,
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const payload = JSON.parse(result.stdout);
    const browserControl = payload.items.find((item) => item.key === "browser-control");
    const mcpTransport = payload.items.find((item) => item.key === "mcp-transport");
    const mcpUrl = payload.items.find((item) => item.key === "mcp-url");

    assert.equal(payload.status, "partial");
    assert.equal(browserControl.status, "ok");
    assert.equal(mcpTransport.status, "ok");
    assert.equal(mcpUrl.status, "ok");
    assert.equal(payload.items.some((item) => item.key.includes("tool")), false);
    assert.equal(payload.items.some((item) => item.key === ["mcp", "server", "id"].join("-")), false);
  } finally {
    rmSync(home, { recursive: true, force: true });
  }
});
