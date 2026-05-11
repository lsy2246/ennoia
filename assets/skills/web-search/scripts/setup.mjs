#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import process from "node:process";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: skillRoot,
    stdio: "inherit",
    shell: false,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function runBun(args, stdio = "inherit") {
  const result =
    process.platform === "win32"
      ? spawnSync("cmd.exe", ["/c", "bun", ...args], {
          cwd: skillRoot,
          stdio,
          shell: false,
        })
      : spawnSync("bun", args, {
          cwd: skillRoot,
          stdio,
          shell: false,
        });

  return result;
}

const bunCheck = runBun(["--version"], "ignore");

if (bunCheck.status !== 0) {
  console.error("未检测到 bun。请先安装 bun，再运行 `node scripts/setup.mjs`。");
  process.exit(1);
}

const install = runBun(["install"]);
if (install.status !== 0) {
  process.exit(install.status ?? 1);
}

run(process.execPath, [path.join(scriptDir, "doctor.mjs")]);
