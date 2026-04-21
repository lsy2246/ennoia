import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";

import { App } from "@/App";
import { Agents } from "@/pages/agents";
import { Extensions } from "@/pages/extensions";
import { Logs } from "@/pages/logs";
import { Memory } from "@/pages/memory";
import { Providers } from "@/pages/providers";
import { Settings } from "@/pages/settings";
import { Skills } from "@/pages/skills";
import { Tasks } from "@/pages/tasks";
import { Welcome } from "@/pages/welcome";
import { Conversations } from "@/pages/conversations";
import { ExtensionPageView } from "@/views/extensions/Page";
import { useRuntimeStore } from "@/stores/runtime";

function requireInitialized() {
  const { bootstrap } = useRuntimeStore.getState();
  if (bootstrap && !bootstrap.is_initialized) {
    throw redirect({ to: "/welcome" });
  }
}

function redirectToConversations() {
  const { bootstrap } = useRuntimeStore.getState();
  if (bootstrap?.is_initialized) {
    throw redirect({ to: "/conversations" });
  }
}

const rootRoute = createRootRoute({ component: () => <Outlet /> });

const welcomeRoute = createRoute({
  getParentRoute: () => rootRoute,
  path: "/welcome",
  component: Welcome,
  beforeLoad: redirectToConversations,
});

const shellRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "shell",
  component: App,
  beforeLoad: requireInitialized,
});

const homeRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/conversations" });
  },
});

const conversationsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/conversations",
  component: Conversations,
});

const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents",
  component: Agents,
});

const skillsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/skills",
  component: Skills,
});

const providersRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/upstreams",
  component: Providers,
});

const extensionPageRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extension-pages/$pageId",
  component: ExtensionPageView,
});

const extensionsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extensions",
  component: Extensions,
});

const tasksRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/tasks",
  component: Tasks,
});

const logsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/logs",
  component: Logs,
});

const memoryRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/memory",
  component: Memory,
});

const settingsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/settings",
  component: Settings,
});

const routeTree = rootRoute.addChildren([
  welcomeRoute,
  shellRoute.addChildren([
    homeRoute,
    conversationsRoute,
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

