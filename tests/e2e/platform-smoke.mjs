import {
  assert,
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

const runtimeDir = createRuntimeFixture("e2e");
const port = nextPort(29);
const baseUrl = buildBaseUrl(port);

let serverHandle;

try {
  initRuntime(runtimeDir);
  configureRuntimePort(runtimeDir, port);

  serverHandle = startServer(runtimeDir);
  await waitForServer(baseUrl, serverHandle);

  await fetchJson(baseUrl, "/api/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify({
      display_name: "Operator",
      locale: "zh-CN",
      time_zone: "Asia/Shanghai",
      theme_id: "system",
    }),
  });
  await ensureAgent(baseUrl, "coder", "Coder");
  await ensureAgent(baseUrl, "planner", "Planner");

  const directConversation = await fetchJson(baseUrl, "/api/conversations", {
    method: "POST",
    body: JSON.stringify({
      topology: "direct",
      agent_ids: ["coder"],
    }),
  });
  const groupConversation = await fetchJson(baseUrl, "/api/conversations", {
    method: "POST",
    body: JSON.stringify({
      topology: "group",
      space_id: "studio",
      agent_ids: ["coder", "planner"],
    }),
  });

  const directEnvelope = await fetchJson(
    baseUrl,
    `/api/conversations/${directConversation.conversation.id}/messages`,
    {
      method: "POST",
      body: JSON.stringify({
        lane_id: directConversation.default_lane.id,
        goal: "实现 settings 页面",
        body: "请整理 settings 页面需求",
      }),
    },
  );
  const groupEnvelope = await fetchJson(
    baseUrl,
    `/api/conversations/${groupConversation.conversation.id}/messages`,
    {
      method: "POST",
      body: JSON.stringify({
        lane_id: groupConversation.default_lane.id,
        goal: "整理 roadmap",
        body: "请一起整理 roadmap",
      }),
    },
  );

  const overview = await fetchJson(baseUrl, "/api/overview");
  const uiMessages = await fetchJson(
    baseUrl,
    "/api/ui/messages?locale=zh-CN&namespaces=web,ext.observatory",
  );
  const conversations = await fetchJson(baseUrl, "/api/conversations");
  const directMessages = await fetchJson(
    baseUrl,
    `/api/conversations/${directConversation.conversation.id}/messages`,
  );
  const groupMessages = await fetchJson(
    baseUrl,
    `/api/conversations/${groupConversation.conversation.id}/messages`,
  );
  const memoryExtension = await fetchJson(baseUrl, "/api/extensions/memory");
  const workflowExtension = await fetchJson(baseUrl, "/api/extensions/workflow");

  assert(overview.counts.extensions >= 1, "overview should expose extensions count");
  assert(uiMessages.bundles.length === 2, "ui messages should include requested builtin bundles");
  assert(overview.counts.extensions >= 3, "overview should expose builtin extensions count");
  assert(directEnvelope.message.id, "direct conversation should return a persisted message");
  assert(groupEnvelope.message.id, "group conversation should return a persisted message");
  assert(conversations.length >= 2, "conversations should include direct and group sessions");
  assert(directMessages.length === 1, "direct conversation should contain one message");
  assert(groupMessages.length === 1, "group conversation should contain one message");
  assert(memoryExtension.id === "memory", "memory extension should be registered");
  assert(workflowExtension.id === "workflow", "workflow extension should be registered");
  assert(memoryExtension.worker?.entry, "memory extension should expose worker entry info");

  console.log("[e2e] platform smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}

async function ensureAgent(baseUrl, id, displayName) {
  return fetchJson(baseUrl, "/api/agents", {
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
