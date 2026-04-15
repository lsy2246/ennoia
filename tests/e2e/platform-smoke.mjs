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

  const overview = await fetchJson(baseUrl, "/api/v1/overview");
  const privateRun = await fetchJson(baseUrl, "/api/v1/runs/private", {
    method: "POST",
    body: JSON.stringify({
      agent_id: "coder",
      goal: "实现 settings 页面",
      message: "请整理 settings 页面需求",
    }),
  });
  const spaceRun = await fetchJson(baseUrl, "/api/v1/runs/space", {
    method: "POST",
    body: JSON.stringify({
      space_id: "studio",
      addressed_agents: ["coder", "planner"],
      goal: "整理 roadmap",
      message: "请一起整理 roadmap",
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

  const runs = await fetchJson(baseUrl, "/api/v1/runs");
  const tasks = await fetchJson(baseUrl, "/api/v1/tasks");
  const jobs = await fetchJson(baseUrl, "/api/v1/jobs");
  const memories = await fetchJson(baseUrl, "/api/v1/memories");

  assert(overview.counts.extensions >= 1, "overview should expose extensions count");
  assert(privateRun.run.owner.id === "coder", "private run owner should be coder");
  assert(spaceRun.run.owner.id === "studio", "space run owner should be studio");
  assert(job.schedule_kind === "cron", "job schedule kind should stay normalized");
  assert(runs.length >= 2, "runs should include private and space runs");
  assert(tasks.length >= 2, "tasks should include planned tasks");
  assert(jobs.length >= 1, "jobs should include the created job");
  assert(memories.length >= 2, "memories should include created context records");

  const privateArtifactPath = join(
    runtimeDir,
    "agents",
    "coder",
    "artifacts",
    "runs",
    privateRun.run.id,
    "summary.json",
  );
  const spaceArtifactPath = join(
    runtimeDir,
    "spaces",
    "studio",
    "artifacts",
    "runs",
    spaceRun.run.id,
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
