#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");
const lightpandaDefaultPath = path.join(
  process.env.USERPROFILE || process.env.HOME || "",
  ".cache",
  "lightpanda-node",
  process.platform === "win32" ? "lightpanda.exe" : "lightpanda",
);

const checks = [];

function pushCheck(name, ok, detail) {
  checks.push({ name, ok, detail });
}

pushCheck(
  "package.json",
  existsSync(path.join(skillRoot, "package.json")),
  "技能包依赖清单存在",
);

pushCheck(
  "agent-browser",
  existsSync(
    path.join(
      skillRoot,
      "node_modules",
      ".bin",
      process.platform === "win32" ? "agent-browser.exe" : "agent-browser",
    ),
  ),
  "agent-browser 二进制入口已安装",
);

pushCheck(
  "lightpanda-package",
  existsSync(path.join(skillRoot, "node_modules", "@lightpanda", "browser", "package.json")),
  "@lightpanda/browser 包已安装",
);

const hasExplicitExecutable = Boolean(process.env.LIGHTPANDA_EXECUTABLE_PATH);
const hasDefaultExecutable = existsSync(lightpandaDefaultPath);

if (process.platform === "win32") {
  pushCheck(
    "lightpanda-runtime",
    hasExplicitExecutable,
    hasExplicitExecutable
      ? "已显式设置 LIGHTPANDA_EXECUTABLE_PATH"
      : "Windows 原生不自带 Lightpanda 二进制；请在 WSL2 中运行，或显式设置 LIGHTPANDA_EXECUTABLE_PATH",
  );
} else {
  pushCheck(
    "lightpanda-runtime",
    hasExplicitExecutable || hasDefaultExecutable,
    hasExplicitExecutable || hasDefaultExecutable
      ? "检测到可用的 Lightpanda 可执行文件"
      : "未检测到 Lightpanda 可执行文件；请重新运行 `node scripts/setup.mjs`",
  );
}

const failed = checks.filter((item) => !item.ok);

for (const check of checks) {
  const prefix = check.ok ? "OK" : "ERR";
  console.log(`${prefix} ${check.name}: ${check.detail}`);
}

if (failed.length > 0) {
  process.exit(1);
}
