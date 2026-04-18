import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";

import { useAuthStore } from "./stores/auth";

import { AppShell } from "./shell/AppShell";
import { DashboardPage } from "./pages/DashboardPage";
import { LoginPage } from "./pages/LoginPage";
import { OnboardingPage } from "./pages/OnboardingPage";
import { RunDetailPage } from "./pages/RunDetailPage";
import { MemoriesPage } from "./pages/MemoriesPage";
import { SettingsPage } from "./pages/SettingsPage";
import { AdminUsersPage } from "./pages/AdminUsersPage";
import { AdminSessionsPage } from "./pages/AdminSessionsPage";
import { AdminApiKeysPage } from "./pages/AdminApiKeysPage";

function requireAuth() {
  const { user, bootstrap } = useAuthStore.getState();
  if (bootstrap && !bootstrap.completed) {
    throw redirect({ to: "/onboarding" });
  }
  if (!user) {
    throw redirect({ to: "/login" });
  }
}

function requireAdmin() {
  requireAuth();
  const { user } = useAuthStore.getState();
  if (user && user.role !== "admin" && user.role !== "anonymous") {
    throw redirect({ to: "/" });
  }
}

const rootRoute = createRootRoute({ component: () => <Outlet /> });

const loginRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/login",
  component: LoginPage,
});

const onboardingRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/onboarding",
  component: OnboardingPage,
});

const shellRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "shell",
  component: AppShell,
  beforeLoad: requireAuth,
});

const dashboardRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  component: DashboardPage,
});

const runDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/runs/$runId",
  component: RunDetailPage,
});

const memoriesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/memories",
  component: MemoriesPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/settings",
  component: SettingsPage,
  beforeLoad: requireAdmin,
});

const adminUsersRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/admin/users",
  component: AdminUsersPage,
  beforeLoad: requireAdmin,
});

const adminSessionsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/admin/sessions",
  component: AdminSessionsPage,
  beforeLoad: requireAdmin,
});

const adminApiKeysRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/admin/api-keys",
  component: AdminApiKeysPage,
  beforeLoad: requireAdmin,
});

const routeTree = rootRoute.addChildren([
  loginRoute,
  onboardingRoute,
  shellRoute.addChildren([
    dashboardRoute,
    runDetailRoute,
    memoriesRoute,
    settingsRoute,
    adminUsersRoute,
    adminSessionsRoute,
    adminApiKeysRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
