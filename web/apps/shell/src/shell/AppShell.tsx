import { Link, Outlet } from "@tanstack/react-router";

import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";

const NAV_ITEMS = [
  { to: "/chat", key: "shell.nav.chat", fallback: "聊天" },
  { to: "/schedules", key: "shell.nav.schedules", fallback: "计划任务" },
  { to: "/agents", key: "shell.nav.agents", fallback: "Agent" },
  { to: "/extensions", key: "shell.nav.extensions", fallback: "扩展" },
  { to: "/logs", key: "shell.nav.logs", fallback: "日志" },
  { to: "/settings", key: "shell.nav.settings", fallback: "设置" },
];

export function AppShell() {
  const profile = useRuntimeStore((state) => state.profile);
  const locale = useUiStore((state) => state.locale);
  const themeId = useUiStore((state) => state.themeId);
  const { resolveText, runtime, t } = useUiHelpers();

  return (
    <div className="workbench-shell">
      <aside className="workbench-sidebar">
        <div className="workbench-brand">
          <Link to="/chat">
            {runtime ? resolveText(runtime.ui_config.shell_title) : t("shell.title", "Ennoia")}
          </Link>
          <p>{t("shell.brand.subtitle", "本地多 Agent 工作台")}</p>
        </div>

        <nav className="workbench-nav">
          {NAV_ITEMS.map((item) => (
            <Link
              key={item.to}
              to={item.to}
              className="workbench-nav__item"
              activeProps={{ className: "workbench-nav__item workbench-nav__item--active" }}
            >
              {t(item.key, item.fallback)}
            </Link>
          ))}
        </nav>

        <div className="workbench-profile">
          <strong>{profile?.display_name ?? t("settings.profile.default_name", "Operator")}</strong>
          <span>
            {locale} · {themeId}
          </span>
        </div>
      </aside>

      <main className="workbench-main">
        <Outlet />
      </main>
    </div>
  );
}
