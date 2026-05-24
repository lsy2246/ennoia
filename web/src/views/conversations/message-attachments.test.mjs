import { describe, expect, test } from "bun:test";

import {
  createMessageAttachmentDownloadClickHandler,
  getMessageAttachmentDownloadName,
  resolveMessageAttachmentLinkProps,
  resolveMessageAttachmentDownloadUrl,
  resolveMessageAttachmentUrl,
} from "./message-attachments.ts";

describe("conversation message attachments", () => {
  test("derives a download filename from generated artifact urls", () => {
    expect(getMessageAttachmentDownloadName(
      "/api/agents/lsy/artifacts/screenshots/Bilibili%20%E5%AE%98%E7%BD%91.png",
    )).toBe("Bilibili 官网.png");
    expect(getMessageAttachmentDownloadName("https://www.bilibili.com")).toBeUndefined();
  });

  test("does not treat non-canonical artifact links as downloadable attachments", () => {
    expect(getMessageAttachmentDownloadName("sandbox:/artifacts/bilibili.png")).toBeUndefined();
    expect(getMessageAttachmentDownloadName("/artifacts/bilibili.png")).toBeUndefined();
  });

  test("resolves canonical artifact urls through the configured API base", () => {
    globalThis.__ENNOIA_API_BASE_URL__ = "http://127.0.0.1:3710";

    expect(resolveMessageAttachmentUrl(
      "/api/agents/lsy/artifacts/screenshots/Bilibili%20%E5%AE%98%E7%BD%91.png",
    )).toBe("http://127.0.0.1:3710/api/agents/lsy/artifacts/screenshots/Bilibili%20%E5%AE%98%E7%BD%91.png");

    delete globalThis.__ENNOIA_API_BASE_URL__;
  });

  test("resolves canonical artifact download links through a backend attachment response", () => {
    globalThis.__ENNOIA_API_BASE_URL__ = "http://127.0.0.1:3710";

    expect(resolveMessageAttachmentDownloadUrl(
      "/api/agents/lsy/artifacts/screenshots/bilibili.png",
    )).toBe("http://127.0.0.1:3710/api/agents/lsy/artifacts/screenshots/bilibili.png?download=1");
    expect(resolveMessageAttachmentDownloadUrl(
      "/api/agents/lsy/artifacts/screenshots/bilibili.png?v=2#preview",
    )).toBe("http://127.0.0.1:3710/api/agents/lsy/artifacts/screenshots/bilibili.png?v=2&download=1#preview");

    delete globalThis.__ENNOIA_API_BASE_URL__;
  });

  test("does not resolve absolute artifact urls as message attachments", () => {
    globalThis.__ENNOIA_API_BASE_URL__ = "http://127.0.0.1:3710";

    expect(resolveMessageAttachmentDownloadUrl(
      "http://127.0.0.1:3710/api/agents/lsy/artifacts/bilibili.png",
    )).toBeUndefined();
    expect(resolveMessageAttachmentUrl(
      "http://127.0.0.1:3710/api/agents/lsy/artifacts/bilibili.png",
    )).toBeUndefined();

    delete globalThis.__ENNOIA_API_BASE_URL__;
  });

  test("renders artifact links as downloads instead of new-tab navigation", () => {
    globalThis.__ENNOIA_API_BASE_URL__ = "http://127.0.0.1:3710";

    expect(resolveMessageAttachmentLinkProps(
      "/api/agents/lsy/artifacts/bilibili.png",
    )).toMatchObject({
      download: "bilibili.png",
      href: "http://127.0.0.1:3710/api/agents/lsy/artifacts/bilibili.png?download=1",
    });
    expect(resolveMessageAttachmentLinkProps("https://www.bilibili.com")).toEqual({
      href: "https://www.bilibili.com",
      rel: "noreferrer",
      target: "_blank",
    });

    delete globalThis.__ENNOIA_API_BASE_URL__;
  });

  test("intercepts artifact link clicks and delegates to the download handler", async () => {
    const calls = [];
    let prevented = false;
    const handler = createMessageAttachmentDownloadClickHandler(
      "http://127.0.0.1:3710/api/agents/lsy/artifacts/bilibili.png?download=1",
      "bilibili.png",
      async (url, filename) => {
        calls.push({ filename, url });
      },
    );

    await handler({
      preventDefault() {
        prevented = true;
      },
    });

    expect(prevented).toBe(true);
    expect(calls).toEqual([{
      filename: "bilibili.png",
      url: "http://127.0.0.1:3710/api/agents/lsy/artifacts/bilibili.png?download=1",
    }]);
  });

  test("does not resolve non-canonical artifact urls as message attachments", () => {
    globalThis.__ENNOIA_API_BASE_URL__ = "http://127.0.0.1:3710";

    expect(resolveMessageAttachmentUrl("sandbox:/artifacts/bilibili.png")).toBeUndefined();
    expect(resolveMessageAttachmentUrl("/artifacts/bilibili.png")).toBeUndefined();
    expect(resolveMessageAttachmentDownloadUrl("sandbox:/artifacts/bilibili.png")).toBeUndefined();
    expect(resolveMessageAttachmentDownloadUrl("/artifacts/bilibili.png")).toBeUndefined();

    delete globalThis.__ENNOIA_API_BASE_URL__;
  });
});
