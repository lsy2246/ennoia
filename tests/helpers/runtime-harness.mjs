import { spawn, spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const repoRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..", "..");
const isWindows = process.platform === "win32";
const cargoExecutable = resolveCargoExecutable();
const cliExecutable = resolveCliExecutable();
let cliBuilt = false;

export function createRuntimeFixture(prefix) {
  return mkdtempSync(join(tmpdir(), `ennoia-${prefix}-`));
}

export function cleanupRuntimeFixture(runtimeDir) {
  killProcessesForRuntime(runtimeDir);
  retrySync(() => rmSync(runtimeDir, { recursive: true, force: true }));
}

export function initRuntime(runtimeDir) {
  ensureCliBuilt();
  runCli(["init", runtimeDir], "runtime init");
}

export function configureRuntimePort(runtimeDir, port) {
  const serverConfigPath = join(runtimeDir, "config", "server.toml");
  const current = readFileSync(serverConfigPath, "utf8");
  const next = current.replace(/port = \d+/, `port = ${port}`);
  writeFileSync(serverConfigPath, next, "utf8");
}

export function startServer(runtimeDir) {
  ensureCliBuilt();
  const child = spawn(...spawnOptionsForCli(["start", runtimeDir], {
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
  await waitForChildExit(handle.child, 2000);

  if (handle.child.exitCode === null) {
    if (isWindows && handle.child.pid) {
      spawnSync("taskkill", ["/PID", String(handle.child.pid), "/T", "/F"], {
        stdio: "ignore",
        shell: false,
      });
    } else {
      handle.child.kill("SIGKILL");
    }

    await waitForChildExit(handle.child, 3000);
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

function runCli(args, label) {
  const result = spawnSync(...spawnOptionsForCli(args, {
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

function waitForChildExit(child, timeoutMs) {
  if (child.exitCode !== null) {
    return Promise.resolve();
  }

  return new Promise((resolvePromise) => {
    const timer = setTimeout(() => {
      child.off("exit", onExit);
      resolvePromise();
    }, timeoutMs);

    function onExit() {
      clearTimeout(timer);
      resolvePromise();
    }

    child.once("exit", onExit);
  });
}

function spawnOptionsForCargo(args, options) {
  return [cargoExecutable, args, options];
}

function spawnOptionsForCli(args, options) {
  return [cliExecutable, args, options];
}

function ensureCliBuilt() {
  if (cliBuilt) {
    return;
  }

  if (existsSync(cliExecutable)) {
    cliBuilt = true;
    return;
  }

  runCargo(["build", "-p", "ennoia-cli", "--bin", "ennoia"], "build ennoia cli");
  cliBuilt = true;
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

function resolveCliExecutable() {
  return join(repoRoot, "target", "debug", isWindows ? "ennoia.exe" : "ennoia");
}

function retrySync(action, attempts = 10, delayMs = 200) {
  let lastError;

  for (let index = 0; index < attempts; index += 1) {
    try {
      action();
      return;
    } catch (error) {
      lastError = error;
      Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delayMs);
    }
  }

  throw lastError;
}

function killProcessesForRuntime(runtimeDir) {
  if (!isWindows) {
    return;
  }

  spawnSync(
    "powershell.exe",
    [
      "-NoProfile",
      "-NonInteractive",
      "-Command",
      "$needle = $env:ENNOIA_CLEANUP_DIR; $alt = $env:ENNOIA_CLEANUP_DIR_ALT; Get-CimInstance Win32_Process | Where-Object { $_.CommandLine -like \"*$needle*\" -or $_.CommandLine -like \"*$alt*\" } | ForEach-Object { taskkill /PID $_.ProcessId /T /F | Out-Null }",
    ],
    {
      env: {
        ...process.env,
        ENNOIA_CLEANUP_DIR: runtimeDir,
        ENNOIA_CLEANUP_DIR_ALT: runtimeDir.replaceAll("\\", "/"),
      },
      stdio: "ignore",
      shell: false,
    },
  );
}
