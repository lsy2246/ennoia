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

  const privateConversation = await fetchJson(baseUrl, "/api/v1/threads/private/messages", {
    method: "POST",
    body: JSON.stringify({
      agent_id: "coder",
      goal: "实现 settings 页面",
      body: "请整理 settings 页面需求",
    }),
  });
  const spaceConversation = await fetchJson(baseUrl, "/api/v1/threads/space/messages", {
    method: "POST",
    body: JSON.stringify({
      space_id: "studio",
      addressed_agents: ["coder", "planner"],
      goal: "整理 roadmap",
      body: "请一起整理 roadmap",
    }),
  });
  const legacyPrivateRun = await fetchJson(baseUrl, "/api/v1/runs/private", {
    method: "POST",
    body: JSON.stringify({
      agent_id: "planner",
      goal: "整理 phase2 backlog",
      message: "请整理 phase2 backlog",
    }),
  });
  const job = await fetchJson(baseUrl, "/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify({
      owner_kind: "space",
      owner_id: "studio",
      schedule_kind: "cron",
      schedule_value: "0 */6 * * *",
      description: "nightly review",
    }),
  });

  const overview = await fetchJson(baseUrl, "/api/v1/overview");
  const threads = await fetchJson(baseUrl, "/api/v1/threads");
  const privateMessages = await fetchJson(
    baseUrl,
    `/api/v1/threads/${privateConversation.thread.id}/messages`,
  );
  const spaceMessages = await fetchJson(
    baseUrl,
    `/api/v1/threads/${spaceConversation.thread.id}/messages`,
  );
  const runs = await fetchJson(baseUrl, "/api/v1/runs");
  const tasks = await fetchJson(baseUrl, "/api/v1/tasks");
  const privateRunTasks = await fetchJson(
    baseUrl,
    `/api/v1/runs/${privateConversation.run.id}/tasks`,
  );
  const spaceRunTasks = await fetchJson(
    baseUrl,
    `/api/v1/runs/${spaceConversation.run.id}/tasks`,
  );
  const privateArtifacts = await fetchJson(
    baseUrl,
    `/api/v1/runs/${privateConversation.run.id}/artifacts`,
  );
  const artifacts = await fetchJson(baseUrl, "/api/v1/artifacts");
  const jobs = await fetchJson(baseUrl, "/api/v1/jobs");
  const memories = await fetchJson(baseUrl, "/api/v1/memories");

  assert(overview.counts.extensions >= 1, "overview should expose extensions count");
  assert(overview.counts.threads >= 2, "overview should expose thread count");
  assert(overview.counts.messages >= 3, "overview should expose message count");
  assert(privateConversation.run.owner.id === "coder", "private run owner should be coder");
  assert(spaceConversation.run.owner.id === "studio", "space run owner should be studio");
  assert(legacyPrivateRun.run.owner.id === "planner", "legacy run wrapper should stay available");
  assert(job.schedule_kind === "cron", "job schedule kind should stay normalized");
  assert(threads.length >= 2, "threads should include private and space threads");
  assert(privateMessages.length === 1, "private thread should contain one message");
  assert(spaceMessages.length === 1, "space thread should contain one message");
  assert(runs.length >= 3, "runs should include new and legacy run entries");
  assert(tasks.length >= 4, "tasks should include private, space and legacy planned tasks");
  assert(privateRunTasks.length === 1, "private run should keep one response task");
  assert(spaceRunTasks.length === 2, "space run should create one task per addressed agent");
  assert(privateArtifacts.length === 1, "private run should expose one artifact");
  assert(artifacts.length >= 3, "artifacts should include all persisted summaries");
  assert(jobs.length >= 1, "jobs should include the created job");
  assert(memories.length >= 3, "memories should include created context records");
  assert(
    memories.some((memory) => memory.run_id === privateConversation.run.id),
    "memory should bind private run id",
  );

  const privateArtifactPath = join(
    runtimeDir,
    "agents",
    "coder",
    "artifacts",
    "runs",
    privateConversation.run.id,
    "summary.json",
  );
  const spaceArtifactPath = join(
    runtimeDir,
    "spaces",
    "studio",
    "artifacts",
    "runs",
    spaceConversation.run.id,
    "summary.json",
  );

  assert(existsSync(privateArtifactPath), "private run artifact should exist");
  assert(existsSync(spaceArtifactPath), "space run artifact should exist");

  const privateArtifact = JSON.parse(readFileSync(privateArtifactPath, "utf8"));
  const spaceArtifact = JSON.parse(readFileSync(spaceArtifactPath, "utf8"));

  assert(privateArtifact.goal === "实现 settings 页面", "private artifact goal should match");
  assert(spaceArtifact.goal === "整理 roadmap", "space artifact goal should match");

  console.log("[e2e] platform smoke passed");
} finally {
  if (serverHandle) {
    await stopServer(serverHandle);
  }
  cleanupRuntimeFixture(runtimeDir);
}
