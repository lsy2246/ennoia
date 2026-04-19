import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

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

  await fetchJson(baseUrl, "/api/v1/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify({
      display_name: "Operator",
      locale: "zh-CN",
      time_zone: "Asia/Shanghai",
      theme_id: "system",
    }),
  });

  const directConversation = await fetchJson(baseUrl, "/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify({
      topology: "direct",
      agent_ids: ["coder"],
    }),
  });
  const groupConversation = await fetchJson(baseUrl, "/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify({
      topology: "group",
      space_id: "studio",
      agent_ids: ["coder", "planner"],
    }),
  });

  const directEnvelope = await fetchJson(
    baseUrl,
    `/api/v1/conversations/${directConversation.conversation.id}/messages`,
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
    `/api/v1/conversations/${groupConversation.conversation.id}/messages`,
    {
      method: "POST",
      body: JSON.stringify({
        lane_id: groupConversation.default_lane.id,
        goal: "整理 roadmap",
        body: "请一起整理 roadmap",
      }),
    },
  );

  const job = await fetchJson(baseUrl, "/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify({
      owner_kind: "space",
      owner_id: "studio",
      job_kind: "maintenance",
      schedule_kind: "cron",
      schedule_value: "0 */6 * * *",
    }),
  });

  const overview = await fetchJson(baseUrl, "/api/v1/overview");
  const uiMessages = await fetchJson(
    baseUrl,
    "/api/v1/ui/messages?locale=zh-CN&namespaces=shell,ext.observatory",
  );
  const conversations = await fetchJson(baseUrl, "/api/v1/conversations");
  const directMessages = await fetchJson(
    baseUrl,
    `/api/v1/conversations/${directConversation.conversation.id}/messages`,
  );
  const groupMessages = await fetchJson(
    baseUrl,
    `/api/v1/conversations/${groupConversation.conversation.id}/messages`,
  );
  const runs = await fetchJson(baseUrl, "/api/v1/runs");
  const tasks = await fetchJson(baseUrl, "/api/v1/tasks");
  const directRunTasks = await fetchJson(
    baseUrl,
    `/api/v1/runs/${directEnvelope.run.id}/tasks`,
  );
  const groupRunTasks = await fetchJson(
    baseUrl,
    `/api/v1/runs/${groupEnvelope.run.id}/tasks`,
  );
  const directArtifacts = await fetchJson(
    baseUrl,
    `/api/v1/runs/${directEnvelope.run.id}/artifacts`,
  );
  const artifacts = await fetchJson(baseUrl, "/api/v1/artifacts");
  const jobs = await fetchJson(baseUrl, "/api/v1/jobs");
  const memories = await fetchJson(baseUrl, "/api/v1/memories");

  assert(overview.counts.extensions >= 1, "overview should expose extensions count");
  assert(uiMessages.bundles.length === 2, "ui messages should include requested builtin bundles");
  assert(overview.counts.conversations >= 2, "overview should expose conversation count");
  assert(overview.counts.messages >= 2, "overview should expose message count");
  assert(directEnvelope.run.owner.id === "coder", "direct run owner should be coder");
  assert(groupEnvelope.run.owner.id === "studio", "group run owner should be studio");
  assert(job.schedule_kind === "cron", "job schedule kind should stay normalized");
  assert(conversations.length >= 2, "conversations should include direct and group sessions");
  assert(directMessages.length === 1, "direct conversation should contain one message");
  assert(groupMessages.length === 1, "group conversation should contain one message");
  assert(runs.length >= 2, "runs should include direct and group entries");
  assert(tasks.length >= 3, "tasks should include direct and group planned tasks");
  assert(directRunTasks.length === 1, "direct run should keep one response task");
  assert(groupRunTasks.length === 2, "group run should create one task per addressed agent");
  assert(directArtifacts.length === 1, "direct run should expose one artifact");
  assert(artifacts.length >= 2, "artifacts should include all persisted summaries");
  assert(jobs.length >= 1, "jobs should include the created job");
  assert(memories.length >= 2, "memories should include created conversation summaries");
  assert(
    memories.some((memory) => memory.namespace.includes(directConversation.conversation.id)),
    "memory should retain direct conversation ledger",
  );

  const directArtifactPath = join(
    runtimeDir,
    "agents",
    "coder",
    "artifacts",
    "runs",
    directEnvelope.run.id,
    "summary.json",
  );
  const groupArtifactPath = join(
    runtimeDir,
    "spaces",
    "studio",
    "artifacts",
    "runs",
    groupEnvelope.run.id,
    "summary.json",
  );

  assert(existsSync(directArtifactPath), "direct run artifact should exist");
  assert(existsSync(groupArtifactPath), "group run artifact should exist");

  const directArtifact = JSON.parse(readFileSync(directArtifactPath, "utf8"));
  const groupArtifact = JSON.parse(readFileSync(groupArtifactPath, "utf8"));

  assert(directArtifact.goal === "实现 settings 页面", "direct artifact goal should match");
  assert(groupArtifact.goal === "整理 roadmap", "group artifact goal should match");

  console.log("[e2e] platform smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}
