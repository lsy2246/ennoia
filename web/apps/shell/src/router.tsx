import {
  createRootRoute,
  createRoute,
  createRouter,
  Outlet,
  redirect,
} from "@tanstack/react-router";

import { AgentsPage } from "@/pages/AgentsPage";
import { ArtifactsPage } from "@/pages/ArtifactsPage";
import { ConversationDetailPage } from "@/pages/ConversationDetailPage";
import { ConversationsPage } from "@/pages/ConversationsPage";
import { RunDetailPage } from "@/pages/RunDetailPage";
import { SettingsPage } from "@/pages/SettingsPage";
import { SpacesPage } from "@/pages/SpacesPage";
import { WelcomePage } from "@/pages/WelcomePage";
import { WorkflowsPage } from "@/pages/WorkflowsPage";
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
    throw redirect({ to: "/conversations" });
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
  component: ConversationsPage,
});

const conversationsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/conversations",
  component: ConversationsPage,
});

const conversationDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/conversations/$conversationId",
  component: ConversationDetailPage,
});

const spacesRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/spaces",
  component: SpacesPage,
});

const agentsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/agents",
  component: AgentsPage,
});

const artifactsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/artifacts",
  component: ArtifactsPage,
});

const workflowsRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/workflows",
  component: WorkflowsPage,
});

const workflowDetailRoute = createRoute({
  getParentRoute: () => shellRoute,
  path: "/workflows/$runId",
  component: RunDetailPage,
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
    conversationsRoute,
    conversationDetailRoute,
    spacesRoute,
    agentsRoute,
    artifactsRoute,
    workflowsRoute,
    workflowDetailRoute,
    settingsRoute,
  ]),
]);

export const router = createRouter({ routeTree });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}
