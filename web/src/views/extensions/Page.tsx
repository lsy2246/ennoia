import { useEffect, useRef, useState } from "react";
import { useRouterState } from "@tanstack/react-router";
import { apiUrl } from "@ennoia/api-client";

import { useUiHelpers, useUiStore } from "@/stores/ui";
import { loadExtensionPageMount } from "@/views/extensions/registry";

function pageIdFromPath(pathname: string) {
  const match = pathname.match(/^\/extension-pages\/([^/]+)$/);
  return match ? decodeURIComponent(match[1]) : "";
}

type ExtensionPageViewProps = {
  pageId?: string;
};

export function ExtensionPageView({ pageId: explicitPageId }: ExtensionPageViewProps = {}) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const helpers = useUiHelpers();
  const themeId = useUiStore((state) => state.themeId);
  const { formatDate, formatDateTime, formatTime, locale, runtime, resolveText, t } = helpers;
  const pageId = explicitPageId ?? pageIdFromPath(pathname);
  const page = runtime?.registry.pages.find((item) => item.page.id === pageId);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [mountStatus, setMountStatus] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [mountError, setMountError] = useState<string | null>(null);
  const panels = runtime?.registry.panels.filter((item) => item.extension_id === page?.extension_id) ?? [];
  const generation = runtime?.versions.registry ?? 0;

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void | Promise<void>) | undefined;
    const container = containerRef.current;
    setMountStatus("idle");
    setMountError(null);
    if (!page || !container) {
      return () => {
        cancelled = true;
      };
    }

    container.replaceChildren();
    setMountStatus("loading");
    void loadExtensionPageMount(page, generation)
      .then(async (mount) => {
        if (cancelled) {
          return;
        }
        if (!mount) {
          setMountStatus("idle");
          return;
        }
        const handle = await mount(container, {
          kind: "page",
          extensionId: page.extension_id,
          mount: page.page.mount,
          page,
          helpers: {
            locale,
            themeId,
            apiBaseUrl: apiUrl(""),
            t,
            formatDateTime,
            formatDate,
            formatTime,
          },
        });
        if (!cancelled) {
          cleanup = handle?.unmount;
          setMountStatus("ready");
        }
      })
      .catch((error) => {
        if (!cancelled) {
          setMountStatus("error");
          setMountError(String(error));
        }
      });

    return () => {
      cancelled = true;
      void cleanup?.();
    };
  }, [formatDate, formatDateTime, formatTime, generation, locale, page, t, themeId]);

  return (
    <div className="extension-view">
      {page ? <div ref={containerRef} data-extension-page={page.page.mount} /> : null}
      {mountStatus === "ready" ? null : (
      <section className="work-panel hero-empty">
        <span>{t("web.extension_page.eyebrow", "Extension View")}</span>
        <h1>{page ? resolveText(page.page.title) : t("web.extension_page.not_found", "扩展视图未找到")}</h1>
        <p>
          {mountStatus === "loading"
            ? t("web.extension_page.loading", "正在加载扩展 UI 模块。")
            : mountStatus === "error"
              ? mountError
              : t("web.extension_page.description", "这是由扩展注册表贡献并挂接到 Web 工作台的动态视图。")}
        </p>
        <div className="tag-row">
          <span>{page?.extension_id ?? "unknown"}</span>
          <span>{page?.page.mount ?? "no mount"}</span>
          <span>{page?.source_mode ?? "unknown"}</span>
        </div>
      </section>
      )}

      {mountStatus === "ready" ? null : (
      <section className="work-panel">
        <div className="panel-title">{t("web.extension_page.highlights", "能力摘要")}</div>
        <div className="card-grid">
          {[t("web.extension_page.dynamic_nav", "动态导航"), t("web.extension_page.dynamic_panel", "动态面板")].map((item) => (
            <article key={item} className="mini-card">
              <strong>{item}</strong>
              <span>{t("web.extension_page.contributed", "由扩展贡献并由 Web 壳层承载")}</span>
            </article>
          ))}
        </div>
      </section>
      )}

      {mountStatus === "ready" ? null : (
      <section className="work-panel">
        <div className="panel-title">{t("web.extension_page.panels", "关联面板")}</div>
        <div className="stack">
          {panels.length === 0 ? (
            <div className="empty-card">{t("web.extension_page.no_panels", "暂无关联扩展面板。")}</div>
          ) : (
            panels.map((panel) => (
              <article key={panel.panel.id} className="resource-card">
                <header>
                  <strong>{resolveText(panel.panel.title)}</strong>
                  <span>{panel.panel.slot}</span>
                </header>
                <p>{panel.panel.mount}</p>
              </article>
            ))
          )}
        </div>
      </section>
      )}
    </div>
  );
}
