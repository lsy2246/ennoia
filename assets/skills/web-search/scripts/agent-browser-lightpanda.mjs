#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");
const binaryName = process.platform === "win32" ? "agent-browser.exe" : "agent-browser";
const binaryPath = path.join(skillRoot, "node_modules", ".bin", binaryName);

if (!existsSync(binaryPath)) {
  console.error("agent-browser 尚未安装。请先运行 `node scripts/setup.mjs`。");
  process.exit(1);
}

const env = {
  ...process.env,
  AGENT_BROWSER_ENGINE: process.env.AGENT_BROWSER_ENGINE || "lightpanda",
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
