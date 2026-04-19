import { Link, Outlet } from "@tanstack/react-router";

import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers } from "@/stores/ui";

export function AppShell() {
  const profile = useRuntimeStore((state) => state.profile);
  const { resolveText, runtime } = useUiHelpers();

  return (
    <div className="app-shell">
      <nav className="app-nav">
        <div className="app-nav__brand">
          <Link to="/conversations">
            {runtime ? resolveText(runtime.ui_config.shell_title) : "Ennoia"}
          </Link>
        </div>
        <div className="app-nav__links">
          <Link to="/conversations" className="app-nav__link">
            Conversations
          </Link>
          <Link to="/spaces" className="app-nav__link">
            Spaces
          </Link>
          <Link to="/agents" className="app-nav__link">
            Agents
          </Link>
          <Link to="/artifacts" className="app-nav__link">
            Artifacts
          </Link>
          <Link to="/workflows" className="app-nav__link">
            Workflows
          </Link>
          <Link to="/settings" className="app-nav__link">
            Settings
          </Link>
        </div>
        <div className="app-nav__user">
          <span className="app-nav__user-label">{profile?.display_name ?? "Operator"}</span>
        </div>
      </nav>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
