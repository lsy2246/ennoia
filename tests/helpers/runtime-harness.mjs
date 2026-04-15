import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const isWindows = process.platform === "win32";
const cargoExecutable = resolveCargoExecutable();

export function createRuntimeFixture(prefix) {
  return mkdtempSync(join(tmpdir(), `ennoia-${prefix}-`));
}

export function cleanupRuntimeFixture(runtimeDir) {
  rmSync(runtimeDir, { recursive: true, force: true });
}

export function initRuntime(runtimeDir) {
  runCargo(["run", "-p", "ennoia-cli", "--bin", "ennoia", "--", "init", runtimeDir], "runtime init");
}

export function configureRuntimePort(runtimeDir, port) {
  const serverConfigPath = join(runtimeDir, "config", "server.toml");
  const current = readFileSync(serverConfigPath, "utf8");
  const next = current.replace(/port = \d+/, `port = ${port}`);
  writeFileSync(serverConfigPath, next, "utf8");
}

export function startServer(runtimeDir) {
  const child = spawn(...spawnOptionsForCargo([
    "run",
    "-p",
    "ennoia-cli",
    "--bin",
    "ennoia",
    "--",
    "start",
    runtimeDir,
  ], {
    cwd: repoRoot,
    stdio: ["ignore", "pipe", "pipe"],
    env: process.env,
    shell: false,
  }));

  let stderr = "";
  let stdout = "";

  child.stdout.on("data", (chunk) => {
    stdout += chunk.toString();
  });

  child.stderr.on("data", (chunk) => {
    stderr += chunk.toString();
  });

  return {
    child,
    runtimeDir,
    getLogs() {
      return [stdout.trim(), stderr.trim()].filter(Boolean).join("\n");
    },
  };
}

export async function waitForServer(baseUrl, handle, timeoutMs = 30000) {
  const startedAt = Date.now();

  while (Date.now() - startedAt < timeoutMs) {
    if (handle.child.exitCode !== null) {
      throw new Error(`server exited early\n${handle.getLogs()}`);
    }

    try {
      const response = await fetch(`${baseUrl}/health`);
      if (response.ok) {
        return;
      }
    } catch {
      // Wait for the next probe while the server binds the socket.
    }

    await sleep(500);
  }

  throw new Error(`server did not become ready in time\n${handle.getLogs()}`);
}

export async function stopServer(handle) {
  if (handle.child.exitCode !== null) {
    return;
  }

  handle.child.kill("SIGTERM");
  await sleep(500);

  if (handle.child.exitCode === null) {
    handle.child.kill("SIGKILL");
    await sleep(500);
  }
}

export async function fetchJson(baseUrl, path, init) {
  const response = await fetch(`${baseUrl}${path}`, {
    headers: {
      "content-type": "application/json",
      ...(init?.headers ?? {}),
    },
    ...init,
  });

  if (!response.ok) {
    throw new Error(`request failed ${response.status}: ${path}`);
  }

  return response.json();
}

export function assert(condition, message) {
  if (!condition) {
    throw new Error(message);
  }
}

export function assertExists(path, label) {
  assert(existsSync(path), `${label} 缺失: ${path}`);
}

export function buildBaseUrl(port) {
  return `http://127.0.0.1:${port}`;
}

export function nextPort(seed = 0) {
  return 3800 + ((process.pid + seed) % 400);
}

function runCargo(args, label) {
  const result = spawnSync(...spawnOptionsForCargo(args, {
    cwd: repoRoot,
    stdio: "inherit",
    env: process.env,
    shell: false,
  }));

  if (result.error) {
    throw new Error(`${label} failed: ${result.error.message}`);
  }

  if (result.status !== 0) {
    throw new Error(`${label} failed with exit code ${result.status ?? 1}`);
  }
}

function sleep(ms) {
  return new Promise((resolvePromise) => setTimeout(resolvePromise, ms));
}

function spawnOptionsForCargo(args, options) {
  return [cargoExecutable, args, options];
}

function resolveCargoExecutable() {
  if (isWindows) {
    const localCargo = join(process.env.USERPROFILE ?? ".", ".cargo", "bin", "cargo.exe");
    if (existsSync(localCargo)) {
      return localCargo;
    }

    return "cargo.exe";
  }

  return "cargo";
}
