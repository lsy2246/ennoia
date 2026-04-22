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
  assertExists(join(runtimeDir, "config", "extensions.toml"), "extensions registry");
  assertExists(join(runtimeDir, "config", "skills.toml"), "skills registry");
  assertExists(
    join(runtimeDir, "extensions", "observatory", "extension.toml"),
    "observatory manifest",
  );

  serverHandle = startServer(runtimeDir);
  await waitForServer(baseUrl, serverHandle);

  const health = await fetchJson(baseUrl, "/health");
  const bootstrap = await fetchJson(baseUrl, "/api/v1/bootstrap/status");
  const setup = await fetchJson(baseUrl, "/api/v1/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify({
      display_name: "Operator",
      locale: "zh-CN",
      time_zone: "Asia/Shanghai",
      theme_id: "system",
    }),
  });
  const profile = await fetchJson(baseUrl, "/api/v1/runtime/profile");
  const preferences = await fetchJson(baseUrl, "/api/v1/runtime/preferences");
  const uiMessages = await fetchJson(
    baseUrl,
    "/api/v1/ui/messages?locale=zh-CN&namespaces=web,settings,ext.observatory",
  );
  await ensureAgent(baseUrl, "coder", "Coder");
  const createdConversation = await fetchJson(baseUrl, "/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify({
      topology: "direct",
      agent_ids: ["coder"],
    }),
  });
  const envelope = await fetchJson(
    baseUrl,
    `/api/v1/conversations/${createdConversation.conversation.id}/messages`,
    {
      method: "POST",
      body: JSON.stringify({
        lane_id: createdConversation.default_lane.id,
        body: "请整理 settings 页面需求",
        goal: "实现 settings 页面",
      }),
    },
  );
  const conversations = await fetchJson(baseUrl, "/api/v1/conversations");
  const messages = await fetchJson(
    baseUrl,
    `/api/v1/conversations/${createdConversation.conversation.id}/messages`,
  );
  const memoryExtension = await fetchJson(baseUrl, "/api/v1/extensions/memory");
  const workflowExtension = await fetchJson(baseUrl, "/api/v1/extensions/workflow");

  assert(health.status === "ok", "health status should be ok");
  assert(bootstrap.is_initialized === false, "bootstrap should start as uninitialized");
  assert(setup.bootstrap.is_initialized === true, "bootstrap setup should initialize workspace");
  assert(profile.display_name === "Operator", "runtime profile should be persisted");
  assert(preferences.preference.theme_id === "system", "instance preference should be persisted");
  assert(uiMessages.bundles.length === 3, "ui messages should return requested namespaces");
  assert(
    uiMessages.bundles.some((bundle) => bundle.namespace === "settings"),
    "ui messages should include settings namespace",
  );
  assert(conversations.length >= 1, "conversations should not be empty");
  assert(messages.length === 1, "conversation should contain the created message");
  assert(envelope.message.id, "journal should return the persisted message");
  assert(memoryExtension.id === "memory", "memory extension should be registered");
  assert(workflowExtension.id === "workflow", "workflow extension should be registered");
  assert(memoryExtension.backend?.base_url, "memory extension should expose backend proxy info");

  console.log("[integration] runtime smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}

async function ensureAgent(baseUrl, id, displayName) {
  return fetchJson(baseUrl, "/api/v1/agents", {
    method: "POST",
    body: JSON.stringify({
      id,
      display_name: displayName,
      description: "",
      system_prompt: "",
      provider_id: "",
      model_id: "",
      generation_options: {},
      skills: [],
      enabled: true,
    }),
  });
}
