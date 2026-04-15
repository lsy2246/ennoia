import { join } from "node:path";

import {
  assert,
  assertExists,
  buildBaseUrl,
  cleanupRuntimeFixture,
  configureRuntimePort,
  createRuntimeFixture,
  fetchJson,
  initRuntime,
  nextPort,
  startServer,
  stopServer,
  waitForServer,
} from "../helpers/runtime-harness.mjs";

const runtimeDir = createRuntimeFixture("integration");
const port = nextPort(11);
const baseUrl = buildBaseUrl(port);

let serverHandle;

try {
  initRuntime(runtimeDir);
  configureRuntimePort(runtimeDir, port);

  assertExists(join(runtimeDir, "config", "ennoia.toml"), "app config");
  assertExists(join(runtimeDir, "config", "server.toml"), "server config");
  assertExists(
    join(runtimeDir, "global", "extensions", "observatory", "manifest.toml"),
    "observatory manifest",
  );

  serverHandle = startServer(runtimeDir);
  await waitForServer(baseUrl, serverHandle);

  const health = await fetchJson(baseUrl, "/health");
  const extensions = await fetchJson(baseUrl, "/api/v1/extensions");
  const registry = await fetchJson(baseUrl, "/api/v1/extensions/registry");
  const pages = await fetchJson(baseUrl, "/api/v1/extensions/pages");
  const panels = await fetchJson(baseUrl, "/api/v1/extensions/panels");

  assert(health.status === "ok", "health status should be ok");
  assert(Array.isArray(extensions) && extensions.length >= 1, "extensions should not be empty");
  assert(registry.extensions.length >= 1, "registry extensions should not be empty");
  assert(registry.pages.length >= 1, "registry pages should not be empty");
  assert(registry.panels.length >= 1, "registry panels should not be empty");
  assert(pages[0].page.mount === "observatory.events.page", "page mount contract should match");
  assert(panels[0].panel.slot === "right", "panel slot contract should match");

  console.log("[integration] runtime smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}
