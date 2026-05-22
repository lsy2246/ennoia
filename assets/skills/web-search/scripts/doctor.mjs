#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";
import {
  browserRuntimeGuidance,
  describeResolvedSource,
  resolveBrowserRuntime,
} from "./runtime.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = typeof process.env.ENNOIA_SKILL_ROOT === "string" && process.env.ENNOIA_SKILL_ROOT.trim()
  ? process.env.ENNOIA_SKILL_ROOT.trim()
  : path.resolve(scriptDir, "..");
const runtime = resolveBrowserRuntime();

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

function packageExists(packagePath) {
  return existsSync(path.join(skillRoot, "node_modules", ...packagePath, "package.json"));
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
  key: "browser-control",
  category: "config",
  label: "浏览器控制方式",
  status: runtime.issue?.code === "unsupported_browser_control" ? "error" : "ok",
  message: runtime.controlMode === "mcp"
    ? "使用 MCP 浏览器连接。"
    : "使用本地自动化浏览器。",
  fix_hint: runtime.issue?.code === "unsupported_browser_control"
    ? browserRuntimeGuidance(runtime)
    : undefined,
});

if (runtime.controlMode === "mcp") {
  pushItem({
    key: "mcp-transport",
    category: "config",
    label: "MCP 传输方式",
    status: runtime.issue?.code === "unsupported_mcp_transport" ? "error" : "ok",
    message: `使用 MCP 传输方式：${runtime.mcpTransport}`,
    fix_hint: runtime.issue?.code === "unsupported_mcp_transport"
      ? browserRuntimeGuidance(runtime)
      : undefined,
  });
  pushItem({
    key: "mcp-url",
    category: "config",
    label: "MCP 服务地址",
    status: runtime.mcpUrl
      ? runtime.issue?.code === "mcp_url_invalid" ? "error" : "ok"
      : "missing",
    message: runtime.mcpUrl
      ? `已配置 MCP 服务地址：${runtime.mcpUrl}`
      : "已选择 MCP 浏览器连接，但尚未填写 MCP 服务地址。",
    fix_hint: runtime.mcpUrl && runtime.issue?.code !== "mcp_url_invalid"
      ? undefined
      : browserRuntimeGuidance(runtime),
  });
  pushItem({
    key: "mcp-provider",
    category: "environment",
    label: "MCP 浏览器服务",
    status: runtime.ready ? "warning" : "skipped",
    required: false,
    message: runtime.ready
      ? "已填写 MCP 服务连接信息；实际连通性、能力发现与工具调用由后续 MCP provider 接入负责。"
      : "MCP 服务连接信息不完整时跳过 provider 连通性检查。",
    fix_hint: runtime.ready
      ? "确认该 MCP 服务可访问，并暴露浏览器搜索或页面访问能力。"
      : undefined,
  });
} else {
  for (const dependency of [
    { key: "cloakbrowser", label: "cloakbrowser", path: ["cloakbrowser"] },
    { key: "playwright-core", label: "playwright-core", path: ["playwright-core"] },
    { key: "readability", label: "@mozilla/readability", path: ["@mozilla", "readability"] },
    { key: "cheerio", label: "cheerio", path: ["cheerio"] },
    { key: "linkedom", label: "linkedom", path: ["linkedom"] },
  ]) {
    const status = packageExists(dependency.path) ? "ok" : "missing";
    pushItem({
      key: dependency.key,
      category: "dependency",
      label: dependency.label,
      status,
      message: status === "ok" ? `${dependency.label} 已安装` : `${dependency.label} 未安装`,
      fix_hint: "请先在技能目录安装依赖，再重新尝试。",
    });
  }

  if (runtime.mode === "system_path" && runtime.configuredPath && !runtime.configuredExists) {
    pushItem({
      key: "browser-config-path",
      category: "config",
      label: "浏览器内核路径",
      status: "error",
      required: false,
      message: `配置中的浏览器内核可执行文件不存在：${runtime.configuredPath}`,
      fix_hint: browserRuntimeGuidance(runtime),
    });
  }

  pushItem({
    key: "browser-runtime",
    category: runtime.issue?.category || (runtime.mode === "builtin" ? "dependency" : "environment"),
    label: "浏览器内核",
    status: runtime.ready ? "ok" : runtime.issue?.category === "config" ? "error" : "missing",
    message: (() => {
      if (runtime.mode === "builtin") {
        return runtime.ready
          ? `使用已准备的 Ennoia 内置 CloakBrowser Chromium：${runtime.resolvedPath}`
          : runtime.issue?.message || "Ennoia 内置 CloakBrowser Chromium 尚未准备。";
      }
      if (runtime.issue?.message) {
        return runtime.issue.message;
      }
      if (runtime.resolvedPath) {
        return `使用${runtime.browserKernelName}（${describeResolvedSource(runtime.resolvedSource)}）：${runtime.resolvedPath}`;
      }
      if (runtime.mode === "system_path") {
        return runtime.configuredPath
          ? "已填写浏览器内核路径，但该路径不可用；system_path 不会回退到其他内核。"
          : "已选择手动路径模式，但尚未填写浏览器内核路径。";
      }
      return "已选择自动查找模式，但未发现可用的系统浏览器内核。";
    })(),
    fix_hint: runtime.ready ? undefined : browserRuntimeGuidance(runtime),
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
    return "运行环境未满足，请先安装依赖或调整浏览器内核来源。";
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
  actions: runtime.controlMode === "local" && runtime.mode === "builtin" && !runtime.ready
    ? [{ key: "prepare-browser", label: "准备内置浏览器", kind: "prepare" }]
    : [],
}));

if (status !== "ready" && status !== "partial") {
  process.exit(1);
}
