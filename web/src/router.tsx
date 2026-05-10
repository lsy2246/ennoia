import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";

import { App } from "@/App";
import { Agents } from "@/pages/agents";
import { Extensions } from "@/pages/extensions";
import { LogsPage } from "@/pages/logs";
import { Schedules } from "@/pages/schedules";
import { Settings } from "@/pages/settings";
import { Skills } from "@/pages/skills";
import { Home } from "@/pages/home";
import { Welcome } from "@/pages/welcome";
import { Conversations } from "@/pages/conversations";
import { ExtensionPageView } from "@/views/extensions/Page";
import { ExtensionPanelView } from "@/views/extensions/Panel";
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
  component: Home,
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

const schedulesRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/schedules",
  component: Schedules,
});

const extensionPageRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/extension-pages/$pageId",
  component: ExtensionPageView,
});

const extensionPanelRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/extension-panels/$panelId",
  component: ExtensionPanelView,
});

const extensionsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/extensions",
  component: Extensions,
});

const logsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/logs",
  component: LogsPage,
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
    schedulesRoute,
    extensionPageRoute,
    extensionPanelRoute,
    extensionsRoute,
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

