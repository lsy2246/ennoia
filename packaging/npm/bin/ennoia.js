#!/usr/bin/env node

import { existsSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const packageRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const isWindows = process.platform === "win32";
const executableName = isWindows ? "ennoia.exe" : "ennoia";
const targetProfile = process.env.ENNOIA_CLI_PROFILE ?? "debug";

const candidatePaths = [
  process.env.ENNOIA_CLI_BIN,
  join(packageRoot, "vendor", executableName),
  resolve(packageRoot, "..", "..", "target", targetProfile, executableName),
].filter(Boolean);

const executablePath = candidatePaths.find((candidate) => existsSync(candidate));

if (!executablePath) {
  console.error(
    [
      "[ennoia] 当前未找到可执行 CLI。",
      "请先执行仓库根目录的 `bun run install:workspace`，或通过 ENNOIA_CLI_BIN 指向已构建的 ennoia 可执行文件。",
    ].join(" "),
  );
  process.exit(1);
}

const result = spawnSync(executablePath, process.argv.slice(2), {
  stdio: "inherit",
  env: process.env,
});

if (result.error) {
  console.error(`[ennoia] 启动失败: ${result.error.message}`);
  process.exit(1);
}

process.exit(result.status ?? 0);
