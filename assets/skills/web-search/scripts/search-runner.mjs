#!/usr/bin/env node

import process from "node:process";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { load as loadHtml } from "cheerio";
import { DOMParser } from "linkedom";
import { Readability } from "@mozilla/readability";

import { isWindowsNative, resolveLightpandaRuntime, windowsRuntimeGuidance } from "./runtime.mjs";

const scriptDir = path.dirname(fileURLToPath(import.meta.url));
const skillRoot = path.resolve(scriptDir, "..");
let lightpandaRuntime = null;

const DEFAULT_LIMIT = 5;
const DEFAULT_PAGES = 3;
const DEFAULT_TEXT_LIMIT = 2400;
const SEARCH_BASE = "https://html.duckduckgo.com/html/";

function usage() {
  console.log(`用法:
  node scripts/search-runner.mjs "query"
  node scripts/search-runner.mjs "query" --limit 5 --pages 3 --format json

参数:
  --limit <n>    搜索结果上限，默认 ${DEFAULT_LIMIT}
  --pages <n>    继续抓取的详情页上限，默认 ${DEFAULT_PAGES}
  --format <f>   输出格式：json | markdown，默认 json
  --help         显示帮助
`);
}

function parseArgs(argv) {
  const args = {
    query: "",
    limit: DEFAULT_LIMIT,
    pages: DEFAULT_PAGES,
    format: "json",
  };

  const rest = [];
  for (let index = 0; index < argv.length; index += 1) {
    const token = argv[index];
    if (token === "--help" || token === "-h") {
      args.help = true;
      continue;
    }
    if (token === "--limit") {
      args.limit = Number.parseInt(argv[index + 1] || "", 10) || DEFAULT_LIMIT;
      index += 1;
      continue;
    }
    if (token === "--pages") {
      args.pages = Number.parseInt(argv[index + 1] || "", 10) || DEFAULT_PAGES;
      index += 1;
      continue;
    }
    if (token === "--format") {
      args.format = argv[index + 1] || "json";
      index += 1;
      continue;
    }
    rest.push(token);
  }

  args.query = rest.join(" ").trim();
  return args;
}

function ensureRuntime() {
  const runtime = resolveLightpandaRuntime();
  if (runtime.resolvedPath) {
    process.env.LIGHTPANDA_EXECUTABLE_PATH = runtime.resolvedPath;
    return;
  }

  if (isWindowsNative) {
    throw new Error(
      windowsRuntimeGuidance(),
    );
  }
}

function normalizeText(input, limit = DEFAULT_TEXT_LIMIT) {
  return input.replace(/\s+/g, " ").trim().slice(0, limit);
}

function normalizeHref(rawHref) {
  if (!rawHref) {
    return "";
  }

  try {
    const url = new URL(rawHref, SEARCH_BASE);
    const redirect = url.searchParams.get("uddg");
    return redirect ? decodeURIComponent(redirect) : url.toString();
  } catch {
    return "";
  }
}

async function fetchHtml(url) {
  const html = await lightpandaRuntime.fetch(url, {
    dump: true,
    dumpOptions: { type: "html" },
  });
  return String(html);
}

function parseSearchResults(html, limit) {
  const $ = loadHtml(html);
  const results = [];
  const seen = new Set();

  $("a.result__a, .result__title a, a[data-testid='result-title-a']").each((_, element) => {
    if (results.length >= limit) {
      return;
    }

    const anchor = $(element);
    const title = normalizeText(anchor.text(), 240);
    const href = normalizeHref(anchor.attr("href"));

    if (!title || !href || seen.has(href)) {
      return;
    }

    seen.add(href);

    const container = anchor.closest(".result, .web-result, .result__body");
    const snippet = normalizeText(
      container.find(".result__snippet, .result-snippet, .snippet").first().text(),
      360,
    );

    results.push({
      rank: results.length + 1,
      title,
      url: href,
      snippet,
    });
  });

  return results;
}

function extractMetadata($) {
  const selectors = [
    "meta[property='article:published_time']",
    "meta[name='article:published_time']",
    "meta[property='og:updated_time']",
    "meta[name='pubdate']",
    "meta[name='publish-date']",
    "time[datetime]",
  ];

  for (const selector of selectors) {
    const node = $(selector).first();
    const content = node.attr("content") || node.attr("datetime") || node.text();
    const normalized = normalizeText(content || "", 120);
    if (normalized) {
      return normalized;
    }
  }

  return "";
}

function extractLinks($, baseUrl, limit = 12) {
  const links = [];
  const seen = new Set();

  $("a[href]").each((_, element) => {
    if (links.length >= limit) {
      return;
    }

    const href = $(element).attr("href");
    const text = normalizeText($(element).text(), 120);
    if (!href || !text) {
      return;
    }

    try {
      const absolute = new URL(href, baseUrl).toString();
      if (seen.has(absolute)) {
        return;
      }
      seen.add(absolute);
      links.push({ text, url: absolute });
    } catch {
      // Ignore malformed links.
    }
  });

  return links;
}

function extractArticle(html, url) {
  const document = new DOMParser().parseFromString(html, "text/html");
  const article = new Readability(document).parse();
  const $ = loadHtml(html);

  return {
    url,
    title: normalizeText(article?.title || $("title").first().text(), 240),
    excerpt: normalizeText(
      article?.excerpt || $("meta[name='description']").attr("content") || "",
      360,
    ),
    byline: normalizeText(article?.byline || "", 160),
    published_at: extractMetadata($),
    site_name: normalizeText(
      $("meta[property='og:site_name']").attr("content") || new URL(url).hostname,
      120,
    ),
    content_text: normalizeText(article?.textContent || "", DEFAULT_TEXT_LIMIT),
    links: extractLinks($, url),
  };
}

async function fetchPages(results, pageLimit) {
  const pages = [];
  for (const result of results.slice(0, pageLimit)) {
    try {
      const html = await fetchHtml(result.url);
      pages.push({
        rank: result.rank,
        source_result: result,
        page: extractArticle(html, result.url),
      });
    } catch (error) {
      pages.push({
        rank: result.rank,
        source_result: result,
        error: error instanceof Error ? error.message : String(error),
      });
    }
  }
  return pages;
}

function toMarkdown(payload) {
  const lines = [];
  lines.push(`# Web Search`);
  lines.push("");
  lines.push(`- 查询：${payload.query}`);
  lines.push(`- 引擎：${payload.engine}`);
  lines.push(`- 候选结果：${payload.results.length}`);
  lines.push(`- 抓取页面：${payload.pages.length}`);
  lines.push("");
  lines.push("## 搜索结果");
  lines.push("");

  for (const result of payload.results) {
    lines.push(`### ${result.rank}. ${result.title}`);
    lines.push(`- URL: ${result.url}`);
    if (result.snippet) {
      lines.push(`- 摘要: ${result.snippet}`);
    }
    lines.push("");
  }

  lines.push("## 页面提取");
  lines.push("");

  for (const item of payload.pages) {
    if (item.error) {
      lines.push(`### ${item.rank}. ${item.source_result.title}`);
      lines.push(`- URL: ${item.source_result.url}`);
      lines.push(`- 错误: ${item.error}`);
      lines.push("");
      continue;
    }

    lines.push(`### ${item.rank}. ${item.page.title || item.source_result.title}`);
    lines.push(`- URL: ${item.page.url}`);
    if (item.page.site_name) {
      lines.push(`- 站点: ${item.page.site_name}`);
    }
    if (item.page.published_at) {
      lines.push(`- 时间: ${item.page.published_at}`);
    }
    if (item.page.excerpt) {
      lines.push(`- 摘要: ${item.page.excerpt}`);
    }
    if (item.page.content_text) {
      lines.push("");
      lines.push(item.page.content_text);
    }
    lines.push("");
  }

  return lines.join("\n");
}

async function main() {
  const args = parseArgs(process.argv.slice(2));
  if (args.help || !args.query) {
    usage();
    process.exit(args.help ? 0 : 1);
  }

  if (!["json", "markdown"].includes(args.format)) {
    throw new Error("`--format` 只支持 json 或 markdown。");
  }

  ensureRuntime();
  ({ lightpanda: lightpandaRuntime } = await import("@lightpanda/browser"));

  const searchUrl = `${SEARCH_BASE}?q=${encodeURIComponent(args.query)}`;
  const searchHtml = await fetchHtml(searchUrl);
  const results = parseSearchResults(searchHtml, args.limit);
  const pages = await fetchPages(results, args.pages);

  const payload = {
    query: args.query,
    engine: "lightpanda",
    runner: path.relative(skillRoot, path.join(scriptDir, "search-runner.mjs")).replaceAll("\\", "/"),
    search_url: searchUrl,
    results,
    pages,
  };

  if (args.format === "markdown") {
    console.log(toMarkdown(payload));
    return;
  }

  console.log(JSON.stringify(payload, null, 2));
}

main().catch((error) => {
  console.error(error instanceof Error ? error.message : String(error));
  process.exit(1);
});
