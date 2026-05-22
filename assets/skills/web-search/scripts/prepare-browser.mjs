#!/usr/bin/env node

import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import path from "node:path";
import process from "node:process";
import { fileURLToPath, pathToFileURL } from "node:url";
import { format } from "node:util";

import {
  applyBuiltinBrowserPrepareEnv,
  resolveBuiltinBrowserCache,
} from "./runtime.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const requiredPackages = [
  "cloakbrowser",
  "playwright-core",
  "@mozilla/readability",
  "cheerio",
  "linkedom",
];

const prepareActions = [
  { key: "prepare-browser", label: "准备内置浏览器", kind: "prepare" },
  { key: "recheck", label: "重新检测", kind: "recheck" },
];

function usage() {
  console.log(`用法:
  node scripts/prepare-browser.mjs

说明:
  将 CloakBrowser Chromium 下载并解压到 Ennoia 管理的 web-search 技能缓存目录。
  运行搜索时只使用这里已经准备好的内核，不在搜索过程中临时下载。
`);
}

function skillRoot() {
  const configured = typeof process.env.ENNOIA_SKILL_ROOT === "string"
    ? process.env.ENNOIA_SKILL_ROOT.trim()
    : "";
  return configured || path.resolve(scriptDir, "..");
}

function packageJsonPath(root) {
  return path.join(root, "package.json");
}

function packageDir(root, name) {
  return path.join(root, "node_modules", ...name.split("/"));
}

function packageExists(root, name) {
  return existsSync(path.join(packageDir(root, name), "package.json"));
}

function missingPackages(root) {
  return requiredPackages.filter((name) => !packageExists(root, name));
}

function dependencyItems(root, missing = missingPackages(root)) {
  const missingSet = new Set(missing);
  return [
    {
      key: "package-json",
      category: "dependency",
      label: "package.json",
      status: existsSync(packageJsonPath(root)) ? "ok" : "missing",
      required: true,
      message: existsSync(packageJsonPath(root))
        ? "技能包依赖清单存在"
        : "技能包依赖清单不存在",
      fix_hint: "确认 web-search 技能目录完整同步到本地实例。",
    },
    ...requiredPackages.map((name) => ({
      key: name.replace(/^@/, "").replace(/[/@]/g, "-"),
      category: "dependency",
      label: name,
      status: missingSet.has(name) ? "missing" : "ok",
      required: true,
      message: missingSet.has(name) ? `${name} 未安装` : `${name} 已安装`,
      fix_hint: "点击“准备内置浏览器”安装技能依赖并准备内置浏览器。",
    })),
  ];
}

function resultPayload({ status, summary, items = [], actions = [] }) {
  return { status, summary, checked_at: null, items, actions };
}

function writeResult(payload) {
  process.stdout.write(`${JSON.stringify(payload, null, 2)}\n`);
}

function writeFailure({ summary, items, error }) {
  if (error) {
    const message = error instanceof Error ? error.stack || error.message : String(error);
    process.stderr.write(`${message}\n`);
  }
  writeResult(resultPayload({
    status: "env_missing",
    summary,
    items,
    actions: prepareActions,
  }));
}

function commandAvailable(command) {
  const result = spawnSync(command, ["--version"], {
    encoding: "utf8",
    shell: process.platform === "win32",
    stdio: ["ignore", "ignore", "ignore"],
    windowsHide: true,
  });
  return result.status === 0;
}

function pickPackageManager(root) {
  if (existsSync(path.join(root, "bun.lock")) && commandAvailable("bun")) {
    return { command: "bun", args: ["install", "--ignore-scripts"] };
  }
  if (commandAvailable("npm")) {
    return { command: "npm", args: ["install", "--ignore-scripts"] };
  }
  if (commandAvailable("bun")) {
    return { command: "bun", args: ["install", "--ignore-scripts"] };
  }
  return null;
}

function installDependencies(root) {
  const manager = pickPackageManager(root);
  if (!manager) {
    throw new Error("未找到可用的包管理器：需要 bun 或 npm。");
  }

  const result = spawnSync(manager.command, manager.args, {
    cwd: root,
    encoding: "utf8",
    shell: process.platform === "win32",
    stdio: ["ignore", "pipe", "pipe"],
    windowsHide: true,
  });

  if (result.stdout) {
    process.stderr.write(result.stdout);
  }
  if (result.stderr) {
    process.stderr.write(result.stderr);
  }
  if (result.status !== 0) {
    throw new Error(`${manager.command} ${manager.args.join(" ")} 执行失败，退出码 ${result.status ?? "unknown"}。`);
  }
}

function assertPackageJson(root) {
  const filePath = packageJsonPath(root);
  if (!existsSync(filePath)) {
    throw new Error(`web-search 技能目录缺少 package.json：${filePath}`);
  }
  JSON.parse(readFileSync(filePath, "utf8"));
}

function packageEntryFromExports(exportsField) {
  if (typeof exportsField === "string") {
    return exportsField;
  }
  const rootExport = exportsField?.["."] || exportsField;
  if (typeof rootExport === "string") {
    return rootExport;
  }
  if (typeof rootExport?.import === "string") {
    return rootExport.import;
  }
  if (typeof rootExport?.default === "string") {
    return rootExport.default;
  }
  return "";
}

function resolvePackageImportEntry(root, name) {
  const dir = packageDir(root, name);
  const manifestPath = path.join(dir, "package.json");
  if (!existsSync(manifestPath)) {
    throw new Error(`${name} 未安装：${manifestPath}`);
  }

  const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
  const entry = packageEntryFromExports(manifest.exports) || manifest.module || manifest.main || "index.js";
  return path.join(dir, entry);
}

async function loadCloakBrowser(root) {
  const entry = resolvePackageImportEntry(root, "cloakbrowser");
  return import(pathToFileURL(entry).href);
}

async function withStdoutLogsOnStderr(callback) {
  const originalLog = console.log;
  const originalInfo = console.info;
  const redirect = (...args) => {
    process.stderr.write(`${format(...args)}\n`);
  };
  console.log = redirect;
  console.info = redirect;
  try {
    return await callback();
  } finally {
    console.log = originalLog;
    console.info = originalInfo;
  }
}

async function main() {
  if (process.argv.includes("--help") || process.argv.includes("-h")) {
    usage();
    return;
  }

  const root = skillRoot();
  try {
    assertPackageJson(root);
  } catch (error) {
    writeFailure({
      summary: "web-search 技能目录不完整，无法准备内置浏览器。",
      items: dependencyItems(root),
      error,
    });
    process.exit(1);
  }

  const missingBeforeInstall = missingPackages(root);
  if (missingBeforeInstall.length > 0) {
    try {
      installDependencies(root);
    } catch (error) {
      writeFailure({
        summary: "技能依赖安装失败，内置浏览器尚未准备。",
        items: dependencyItems(root),
        error,
      });
      process.exit(1);
    }
  }

  const missingAfterInstall = missingPackages(root);
  if (missingAfterInstall.length > 0) {
    writeFailure({
      summary: "技能依赖安装后仍不完整，内置浏览器尚未准备。",
      items: dependencyItems(root, missingAfterInstall),
    });
    process.exit(1);
  }

  const cacheDir = applyBuiltinBrowserPrepareEnv();
  let binaryPath = "";
  try {
    const { ensureBinary } = await loadCloakBrowser(root);
    binaryPath = await withStdoutLogsOnStderr(() => ensureBinary());
  } catch (error) {
    writeFailure({
      summary: "内置 CloakBrowser Chromium 准备失败。",
      items: [
        ...dependencyItems(root, []),
        {
          key: "browser-runtime",
          category: "dependency",
          label: "内置浏览器",
          status: "error",
          required: true,
          message: error instanceof Error ? error.message : String(error),
          fix_hint: "确认网络可用后重新点击“准备内置浏览器”。",
        },
      ],
      error,
    });
    process.exit(1);
  }

  const cache = resolveBuiltinBrowserCache();
  const ready = cache.ready;

  writeResult(resultPayload({
    status: ready ? "ready" : "error",
    summary: ready ? "内置浏览器准备完成。" : "内置浏览器准备后仍不可用。",
    items: [
      ...dependencyItems(root, []),
      {
        key: "browser-runtime",
        category: "dependency",
        label: "内置浏览器",
        status: ready ? "ok" : "error",
        required: true,
        message: ready
          ? `已准备 CloakBrowser Chromium：${cache.binaryPath || binaryPath}`
          : `未在缓存目录发现可用浏览器：${cacheDir}`,
        fix_hint: ready ? undefined : "重新点击“准备内置浏览器”。",
      },
    ],
    actions: ready ? [] : prepareActions,
  }));

  if (!ready) {
    process.exit(1);
  }
}

main().catch((error) => {
  writeFailure({
    summary: "内置浏览器准备流程异常退出。",
    items: [],
    error,
  });
  process.exit(1);
});
