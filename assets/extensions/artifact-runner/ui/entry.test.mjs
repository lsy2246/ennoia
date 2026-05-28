import { describe, expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const currentDir = dirname(fileURLToPath(import.meta.url));
const entrySource = readFileSync(join(currentDir, "entry.tsx"), "utf8");
const styleSource = readFileSync(join(currentDir, "artifact-runner.css"), "utf8");

describe("artifact runner HTML preview", () => {
  test("allows scripts in the sandboxed iframe so canvas previews can draw", () => {
    expect(entrySource).toContain('sandbox="allow-scripts"');
    expect(entrySource).not.toContain("allow-same-origin");
  });

  test("injects a preview runtime without relaxing iframe origin isolation", () => {
    expect(entrySource).toContain("function injectPreviewRuntime");
    expect(entrySource).toContain("function installStorageShim");
    expect(entrySource).toContain("Object.defineProperty(window, name");
    expect(entrySource).toContain("artifact-runner-preview-error");
    expect(entrySource).toContain('window.addEventListener("error"');
    expect(entrySource).toContain('window.addEventListener("unhandledrejection"');
  });

  test("renders explicit HTML source artifacts as code instead of preview", () => {
    expect(entrySource).toContain('type === "html-source"');
    expect(entrySource).toContain('<HtmlArtifactBlock title={title} content={content} initialMode="source" />');
    expect(entrySource).toContain('initialMode="preview"');
    expect(entrySource).toContain('artifact-runner-tabs');
    expect(entrySource).toContain('artifact-runner-viewport');
    expect(entrySource).toContain('artifact-runner-source-scroll');
    expect(styleSource).toContain(".artifact-runner-card--html");
    expect(styleSource).toContain("grid-template-rows: auto auto minmax(0, 1fr);");
    expect(styleSource).toContain(".artifact-runner-viewport");
    expect(styleSource).toContain("height: clamp(260px, 52vh, 560px);");
    expect(styleSource).toContain(".artifact-runner-source-scroll");
    expect(styleSource).toContain("padding: 14px 14px 36px;");
    expect(styleSource).toContain("white-space: pre;");
  });

  test("provides an extension-owned large preview dialog for cramped HTML previews", () => {
    expect(entrySource).toContain("largePreviewOpen");
    expect(entrySource).toContain("artifact-runner-large-preview");
    expect(entrySource).toContain("打开大预览");
    expect(entrySource).toContain("关闭大预览");
    expect(entrySource).toContain("刷新");
    expect(entrySource).toContain("previewRevision");
    expect(styleSource).toContain(".artifact-runner-large-preview");
    expect(styleSource).toContain("width: min(1180px, 96vw);");
    expect(styleSource).toContain("height: 92vh;");
    expect(styleSource).toContain(".artifact-runner-large-preview__frame");
  });
});

describe("artifact runner Python artifact", () => {
  test("keeps Python execution inside artifact-runner extension RPC", () => {
    expect(entrySource).toContain("/api/extensions/artifact-runner/rpc/artifact.run_python");
    expect(entrySource).toContain("运行");
    expect(entrySource).toContain("stdout");
    expect(entrySource).toContain("stderr");
    expect(entrySource).toContain("退出码");
  });
});
