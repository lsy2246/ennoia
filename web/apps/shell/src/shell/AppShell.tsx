import { Link, Outlet, useNavigate } from "@tanstack/react-router";

import { useAuthStore } from "@/stores/auth";
import { useUiHelpers } from "@/stores/ui";

export function AppShell() {
  const user = useAuthStore((s) => s.user);
  const logout = useAuthStore((s) => s.logout);
  const { t, runtime, resolveText } = useUiHelpers();
  const navigate = useNavigate();

  async function handleLogout() {
    await logout();
    navigate({ to: "/login" });
  }

  const isAdmin = user?.role === "admin" || user?.role === "anonymous";

  return (
    <div className="app-shell">
      <nav className="app-nav">
        <div className="app-nav__brand">
          <Link to="/">
            {runtime ? resolveText(runtime.ui_config.shell_title) : "Ennoia"}
          </Link>
        </div>
        <div className="app-nav__links">
          <Link to="/" className="app-nav__link">
            {t("nav.dashboard", "Dashboard")}
          </Link>
          <Link to="/memories" className="app-nav__link">
            {t("nav.memories", "Memories")}
          </Link>
          <Link to="/settings" className="app-nav__link">
            {t("nav.settings", "Settings")}
          </Link>
          {isAdmin && (
            <>
              <Link to="/admin/users" className="app-nav__link">
                {t("nav.users", "Users")}
              </Link>
              <Link to="/admin/sessions" className="app-nav__link">
                {t("nav.sessions", "Sessions")}
              </Link>
              <Link to="/admin/api-keys" className="app-nav__link">
                {t("nav.api_keys", "API Keys")}
              </Link>
            </>
          )}
        </div>
        <div className="app-nav__user">
          <span className="app-nav__user-label">
            {user?.username ?? "anonymous"}
            {user?.role ? ` · ${user.role}` : ""}
          </span>
          <button onClick={handleLogout} className="app-nav__logout">
            {t("auth.logout", "Logout")}
          </button>
        </div>
      </nav>
      <main className="app-main">
        <Outlet />
      </main>
    </div>
  );
}
