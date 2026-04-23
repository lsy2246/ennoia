import { useEffect, useState, type ComponentType } from "react";
import { useRouterState } from "@tanstack/react-router";

import { useUiHelpers } from "@/stores/ui";
import { loadExtensionPageComponent } from "@/views/extensions/registry";

function pageIdFromPath(pathname: string) {
  const match = pathname.match(/^\/extension-pages\/([^/]+)$/);
  return match ? decodeURIComponent(match[1]) : "";
}

type ExtensionPageViewProps = {
  pageId?: string;
};

export function ExtensionPageView({ pageId: explicitPageId }: ExtensionPageViewProps = {}) {
  const pathname = useRouterState({ select: (state) => state.location.pathname });
  const { runtime, resolveText, t } = useUiHelpers();
  const pageId = explicitPageId ?? pageIdFromPath(pathname);
  const page = runtime?.registry.pages.find((item) => item.page.id === pageId);
  const [PageComponent, setPageComponent] = useState<ComponentType | null>(null);
  const panels = runtime?.registry.panels.filter((item) => item.extension_id === page?.extension_id) ?? [];

  useEffect(() => {
    let cancelled = false;
    setPageComponent(null);
    if (!page) {
      return () => {
        cancelled = true;
      };
    }

    void loadExtensionPageComponent(page)
      .then((component) => {
        if (!cancelled) {
          setPageComponent(() => component);
        }
      })
      .catch(() => {
        if (!cancelled) {
          setPageComponent(null);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [page]);

  if (page && PageComponent) {
    return <PageComponent />;
  }

  return (
    <div className="extension-view">
      <section className="work-panel hero-empty">
        <span>{t("web.extension_page.eyebrow", "Extension View")}</span>
        <h1>{page ? resolveText(page.page.title) : t("web.extension_page.not_found", "扩展视图未找到")}</h1>
        <p>
          {t("web.extension_page.description", "这是由扩展注册表贡献并挂接到 Web 工作台的动态视图。")}
        </p>
        <div className="tag-row">
          <span>{page?.extension_id ?? "unknown"}</span>
          <span>{page?.page.mount ?? "no mount"}</span>
          <span>{page?.source_mode ?? "unknown"}</span>
        </div>
      </section>

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
    </div>
  );
}
