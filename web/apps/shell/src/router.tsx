import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";

import { AgentDetailPage } from "@/pages/AgentDetailPage";
import { AgentsPage } from "@/pages/AgentsPage";
import { ChatDetailPage } from "@/pages/ChatDetailPage";
import { ChatPage } from "@/pages/ChatPage";
import { DelegationDetailPage } from "@/pages/DelegationDetailPage";
import { ExtensionDetailPage } from "@/pages/ExtensionDetailPage";
import { ExtensionsPage } from "@/pages/ExtensionsPage";
import { LogsPage } from "@/pages/LogsPage";
import { ScheduleDetailPage } from "@/pages/ScheduleDetailPage";
import { SchedulesPage } from "@/pages/SchedulesPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { WelcomePage } from "@/pages/WelcomePage";
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
    throw redirect({ to: "/chat" });
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
    throw redirect({ to: "/chat" });
  },
});

const chatRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/chat",
  component: ChatPage,
});

const chatDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/chat/$chatId",
  component: ChatDetailPage,
});

const delegationDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/chat/$chatId/delegations/$delegationId",
  component: DelegationDetailPage,
});

const schedulesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/schedules",
  component: SchedulesPage,
});

const scheduleDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/schedules/$scheduleId",
  component: ScheduleDetailPage,
});

const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents",
  component: AgentsPage,
});

const agentDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents/$agentId",
  component: AgentDetailPage,
});

const extensionsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extensions",
  component: ExtensionsPage,
});

const extensionDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/extensions/$extensionId",
  component: ExtensionDetailPage,
});

const logsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/logs",
  component: LogsPage,
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
    chatRoute,
    chatDetailRoute,
    delegationDetailRoute,
    schedulesRoute,
    scheduleDetailRoute,
    agentsRoute,
    agentDetailRoute,
    extensionsRoute,
    extensionDetailRoute,
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
