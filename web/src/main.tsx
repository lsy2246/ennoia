import React from "react";
import ReactDOM from "react-dom/client";
import "dockview/dist/styles/dockview.css";
import { RouterProvider } from "@tanstack/react-router";
import { useEffect } from "react";

import { bootstrapTheme } from "@ennoia/theme-runtime";
import { reportFrontendLog } from "@ennoia/api-client";
import { router } from "@/router";
import { useRuntimeStore } from "@/stores/runtime";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import "./styles.css";

bootstrapTheme();

function reportRuntimeError(title: string, error: unknown) {
  void reportFrontendLog({
    level: "error",
    source: "frontend",
    title,
    summary: error instanceof Error ? error.message : String(error),
    details: error instanceof Error ? error.stack : undefined,
    at: new Date().toISOString(),
  }).catch(() => undefined);
}

window.addEventListener("error", (event) => {
  reportRuntimeError("window.error", event.error ?? event.message);
});

window.addEventListener("unhandledrejection", (event) => {
  reportRuntimeError("unhandledrejection", event.reason);
});

function App() {
  const runtimeHydrate = useRuntimeStore((state) => state.hydrate);
  const runtimeStatus = useRuntimeStore((state) => state.status);
  const uiHydrate = useUiStore((state) => state.hydrate);
  const uiStatus = useUiStore((state) => state.status);
  const { t } = useUiHelpers();

  useEffect(() => {
    runtimeHydrate();
    uiHydrate();
  }, [runtimeHydrate, uiHydrate]);

  if (
    runtimeStatus === "idle" ||
    runtimeStatus === "checking" ||
    uiStatus === "idle" ||
    uiStatus === "checking"
  ) {
    return (
      <div className="page page--centered">
        <p>{t("web.loading.connecting", "Connecting to Ennoia…")}</p>
      </div>
    );
  }

  return <RouterProvider router={router} />;
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
