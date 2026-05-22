#!/usr/bin/env node

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { chmodSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";

const scriptPath = fileURLToPath(new URL("./prepare-browser.mjs", import.meta.url));

test("prepare browser help explains the prepared-cache flow without dependencies", () => {
  const result = spawnSync(process.execPath, [scriptPath, "--help"], {
    encoding: "utf8",
    env: { ...process.env, NODE_PATH: "" },
  });

  assert.equal(result.status, 0);
  assert.match(result.stdout, /运行搜索时只使用这里已经准备好的内核/);
});

test("prepare browser installs skill dependencies before loading cloakbrowser", () => {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-prepare-"));
  const fakeBin = path.join(tempRoot, "bin");
  const skillRoot = path.join(tempRoot, "skill");
  const dataDir = path.join(tempRoot, "data");
  const installScript = path.join(fakeBin, "fake-install.mjs");

  try {
    mkdirSync(fakeBin, { recursive: true });
    mkdirSync(skillRoot, { recursive: true });
    writeFileSync(path.join(skillRoot, "bun.lock"), "");
    writeFileSync(
      path.join(skillRoot, "package.json"),
      JSON.stringify({
        type: "module",
        dependencies: {
          cloakbrowser: "0.0.0-test",
          "playwright-core": "0.0.0-test",
          "@mozilla/readability": "0.0.0-test",
          cheerio: "0.0.0-test",
          linkedom: "0.0.0-test",
        },
      }),
    );
    writeFileSync(
      installScript,
      `
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";

if (process.argv.includes("--version")) {
  console.log("1.0.0-test");
  process.exit(0);
}

writeFileSync(path.join(process.cwd(), "install-marker.json"), JSON.stringify({ args: process.argv.slice(2) }));

function writePackage(name, body = "{}") {
  const parts = name.split("/");
  const dir = path.join(process.cwd(), "node_modules", ...parts);
  mkdirSync(dir, { recursive: true });
  writeFileSync(path.join(dir, "package.json"), body);
}

writePackage("cloakbrowser", JSON.stringify({ type: "module", exports: { ".": { import: "./index.mjs" } } }));
writeFileSync(path.join(process.cwd(), "node_modules", "cloakbrowser", "index.mjs"), \`
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
export async function ensureBinary() {
  console.log("[cloakbrowser] preparing test browser");
  const binaryDir = path.join(process.env.CLOAKBROWSER_CACHE_DIR, "chromium-test");
  mkdirSync(binaryDir, { recursive: true });
  const binaryPath = path.join(binaryDir, process.platform === "win32" ? "chrome.exe" : "chrome");
  writeFileSync(binaryPath, "");
  return binaryPath;
}
\`);
writePackage("playwright-core");
writePackage("@mozilla/readability");
writePackage("cheerio");
writePackage("linkedom");
`,
    );
    writeFileSync(
      path.join(fakeBin, "bun.cmd"),
      `@echo off\r\n"${process.execPath}" "${installScript}" %*\r\n`,
    );
    writeFileSync(
      path.join(fakeBin, "bun"),
      `#!/bin/sh\n"${process.execPath}" "${installScript}" "$@"\n`,
    );
    chmodSync(path.join(fakeBin, "bun"), 0o755);

    const result = spawnSync(process.execPath, [scriptPath], {
      encoding: "utf8",
      env: {
        ...process.env,
        ENNOIA_SKILL_ROOT: skillRoot,
        ENNOIA_SKILL_DATA_DIR: dataDir,
        NODE_PATH: "",
        PATH: `${fakeBin}${path.delimiter}${process.env.PATH || ""}`,
      },
    });

    assert.equal(result.status, 0, result.stderr || result.stdout);
    const installMarker = JSON.parse(readFileSync(path.join(skillRoot, "install-marker.json"), "utf8"));
    assert.deepEqual(installMarker.args, ["install", "--ignore-scripts"]);

    const output = JSON.parse(result.stdout);
    assert.equal(output.status, "ready");
    const browserRuntime = output.items.find((item) => item.key === "browser-runtime");
    assert.equal(browserRuntime.status, "ok");
    assert.match(browserRuntime.message, /chromium-test/);
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});

test("prepare browser keeps prepare action visible when dependency install fails", () => {
  const tempRoot = mkdtempSync(path.join(os.tmpdir(), "ennoia-web-search-prepare-fail-"));
  const fakeBin = path.join(tempRoot, "bin");
  const skillRoot = path.join(tempRoot, "skill");
  const dataDir = path.join(tempRoot, "data");

  try {
    mkdirSync(fakeBin, { recursive: true });
    mkdirSync(skillRoot, { recursive: true });
    writeFileSync(path.join(skillRoot, "bun.lock"), "");
    writeFileSync(
      path.join(skillRoot, "package.json"),
      JSON.stringify({
        type: "module",
        dependencies: {
          cloakbrowser: "0.0.0-test",
        },
      }),
    );
    writeFileSync(
      path.join(fakeBin, "bun.cmd"),
      `@echo off\r\nif "%1"=="--version" (echo 1.0.0-test & exit /b 0)\r\necho install failed 1>&2\r\nexit /b 42\r\n`,
    );
    writeFileSync(
      path.join(fakeBin, "bun"),
      `#!/bin/sh\nif [ "$1" = "--version" ]; then echo "1.0.0-test"; exit 0; fi\necho "install failed" >&2\nexit 42\n`,
    );
    chmodSync(path.join(fakeBin, "bun"), 0o755);

    const result = spawnSync(process.execPath, [scriptPath], {
      encoding: "utf8",
      env: {
        ...process.env,
        ENNOIA_SKILL_ROOT: skillRoot,
        ENNOIA_SKILL_DATA_DIR: dataDir,
        NODE_PATH: "",
        PATH: `${fakeBin}${path.delimiter}${process.env.PATH || ""}`,
      },
    });

    assert.equal(result.status, 1);
    const output = JSON.parse(result.stdout);
    assert.equal(output.status, "env_missing");
    assert.equal(output.summary, "技能依赖安装失败，内置浏览器尚未准备。");
    assert.ok(output.actions.some((action) => action.kind === "prepare"));
    assert.ok(output.actions.some((action) => action.kind === "recheck"));
  } finally {
    rmSync(tempRoot, { recursive: true, force: true });
  }
});
