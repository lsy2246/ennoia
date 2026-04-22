import { useRouterState } from "@tanstack/react-router";

import { builtinExtensionPages, builtinExtensionPanels } from "@ennoia/builtins";
import { useUiHelpers } from "@/stores/ui";
import { extensionPageComponents } from "@/views/extensions/registry";

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
  const page = runtime?.registry.pages.find(
    (item) =>
      item.page.id === pageId &&
      item.extension_id !== "observatory" &&
      item.extension_id !== "ext.observatory" &&
      item.page.route !== "/observatory" &&
      !item.page.mount.startsWith("observatory."),
  );
  const descriptor = page ? builtinExtensionPages[page.page.mount] : undefined;
  const PageComponent = page ? extensionPageComponents[page.page.mount] : undefined;
  const panels = runtime?.registry.panels.filter((item) => item.extension_id === page?.extension_id) ?? [];

  if (page && PageComponent) {
    return <PageComponent />;
  }

  return (
    <div className="extension-view">
      <section className="work-panel hero-empty">
        <span>{descriptor?.eyebrow ?? t("web.extension_page.eyebrow", "Extension View")}</span>
        <h1>{page ? resolveText(page.page.title) : t("web.extension_page.not_found", "扩展视图未找到")}</h1>
        <p>
          {descriptor?.summary ??
            t("web.extension_page.description", "这是由扩展注册表贡献并挂接到 Web 工作台的动态视图。")}
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
          {(descriptor?.highlights ?? [t("web.extension_page.dynamic_nav", "动态导航"), t("web.extension_page.dynamic_panel", "动态面板")]).map((item) => (
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
            panels.map((panel) => {
              const panelDescriptor = builtinExtensionPanels[panel.panel.mount];
              return (
                <article key={panel.panel.id} className="resource-card">
                  <header>
                    <strong>{resolveText(panel.panel.title)}</strong>
                    <span>{panel.panel.slot}</span>
                  </header>
                  <p>{panelDescriptor?.summary ?? panel.panel.mount}</p>
                </article>
              );
            })
          )}
        </div>
      </section>
    </div>
  );
}
