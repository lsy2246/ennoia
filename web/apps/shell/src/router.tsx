import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";

import { AgentsPage } from "@/pages/AgentsPage";
import { ExtensionsPage } from "@/pages/ExtensionsPage";
import { ExtensionPageView } from "@/pages/ExtensionPageView";
import { LogsPage } from "@/pages/LogsPage";
import { MemoryPage } from "@/pages/MemoryPage";
import { ProvidersPage } from "@/pages/ProvidersPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { SkillsPage } from "@/pages/SkillsPage";
import { TasksPage } from "@/pages/TasksPage";
import { WelcomePage } from "@/pages/WelcomePage";
import { WorkspacePage } from "@/pages/WorkspacePage";
import { AppShell } from "@/shell/AppShell";
import { useRuntimeStore } from "@/stores/runtime";

function requireInitialized() {
  const { bootstrap } = useRuntimeStore.getState();
  if (bootstrap && !bootstrap.is_initialized) {
    throw redirect({ to: "/welcome" });
  }
}

function redirectToWorkspace() {
  const { bootstrap } = useRuntimeStore.getState();
  if (bootstrap?.is_initialized) {
    throw redirect({ to: "/workspace" });
  }
}

const rootRoute = createRootRoute({ component: () => <Outlet /> });

const welcomeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/welcome",
  component: WelcomePage,
  beforeLoad: redirectToWorkspace,
});

const shellRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "shell",
  component: AppShell,
  beforeLoad: requireInitialized,
});

const homeRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/workspace" });
  },
});

const workspaceRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/workspace",
  component: WorkspacePage,
});

const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents",
  component: AgentsPage,
});

const skillsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/skills",
  component: SkillsPage,
});

const providersRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/upstreams",
  component: ProvidersPage,
});

const extensionPageRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extension-pages/$pageId",
  component: ExtensionPageView,
});

const extensionsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extensions",
  component: ExtensionsPage,
});

const tasksRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/tasks",
  component: TasksPage,
});

const logsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/logs",
  component: LogsPage,
});

const memoryRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/memory",
  component: MemoryPage,
});

const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/settings",
  component: SettingsPage,
});

const routeTree = rootRoute.addChildren([
  welcomeRoute,
  shellRoute.addChildren([
    homeRoute,
    workspaceRoute,
    agentsRoute,
    skillsRoute,
    providersRoute,
    extensionPageRoute,
    extensionsRoute,
    memoryRoute,
    tasksRoute,
    logsRoute,
    settingsRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
