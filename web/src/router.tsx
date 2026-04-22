import { createRootRoute, createRoute, createRouter, Outlet, redirect } from "@tanstack/react-router";

import { App } from "@/App";
import { Agents } from "@/pages/agents";
import { Extensions } from "@/pages/extensions";
import { Logs } from "@/pages/logs";
import { Providers } from "@/pages/providers";
import { Settings } from "@/pages/settings";
import { Skills } from "@/pages/skills";
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

function Dashboard() {
  return (
    <div style={{ height: "100%", display: "flex", flexDirection: "column", justifyContent: "center", alignItems: "center" }}>
      <div style={{
        width: "120px",
        height: "120px",
        borderRadius: "28px",
        background: "linear-gradient(135deg, var(--accent), color-mix(in srgb, var(--accent) 70%, transparent))",
        boxShadow: "0 12px 32px rgba(0, 122, 255, 0.2)",
        display: "flex",
        justifyContent: "center",
        alignItems: "center",
        marginBottom: "32px",
        color: "white",
        fontSize: "64px",
        fontWeight: "200"
      }}>
        E
      </div>
      <h1 style={{ fontSize: "36px", marginBottom: "12px", fontWeight: "600", letterSpacing: "-0.02em", color: "var(--text)" }}>Welcome to Ennoia</h1>
      <p style={{ fontSize: "16px", color: "var(--text-muted)", maxWidth: "360px", textAlign: "center", lineHeight: "1.5" }}>
        Select an item from the sidebar to start a conversation, manage agents, or configure extensions.
      </p>
    </div>
  );
}

const homeRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/",
  component: Dashboard,
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

const logsRoute = createRoute({
  getParentRoute: () => webRoute,
  path: "/logs",
  component: Logs,
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

