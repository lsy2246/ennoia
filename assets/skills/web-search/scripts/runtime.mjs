#!/usr/bin/env node

import { existsSync } from "node:fs";
import path from "node:path";
import process from "node:process";

export const isWindowsNative = process.platform === "win32";

export const lightpandaDefaultPath = path.join(
  process.env.USERPROFILE || process.env.HOME || "",
  ".cache",
  "lightpanda-node",
  isWindowsNative ? "lightpanda.exe" : "lightpanda",
);

function normalizePath(value) {
  return typeof value === "string" ? value.trim() : "";
}

export function readSkillConfigFromEnv() {
  try {
    return JSON.parse(process.env.ENNOIA_SKILL_CONFIG_JSON || "{}");
  } catch {
    return {};
  }
}

export function resolveLightpandaRuntime(config = readSkillConfigFromEnv()) {
  const configuredPath = normalizePath(config.lightpanda_executable_path);
  const envPath = normalizePath(process.env.LIGHTPANDA_EXECUTABLE_PATH);
  const defaultPath = normalizePath(lightpandaDefaultPath);

  const configuredExists = configuredPath ? existsSync(configuredPath) : false;
  const envExists = envPath ? existsSync(envPath) : false;
  const defaultExists = defaultPath ? existsSync(defaultPath) : false;

  if (configuredExists) {
    return {
      configuredPath,
      configuredExists,
      envPath,
      envExists,
      defaultPath,
      defaultExists,
      resolvedPath: configuredPath,
      resolvedSource: "config",
    };
  }

  if (envExists) {
    return {
      configuredPath,
      configuredExists,
      envPath,
      envExists,
      defaultPath,
      defaultExists,
      resolvedPath: envPath,
      resolvedSource: "env",
    };
  }

  if (defaultExists) {
    return {
      configuredPath,
      configuredExists,
      envPath,
      envExists,
      defaultPath,
      defaultExists,
      resolvedPath: defaultPath,
      resolvedSource: "default",
    };
  }

  return {
    configuredPath,
    configuredExists,
    envPath,
    envExists,
    defaultPath,
    defaultExists,
    resolvedPath: "",
    resolvedSource: null,
  };
}

export function describeResolvedSource(source) {
  switch (source) {
    case "config":
      return "技能配置";
    case "env":
      return "环境变量";
    case "default":
      return "默认缓存目录";
    default:
      return "未知来源";
  }
}

export function windowsRuntimeGuidance() {
  return [
    "Windows 原生环境下需要额外提供 Lightpanda 可执行文件。",
    "可选方案：",
    "1. 在 WSL2 中自行安装 skill 依赖后运行 `node scripts/search-runner.mjs`。",
    "2. 手动准备一个可用的 Lightpanda 可执行文件。",
    `3. 把路径写入技能配置项 \`lightpanda_executable_path\`，或设置 \`LIGHTPANDA_EXECUTABLE_PATH\`。`,
    `4. 如果你希望走默认缓存目录，可把文件放到：${lightpandaDefaultPath}`,
  ].join("\n");
}
