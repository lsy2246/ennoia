#!/usr/bin/env node

import assert from "node:assert/strict";
import { mkdtempSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  resolveBrowserRuntime,
  webSearchCloakBrowserCacheDir,
} from "./runtime.mjs";

test("builtin runtime is not ready before the Ennoia-managed browser cache is prepared", () => {
  const previousHome = process.env.ENNOIA_HOME;
  const previousDataDir = process.env.ENNOIA_SKILL_DATA_DIR;
  const home = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-runtime-"));

  try {
    delete process.env.ENNOIA_SKILL_DATA_DIR;
    process.env.ENNOIA_HOME = home;

    const runtime = resolveBrowserRuntime({ browser_kernel: "builtin" });

    assert.equal(runtime.mode, "builtin");
    assert.equal(runtime.ready, false);
    assert.equal(runtime.issue?.code, "builtin_browser_not_prepared");
    assert.equal(
      runtime.cacheDir,
      path.join(home, "data", "skills", "web-search", "cloakbrowser"),
    );
    assert.equal(webSearchCloakBrowserCacheDir(), runtime.cacheDir);
  } finally {
    if (previousHome === undefined) {
      delete process.env.ENNOIA_HOME;
    } else {
      process.env.ENNOIA_HOME = previousHome;
    }
    if (previousDataDir === undefined) {
      delete process.env.ENNOIA_SKILL_DATA_DIR;
    } else {
      process.env.ENNOIA_SKILL_DATA_DIR = previousDataDir;
    }
    rmSync(home, { recursive: true, force: true });
  }
});

test("mcp browser control uses a direct streamable HTTP service configuration", () => {
  const runtime = resolveBrowserRuntime({
    browser_control: "mcp",
    mcp_transport: "streamable-http",
    mcp_url: "https://browser.example.com/mcp",
  });

  assert.equal(runtime.controlMode, "mcp");
  assert.equal(runtime.ready, true);
  assert.equal(runtime.mcpTransport, "streamable-http");
  assert.equal(runtime.mcpUrl, "https://browser.example.com/mcp");
  assert.equal(runtime.browserKernelId, "");
  assert.equal(runtime.issue, null);
});
