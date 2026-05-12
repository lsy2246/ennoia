#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

import { resolveLightpandaRuntime } from "./runtime.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");
const binaryName = process.platform === "win32" ? "agent-browser.exe" : "agent-browser";
const binaryPath = path.join(skillRoot, "node_modules", ".bin", binaryName);

if (!existsSync(binaryPath)) {
  console.error("agent-browser 尚未安装。请先在技能目录自行安装依赖。");
  process.exit(1);
}

const runtime = resolveLightpandaRuntime();
const env = {
  ...process.env,
  AGENT_BROWSER_ENGINE: process.env.AGENT_BROWSER_ENGINE || "lightpanda",
  ...(runtime.resolvedPath ? { LIGHTPANDA_EXECUTABLE_PATH: runtime.resolvedPath } : {}),
};

const child = spawn(binaryPath, process.argv.slice(2), {
  stdio: "inherit",
  cwd: skillRoot,
  env,
  shell: false,
});

child.on("exit", (code, signal) => {
  if (signal) {
    process.kill(process.pid, signal);
    return;
  }
  process.exit(code ?? 1);
});
