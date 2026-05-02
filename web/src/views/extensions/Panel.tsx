import { useEffect, useRef, useState } from "react";
import { useRouterState } from "@tanstack/react-router";
import { apiUrl } from "@ennoia/api-client";

import { useUiHelpers, useUiStore } from "@/stores/ui";
import { loadExtensionPanelMount } from "@/views/extensions/registry";

function panelIdFromPath(pathname: string) {
  const match = pathname.match(/^\/extension-panels\/([^/]+)$/);
  return match ? decodeURIComponent(match[1]) : "";
}

type ExtensionPanelViewProps = {
  panelId?: string;
};

export function ExtensionPanelView({ panelId: explicitPanelId }: ExtensionPanelViewProps = {}) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const helpers = useUiHelpers();
  const themeId = useUiStore((state) => state.themeId);
  const { formatDate, formatDateTime, formatTime, locale, runtime, resolveText, t } = helpers;
  const panelId = explicitPanelId ?? panelIdFromPath(pathname);
  const panel = runtime?.registry.panels.find((item) => item.panel.id === panelId);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const [mountStatus, setMountStatus] = useState<"idle" | "loading" | "ready" | "error">("idle");
  const [mountError, setMountError] = useState<string | null>(null);
  const relatedPages = runtime?.registry.pages.filter((item) => item.extension_id === panel?.extension_id) ?? [];
  const generation = runtime?.versions.registry ?? 0;

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void | Promise<void>) | undefined;
    const container = containerRef.current;
    setMountStatus("idle");
    setMountError(null);
    if (!panel || !container) {
      return () => {
        cancelled = true;
      };
    }

    container.replaceChildren();
    setMountStatus("loading");
    void loadExtensionPanelMount(panel, generation)
      .then(async (mount) => {
        if (cancelled) {
          return;
        }
        if (!mount) {
          setMountStatus("idle");
          return;
        }
        const handle = await mount(container, {
          kind: "panel",
          extensionId: panel.extension_id,
          mount: panel.panel.mount,
          panel,
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
  }, [formatDate, formatDateTime, formatTime, generation, locale, panel, t, themeId]);

  return (
    <div className="extension-view">
      <section className="work-panel extensions-host-bar">
        <div className="extensions-section__header">
          <div className="page-heading">
            <span>{t("web.extension_page.eyebrow", "Extension View")}</span>
            <h1>{panel ? resolveText(panel.panel.title) : t("web.extension_page.not_found", "扩展视图未找到")}</h1>
            <p>
              {panel
                ? t("web.extension_panel.host_description", "这个区域由扩展自身渲染，宿主会保留面板槽位和扩展来源信息。")
                : t("web.extension_page.description", "这是由扩展注册表贡献并挂接到 Web 工作台的动态视图。")}
            </p>
          </div>
          <div className="extensions-inline-meta">
            <span className="badge badge--muted">{panel?.extension_id ?? "unknown"}</span>
            <span className="badge badge--muted">{panel?.panel.mount ?? "no mount"}</span>
            <span className="badge badge--muted">{panel?.panel.slot ?? "no slot"}</span>
            <span className="badge badge--muted">{`${t("web.extension_panel.pages", "关联页面")} ${relatedPages.length}`}</span>
          </div>
        </div>
      </section>

      {panel ? <div ref={containerRef} data-extension-panel={panel.panel.mount} /> : null}
      {mountStatus === "ready" ? null : (
        <section className="work-panel hero-empty">
          <span>{t("web.extension_page.eyebrow", "Extension View")}</span>
          <h1>{panel ? resolveText(panel.panel.title) : t("web.extension_page.not_found", "扩展视图未找到")}</h1>
          <p>
            {mountStatus === "loading"
              ? t("web.extension_page.loading", "正在加载扩展 UI 模块。")
              : mountStatus === "error"
                ? mountError
                : t("web.extension_page.description", "这是由扩展注册表贡献并挂接到 Web 工作台的动态视图。")}
          </p>
          <div className="tag-row">
            <span>{panel?.extension_id ?? "unknown"}</span>
            <span>{panel?.panel.mount ?? "no mount"}</span>
            <span>{panel?.panel.slot ?? "no slot"}</span>
          </div>
        </section>
      )}
    </div>
  );
}
