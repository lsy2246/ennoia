import { Link, Outlet } from "@tanstack/react-router";

import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

export function AppShell() {
  const profile = useRuntimeStore((state) => state.profile);
  const locale = useUiStore((state) => state.locale);
  const themeId = useUiStore((state) => state.themeId);
  const { resolveText, runtime, t } = useUiHelpers();
  const dynamicPages = runtime?.registry.pages ?? [];

  return (
    <div className="app-shell">
      <nav className="app-nav">
        <div className="app-nav__brand">
          <Link to="/conversations">
            {runtime ? resolveText(runtime.ui_config.shell_title) : "Ennoia"}
          </Link>
        </div>
        <div className="app-nav__links">
          <Link
            to="/conversations"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.conversations", "Conversations")}
          </Link>
          <Link
            to="/spaces"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.spaces", "Spaces")}
          </Link>
          <Link
            to="/jobs"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.jobs", "Jobs")}
          </Link>
          <Link
            to="/workflows"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.workflows", "Workflows")}
          </Link>
          <Link
            to="/memories"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.memories", "Memories")}
          </Link>
          <Link
            to="/extensions"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.extensions", "Extensions")}
          </Link>
          {dynamicPages.slice(0, 4).map((page) => (
            <Link
              key={`${page.extension_id}:${page.page.id}`}
              to="/ext/$extensionId/$pageId"
              params={{ extensionId: page.extension_id, pageId: page.page.id }}
              className="app-nav__link"
              activeProps={{ className: "app-nav__link app-nav__link--active" }}
            >
              {resolveText(page.page.title)}
            </Link>
          ))}
          <Link
            to="/agents"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.agents", "Agents")}
          </Link>
          <Link
            to="/artifacts"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.artifacts", "Artifacts")}
          </Link>
          <Link
            to="/logs"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.logs", "Logs")}
          </Link>
          <Link
            to="/settings"
            className="app-nav__link"
            activeProps={{ className: "app-nav__link app-nav__link--active" }}
          >
            {t("shell.nav.settings", "Settings")}
          </Link>
        </div>
        <div className="app-nav__user">
          <div className="app-nav__user-meta">
            <span className="app-nav__user-label">{profile?.display_name ?? "Operator"}</span>
            <span className="app-nav__meta">
              {locale} · {themeId}
            </span>
          </div>
        </div>
      </nav>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
