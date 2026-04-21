import React from "react";
import ReactDOM from "react-dom/client";
import "dockview/dist/styles/dockview.css";

import { bootstrapTheme } from "@ennoia/theme-runtime";
import { reportFrontendLog } from "@ennoia/api-client";
import { App } from "@/App";
import { applyWorkbenchPalette, readWorkbenchPalette } from "@/lib/palette";
import "./styles.css";

bootstrapTheme();
applyWorkbenchPalette(readWorkbenchPalette());

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

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
