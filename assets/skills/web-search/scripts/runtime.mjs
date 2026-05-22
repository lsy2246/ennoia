#!/usr/bin/env node

import { accessSync, constants, existsSync, readdirSync } from "node:fs";
import os from "node:os";
import path from "node:path";
import process from "node:process";

export const browserEngineId = "builtin-browser";
export const browserEngineName = "内核自动化浏览器";
export const browserDriverId = "cloakbrowser";

export const browserControlModes = new Set(["local", "mcp"]);
export const defaultBrowserControlMode = "local";
export const browserKernelModes = new Set(["builtin", "system_auto", "system_path"]);
export const defaultBrowserKernelMode = "builtin";
export const mcpTransportModes = new Set(["streamable-http", "sse"]);
export const defaultMcpTransportMode = "streamable-http";

function normalizeText(value) {
  return typeof value === "string" ? value.trim() : "";
}

function expandHome(input) {
  if (!input || (input !== "~" && !input.startsWith("~/") && !input.startsWith("~\\"))) {
    return input;
  }
  if (input === "~") {
    return os.homedir();
  }
  return path.join(os.homedir(), input.slice(2));
}

function defaultEnnoiaHome() {
  return path.join(process.env.USERPROFILE || process.env.HOME || os.homedir() || ".", ".ennoia");
}

export function ennoiaHome() {
  return expandHome(normalizeText(process.env.ENNOIA_HOME)) || defaultEnnoiaHome();
}

export function webSearchSkillDataDir() {
  return (
    expandHome(normalizeText(process.env.ENNOIA_SKILL_DATA_DIR))
    || path.join(ennoiaHome(), "data", "skills", "web-search")
  );
}

export function webSearchCloakBrowserCacheDir() {
  return path.join(webSearchSkillDataDir(), "cloakbrowser");
}

function cloakBrowserExecutablePath(binaryDir) {
  if (process.platform === "darwin") {
    return path.join(binaryDir, "Chromium.app", "Contents", "MacOS", "Chromium");
  }
  if (process.platform === "win32") {
    return path.join(binaryDir, "chrome.exe");
  }
  return path.join(binaryDir, "chrome");
}

function executableExists(filePath) {
  if (!filePath || !existsSync(filePath)) {
    return false;
  }
  if (process.platform === "win32") {
    return true;
  }
  try {
    accessSync(filePath, constants.X_OK);
    return true;
  } catch {
    return false;
  }
}

export function resolvePreparedCloakBrowserBinary(cacheDir = webSearchCloakBrowserCacheDir()) {
  if (!existsSync(cacheDir)) {
    return null;
  }

  const candidates = readdirSync(cacheDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory() && entry.name.startsWith("chromium-"))
    .map((entry) => {
      const version = entry.name.replace(/^chromium-/, "");
      const binaryPath = cloakBrowserExecutablePath(path.join(cacheDir, entry.name));
      return {
        version,
        binaryPath,
        ready: executableExists(binaryPath),
      };
    })
    .filter((entry) => entry.ready)
    .sort((left, right) => right.version.localeCompare(left.version, undefined, { numeric: true }));

  return candidates[0] || null;
}

export function resolveBuiltinBrowserCache() {
  const cacheDir = webSearchCloakBrowserCacheDir();
  const prepared = resolvePreparedCloakBrowserBinary(cacheDir);
  return {
    cacheDir,
    binaryPath: prepared?.binaryPath || "",
    version: prepared?.version || "",
    ready: Boolean(prepared),
  };
}

function candidate(id, name, executablePath) {
  return {
    id,
    name,
    path: executablePath,
    exists: Boolean(executablePath) && existsSync(executablePath),
  };
}

function windowsCandidates() {
  const programFiles = process.env.ProgramFiles || "C:\\Program Files";
  const programFilesX86 = process.env["ProgramFiles(x86)"] || "C:\\Program Files (x86)";
  const localAppData = process.env.LOCALAPPDATA || path.join(os.homedir(), "AppData", "Local");

  return [
    candidate("chrome", "Google Chrome", path.join(programFiles, "Google", "Chrome", "Application", "chrome.exe")),
    candidate("chrome", "Google Chrome", path.join(programFilesX86, "Google", "Chrome", "Application", "chrome.exe")),
    candidate("chrome", "Google Chrome", path.join(localAppData, "Google", "Chrome", "Application", "chrome.exe")),
    candidate("edge", "Microsoft Edge", path.join(programFiles, "Microsoft", "Edge", "Application", "msedge.exe")),
    candidate("edge", "Microsoft Edge", path.join(programFilesX86, "Microsoft", "Edge", "Application", "msedge.exe")),
    candidate("edge", "Microsoft Edge", path.join(localAppData, "Microsoft", "Edge", "Application", "msedge.exe")),
    candidate("brave", "Brave", path.join(programFiles, "BraveSoftware", "Brave-Browser", "Application", "brave.exe")),
    candidate("brave", "Brave", path.join(programFilesX86, "BraveSoftware", "Brave-Browser", "Application", "brave.exe")),
    candidate("brave", "Brave", path.join(localAppData, "BraveSoftware", "Brave-Browser", "Application", "brave.exe")),
    candidate("chromium", "Chromium", path.join(programFiles, "Chromium", "Application", "chrome.exe")),
    candidate("chromium", "Chromium", path.join(programFilesX86, "Chromium", "Application", "chrome.exe")),
    candidate("chromium", "Chromium", path.join(localAppData, "Chromium", "Application", "chrome.exe")),
  ];
}

function macCandidates() {
  return [
    candidate("chrome", "Google Chrome", "/Applications/Google Chrome.app/Contents/MacOS/Google Chrome"),
    candidate("edge", "Microsoft Edge", "/Applications/Microsoft Edge.app/Contents/MacOS/Microsoft Edge"),
    candidate("brave", "Brave", "/Applications/Brave Browser.app/Contents/MacOS/Brave Browser"),
    candidate("chromium", "Chromium", "/Applications/Chromium.app/Contents/MacOS/Chromium"),
    candidate("chrome-canary", "Google Chrome Canary", "/Applications/Google Chrome Canary.app/Contents/MacOS/Google Chrome Canary"),
  ];
}

function linuxCandidates() {
  const names = [
    ["chrome", "Google Chrome", "google-chrome"],
    ["chrome", "Google Chrome", "google-chrome-stable"],
    ["chromium", "Chromium", "chromium"],
    ["chromium", "Chromium", "chromium-browser"],
    ["edge", "Microsoft Edge", "microsoft-edge"],
    ["edge", "Microsoft Edge", "microsoft-edge-stable"],
    ["brave", "Brave", "brave-browser"],
  ];

  const pathDirs = (process.env.PATH || "").split(path.delimiter).filter(Boolean);
  return names.flatMap(([id, name, executableName]) => {
    return pathDirs.map((dir) => candidate(id, name, path.join(dir, executableName)));
  });
}

export function browserKernelCandidates() {
  if (process.platform === "win32") {
    return windowsCandidates();
  }
  if (process.platform === "darwin") {
    return macCandidates();
  }
  return linuxCandidates();
}

export function findSystemBrowserKernel() {
  return browserKernelCandidates().find((item) => item.exists) || null;
}

export function readSkillConfigFromEnv() {
  try {
    return JSON.parse(process.env.ENNOIA_SKILL_CONFIG_JSON || "{}");
  } catch {
    return {};
  }
}

export function normalizeBrowserKernelMode(value) {
  return normalizeText(value) || defaultBrowserKernelMode;
}

export function normalizeBrowserControlMode(value) {
  return normalizeText(value) || defaultBrowserControlMode;
}

export function normalizeMcpTransportMode(value) {
  return normalizeText(value) || defaultMcpTransportMode;
}

function isHttpUrl(value) {
  try {
    const url = new URL(value);
    return url.protocol === "http:" || url.protocol === "https:";
  } catch {
    return false;
  }
}

export function resolveBrowserRuntime(config = readSkillConfigFromEnv()) {
  const controlMode = normalizeBrowserControlMode(config.browser_control);
  const mcpTransport = normalizeMcpTransportMode(config.mcp_transport);
  const mcpUrl = normalizeText(config.mcp_url);
  const mode = normalizeBrowserKernelMode(config.browser_kernel);
  const configuredPath = expandHome(normalizeText(config.browser_executable_path));
  const configuredExists = configuredPath ? existsSync(configuredPath) : false;
  const discovered = mode === "system_auto" ? findSystemBrowserKernel() : null;
  const builtinCache = resolveBuiltinBrowserCache();

  if (!browserControlModes.has(controlMode)) {
    return {
      controlMode,
      mode,
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: "",
      resolvedSource: null,
      browserKernelId: "",
      browserKernelName: "",
      ready: false,
      issue: {
        category: "config",
        code: "unsupported_browser_control",
        message: `不支持的浏览器控制方式：${controlMode}`,
      },
    };
  }

  if (controlMode === "mcp") {
    if (!mcpTransportModes.has(mcpTransport)) {
      return {
        controlMode,
        mode: "mcp",
        cacheDir: builtinCache.cacheDir,
        configuredPath,
        configuredExists,
        discovered,
        mcpTransport,
        mcpUrl,
        resolvedPath: "",
        resolvedSource: "mcp",
        browserKernelId: "",
        browserKernelName: "",
        ready: false,
        issue: {
          category: "config",
          code: "unsupported_mcp_transport",
          message: `不支持的 MCP 传输方式：${mcpTransport}`,
        },
      };
    }

    if (!mcpUrl) {
      return {
        controlMode,
        mode: "mcp",
        cacheDir: builtinCache.cacheDir,
        configuredPath,
        configuredExists,
        discovered,
        mcpTransport,
        mcpUrl,
        resolvedPath: "",
        resolvedSource: "mcp",
        browserKernelId: "",
        browserKernelName: "",
        ready: false,
        issue: {
          category: "config",
          code: "mcp_url_missing",
          message: "已选择 MCP 浏览器连接，但尚未填写 mcp_url。",
        },
      };
    }

    if (!isHttpUrl(mcpUrl)) {
      return {
        controlMode,
        mode: "mcp",
        cacheDir: builtinCache.cacheDir,
        configuredPath,
        configuredExists,
        discovered,
        mcpTransport,
        mcpUrl,
        resolvedPath: "",
        resolvedSource: "mcp",
        browserKernelId: "",
        browserKernelName: "",
        ready: false,
        issue: {
          category: "config",
          code: "mcp_url_invalid",
          message: `MCP 服务地址必须是 http 或 https URL：${mcpUrl}`,
        },
      };
    }

    return {
      controlMode,
      mode: "mcp",
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: "",
      resolvedSource: "mcp",
      browserKernelId: "",
      browserKernelName: "MCP 浏览器服务",
      ready: true,
      issue: null,
    };
  }

  if (!browserKernelModes.has(mode)) {
    return {
      controlMode,
      mode,
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: "",
      resolvedSource: null,
      browserKernelId: "",
      browserKernelName: "",
      ready: false,
      issue: {
        category: "config",
        code: "unsupported_browser_kernel",
        message: `不支持的浏览器内核来源：${mode}`,
      },
    };
  }

  if (mode === "builtin") {
    if (!builtinCache.ready) {
      return {
        controlMode,
        mode,
        cacheDir: builtinCache.cacheDir,
        configuredPath,
        configuredExists,
        discovered,
        mcpTransport,
        mcpUrl,
        resolvedPath: "",
        resolvedSource: "builtin",
        browserKernelId: "builtin",
        browserKernelName: "CloakBrowser Chromium",
        ready: false,
        issue: {
          category: "dependency",
          code: "builtin_browser_not_prepared",
          message: `内置 CloakBrowser Chromium 尚未准备：${builtinCache.cacheDir}`,
        },
      };
    }

    return {
      controlMode,
      mode,
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: builtinCache.binaryPath,
      resolvedSource: "builtin",
      browserKernelId: "builtin",
      browserKernelName: "CloakBrowser Chromium",
      ready: true,
      issue: null,
    };
  }

  if (mode === "system_auto" && discovered) {
    return {
      controlMode,
      mode,
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: discovered.path,
      resolvedSource: "system_auto",
      browserKernelId: discovered.id,
      browserKernelName: discovered.name,
      ready: true,
      issue: null,
    };
  }

  if (mode === "system_path" && configuredExists) {
    return {
      controlMode,
      mode,
      cacheDir: builtinCache.cacheDir,
      configuredPath,
      configuredExists,
      discovered,
      mcpTransport,
      mcpUrl,
      resolvedPath: configuredPath,
      resolvedSource: "system_path",
      browserKernelId: "custom",
      browserKernelName: "用户配置浏览器",
      ready: true,
      issue: null,
    };
  }

  const issue = (() => {
    if (mode === "system_path" && !configuredPath) {
      return {
        category: "config",
        code: "browser_executable_path_missing",
        message: "已选择手动浏览器路径，但尚未填写 browser_executable_path。",
      };
    }
    if (mode === "system_path" && !configuredExists) {
      return {
        category: "config",
        code: "browser_executable_path_not_found",
        message: `配置中的浏览器可执行文件不存在：${configuredPath}`,
      };
    }
    return {
      category: "environment",
      code: "system_browser_not_found",
      message: "已选择自动查找系统浏览器，但未发现 Chrome、Edge、Brave 或 Chromium。",
    };
  })();

  return {
    controlMode,
    mode,
    cacheDir: builtinCache.cacheDir,
    configuredPath,
    configuredExists,
    discovered,
    mcpTransport,
    mcpUrl,
    resolvedPath: "",
    resolvedSource: null,
    browserKernelId: "",
    browserKernelName: "",
    ready: false,
    issue,
  };
}

export function applyBrowserRuntimeEnv(runtime = resolveBrowserRuntime()) {
  if (!runtime.ready) {
    throw new Error(browserRuntimeGuidance(runtime));
  }

  if (runtime.controlMode === "mcp") {
    return runtime;
  }

  if (runtime.mode === "builtin") {
    process.env.CLOAKBROWSER_CACHE_DIR = runtime.cacheDir || webSearchCloakBrowserCacheDir();
    process.env.CLOAKBROWSER_BINARY_PATH = runtime.resolvedPath;
    return runtime;
  }

  process.env.CLOAKBROWSER_BINARY_PATH = runtime.resolvedPath;
  return runtime;
}

export function applyBuiltinBrowserPrepareEnv() {
  const cacheDir = webSearchCloakBrowserCacheDir();
  process.env.CLOAKBROWSER_CACHE_DIR = cacheDir;
  delete process.env.CLOAKBROWSER_BINARY_PATH;
  return cacheDir;
}

export function describeResolvedSource(source) {
  switch (source) {
    case "builtin":
      return "内置 CloakBrowser";
    case "system_auto":
      return "系统自动查找";
    case "system_path":
      return "用户手动路径";
    default:
      return "未解析";
  }
}

export function browserRuntimeGuidance(runtime = resolveBrowserRuntime()) {
  if (runtime.controlMode === "mcp") {
    const lines = [
      "浏览器控制方式为 mcp 时，web-search 直接连接本技能配置的 MCP 浏览器服务。",
      "需要填写 mcp_transport 与 mcp_url；mcp_url 必须是 http 或 https 地址。",
    ];
    if (runtime.issue?.message) {
      lines.push(runtime.issue.message);
    }
    if (!runtime.mcpTransport) {
      lines.push("当前缺少 mcp_transport。");
    } else if (!mcpTransportModes.has(runtime.mcpTransport)) {
      lines.push(`当前支持的 mcp_transport：${Array.from(mcpTransportModes).join("、")}。`);
    }
    if (!runtime.mcpUrl) {
      lines.push("当前缺少 mcp_url。");
    }
    return lines.join("\n");
  }

  const lines = [
    "浏览器控制方式必须是 local 或 mcp；local 模式下浏览器内核来源必须三选一且互斥：builtin、system_auto 或 system_path。",
    "builtin 使用 Ennoia 管理的 CloakBrowser Chromium 缓存；system_auto 自动查找 Chrome、Edge、Brave 或 Chromium；system_path 使用 browser_executable_path。",
  ];

  if (runtime.issue?.message) {
    lines.push(runtime.issue.message);
  }

  if (runtime.mode === "system_auto" && !runtime.discovered) {
    lines.push("当前未自动发现可用的系统浏览器内核。");
  }

  if (runtime.mode === "builtin" && !runtime.ready) {
    lines.push(`当前内置浏览器缓存目录：${runtime.cacheDir || webSearchCloakBrowserCacheDir()}`);
    lines.push("请先在 web-search 技能目录运行：node scripts/prepare-browser.mjs");
  }

  if (runtime.mode === "system_path") {
    if (!runtime.configuredPath) {
      lines.push("当前选择了 system_path，但尚未填写 browser_executable_path。");
    } else if (!runtime.configuredExists) {
      lines.push(`当前配置路径不存在：${runtime.configuredPath}`);
    }
  }

  return lines.join("\n");
}
