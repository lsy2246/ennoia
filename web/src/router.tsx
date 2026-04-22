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

const webRoute = createRoute({
  getParentRoute: () => rootRoute,
  id: "web",
  component: App,
  beforeLoad: requireInitialized,
});

const homeRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/",
  beforeLoad: () => {
    throw redirect({ to: "/conversations" });
  },
});

const conversationsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/conversations",
  component: Conversations,
});

const agentsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/agents",
  component: Agents,
});

const skillsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/skills",
  component: Skills,
});

const providersRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/upstreams",
  component: Providers,
});

const extensionPageRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/extension-pages/$pageId",
  component: ExtensionPageView,
});

const extensionsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/extensions",
  component: Extensions,
});

const tasksRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/tasks",
  component: Tasks,
});

const logsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/logs",
  component: Logs,
});

const memoryRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/memory",
  component: Memory,
});

const settingsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/settings",
  component: Settings,
});

const routeTree = rootRoute.addChildren([
  welcomeRoute,
  webRoute.addChildren([
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

