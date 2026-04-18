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
  const privateConversation = await fetchJson(baseUrl, "/api/v1/threads/private/messages", {
    method: "POST",
    body: JSON.stringify({
      agent_id: "coder",
      body: "请整理 settings 页面需求",
      goal: "实现 settings 页面",
    }),
  });
  const threads = await fetchJson(baseUrl, "/api/v1/threads");
  const messages = await fetchJson(
    baseUrl,
    `/api/v1/threads/${privateConversation.thread.id}/messages`,
  );
  const threadRuns = await fetchJson(
    baseUrl,
    `/api/v1/threads/${privateConversation.thread.id}/runs`,
  );
  const runTasks = await fetchJson(baseUrl, `/api/v1/runs/${privateConversation.run.id}/tasks`);
  const runArtifacts = await fetchJson(
    baseUrl,
    `/api/v1/runs/${privateConversation.run.id}/artifacts`,
  );
  const memories = await fetchJson(baseUrl, "/api/v1/memories");

  assert(health.status === "ok", "health status should be ok");
  assert(Array.isArray(extensions) && extensions.length >= 1, "extensions should not be empty");
  assert(registry.extensions.length >= 1, "registry extensions should not be empty");
  assert(registry.pages.length >= 1, "registry pages should not be empty");
  assert(registry.panels.length >= 1, "registry panels should not be empty");
  assert(pages[0].page.mount === "observatory.events.page", "page mount contract should match");
  assert(panels[0].panel.slot === "right", "panel slot contract should match");
  assert(threads.length >= 1, "threads should not be empty");
  assert(messages.length === 1, "thread messages should contain the created message");
  assert(threadRuns.length === 1, "thread runs should contain the created run");
  assert(runTasks.length === 1, "private thread should create one response task");
  assert(runArtifacts.length === 1, "run should expose one persisted artifact");
  assert(memories.some((memory) => memory.thread_id === privateConversation.thread.id), "memory should bind thread id");

  console.log("[integration] runtime smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}
