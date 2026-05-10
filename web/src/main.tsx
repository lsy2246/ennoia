import React from "react";
import ReactDOM from "react-dom/client";
import "dockview/dist/styles/dockview.css";

import { bootstrapTheme } from "@ennoia/theme-runtime";
import { getApiBaseUrl } from "@ennoia/api-client";
import { AppShell } from "@/AppShell";
import "./styles.css";

type BrowserProcessShim = {
  env: Record<string, string>;
  argv: string[];
  browser: true;
  platform: "browser";
  cwd: () => string;
  emit: () => false;
};

function installProcessShim() {
  const globalScope = globalThis as typeof globalThis & { process?: Partial<BrowserProcessShim> };
  if (globalScope.process && typeof globalScope.process === "object") {
    globalScope.process.env ??= {};
    globalScope.process.argv ??= [];
    globalScope.process.browser ??= true;
    globalScope.process.platform ??= "browser";
    globalScope.process.cwd ??= () => "/";
    globalScope.process.emit ??= () => false;
    return;
  }

  globalScope.process = {
    env: {},
    argv: [],
    browser: true,
    platform: "browser",
    cwd: () => "/",
    emit: () => false,
  };
}

bootstrapTheme();
installProcessShim();
(globalThis as { __ENNOIA_API_BASE_URL__?: string }).__ENNOIA_API_BASE_URL__ = getApiBaseUrl();

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <AppShell />
  </React.StrictMode>,
);
