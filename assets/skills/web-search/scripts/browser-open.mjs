#!/usr/bin/env node

import process from "node:process";
import {
  applyBrowserRuntimeEnv,
  browserDriverId,
  resolveBrowserRuntime,
} from "./runtime.mjs";

const DEFAULT_TIMEOUT_MS = 30000;

function usage() {
  console.log(`用法:
  node scripts/browser-open.mjs <url> [--headful] [--screenshot <path>]

参数:
  --headful              以有界面模式打开浏览器
  --screenshot <path>    打开页面后保存截图
  --help                 显示帮助
`);
}

function parseArgs(argv) {
  const args = {
    url: "",
    headless: true,
    screenshot: "",
  };

  const rest = [];
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--help" || token === "-h") {
      args.help = true;
      continue;
    }
    if (token === "--headful") {
      args.headless = false;
      continue;
    }
    if (token === "--screenshot") {
      args.screenshot = argv[index + 1] || "";
      index += 1;
      continue;
    }
    rest.push(token);
  }

  args.url = rest[0] || "";
  return args;
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || !args.url) {
    usage();
    process.exit(args.help ? 0 : 1);
  }

  const runtime = applyBrowserRuntimeEnv(resolveBrowserRuntime());
  if (runtime.controlMode === "mcp") {
    throw new Error("browser-open 只支持本地自动化浏览器调试；MCP 浏览器服务请通过 MCP 连接检测。");
  }

  const { launch } = await import("cloakbrowser");

  const browser = await launch({ headless: args.headless });
  try {
    const page = await browser.newPage();
    page.setDefaultTimeout(DEFAULT_TIMEOUT_MS);
    page.setDefaultNavigationTimeout(DEFAULT_TIMEOUT_MS);
    await page.goto(args.url, { waitUntil: "domcontentloaded", timeout: DEFAULT_TIMEOUT_MS });

    const title = await page.title();
    const payload = {
      url: page.url(),
      title,
      browser_kernel_mode: runtime.mode,
      browser_kernel: runtime.browserKernelId,
      browser_kernel_name: runtime.browserKernelName,
      runtime: {
        source: runtime.resolvedSource,
        driver: browserDriverId,
      },
      screenshot: args.screenshot || null,
    };

    if (args.screenshot) {
      await page.screenshot({ path: args.screenshot, fullPage: true });
    }

    console.log(JSON.stringify(payload, null, 2));
    await page.close().catch(() => {});
  } finally {
    await browser.close().catch(() => {});
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
