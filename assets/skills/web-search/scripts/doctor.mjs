#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  describeResolvedSource,
  isWindowsNative,
  lightpandaDefaultPath,
  resolveLightpandaRuntime,
  windowsRuntimeGuidance,
} from "./runtime.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");
const runtime = resolveLightpandaRuntime();

const items = [];

function pushItem({
  key,
  category,
  label,
  status,
  required = true,
  message,
  fix_hint,
}) {
  items.push({ key, category, label, status, required, message, fix_hint });
}

pushItem({
  key: "package-json",
  category: "dependency",
  label: "package.json",
  status: existsSync(path.join(skillRoot, "package.json")) ? "ok" : "missing",
  message: "技能包依赖清单存在",
  fix_hint: "确认技能目录完整同步到本地实例。",
});

pushItem({
  key: "agent-browser",
  category: "dependency",
  label: "agent-browser",
  status: existsSync(
    path.join(
      skillRoot,
      "node_modules",
      ".bin",
      process.platform === "win32" ? "agent-browser.exe" : "agent-browser",
    ),
  ) ? "ok" : "missing",
  message: "agent-browser 二进制入口已安装",
  fix_hint: "请先在技能目录自行安装依赖，再重新尝试。",
});

pushItem({
  key: "lightpanda-package",
  category: "dependency",
  label: "@lightpanda/browser",
  status: existsSync(path.join(skillRoot, "node_modules", "@lightpanda", "browser", "package.json"))
    ? "ok"
    : "missing",
  message: "@lightpanda/browser 包已安装",
  fix_hint: "请先在技能目录自行安装依赖，再重新尝试。",
});

if (runtime.configuredPath && !runtime.configuredExists) {
  pushItem({
    key: "lightpanda-config-path",
    category: "config",
    label: "Lightpanda 路径",
    status: "error",
    required: false,
    message: `配置中的可执行文件不存在：${runtime.configuredPath}`,
    fix_hint: "修正配置里的路径，或清空后改用环境变量 / 默认缓存目录。",
  });
}

if (isWindowsNative) {
  pushItem({
    key: "lightpanda-runtime",
    category: "environment",
    label: "Lightpanda 运行时",
    status: runtime.resolvedPath ? "ok" : "missing",
    message: runtime.resolvedPath
      ? `检测到可用的 Lightpanda 可执行文件（${describeResolvedSource(runtime.resolvedSource)}）`
      : `Windows 原生环境还缺少 Lightpanda 可执行文件。默认缓存目录：${lightpandaDefaultPath}`,
    fix_hint: runtime.resolvedPath ? undefined : windowsRuntimeGuidance(),
  });
} else {
  pushItem({
    key: "lightpanda-runtime",
    category: "environment",
    label: "Lightpanda 运行时",
    status: runtime.resolvedPath ? "ok" : "missing",
    message: runtime.resolvedPath
      ? `检测到可用的 Lightpanda 可执行文件（${describeResolvedSource(runtime.resolvedSource)}）`
      : "未检测到 Lightpanda 可执行文件",
    fix_hint: runtime.resolvedPath
      ? undefined
      : "请先自行补齐运行时，或在配置中显式填写路径。",
  });
}

const issues = items.filter((item) => item.status !== "ok" && item.status !== "skipped");

let status = "ready";
if (issues.some((item) => item.category === "config")) {
  status = "missing_config";
} else if (issues.some((item) => item.required && item.category !== "config")) {
  status = "env_missing";
} else if (issues.some((item) => item.status === "error")) {
  status = "error";
} else if (issues.length > 0) {
  status = "partial";
}

const summary = (() => {
  if (issues.length === 0) {
    return "web-search 已就绪。";
  }
  if (status === "missing_config") {
    return "配置存在问题，请修正后重新检测。";
  }
  if (status === "env_missing") {
    return "运行环境未满足，请先补齐依赖或运行时。";
  }
  if (status === "partial") {
    return "部分检查未通过。";
  }
  return "技能检测失败。";
})();

console.log(JSON.stringify({
  status,
  summary,
  items,
}));

if (issues.length > 0) {
  process.exit(1);
}
