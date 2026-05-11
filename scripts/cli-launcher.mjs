import { spawn, spawnSync } from "node:child_process";
import {
  copyFileSync,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  rmSync,
  unlinkSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const [command, ...args] = process.argv.slice(2);
const wantsHelp = args.includes("--help") || args.includes("-h");
const isWindows = process.platform === "win32";
const binaryName = isWindows ? "ennoia.exe" : "ennoia";
const buildTargetDir = path.join(repoRoot, "target", "ennoia-cli-build");
const builtBinary = path.join(buildTargetDir, "debug", binaryName);
const snapshotDir = path.join(repoRoot, "target", "ennoia-cli-snapshots");
const snapshotPath = path.join(
  snapshotDir,
  isWindows
    ? `ennoia-cli-${command}-${Date.now()}.exe`
    : `ennoia-cli-${command}-${Date.now()}`,
);

if (!isSupportedCommand(command)) {
  const suffix = command ? `: ${command}` : "";
  console.error(`unknown launcher command${suffix}\n\n${summaryText()}`);
  process.exit(1);
}

if (wantsHelp) {
  console.log(helpText(command));
  process.exit(0);
}

const validationError = validateArgs(command, args);
if (validationError) {
  console.error(`${validationError}\n\n${helpText(command)}`);
  process.exit(1);
}

if (command === "stop") {
  try {
    stopRuntime(args);
    process.exit(0);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exit(1);
  }
}

if (shouldBuild(command, builtBinary)) {
  const build = spawnSync("cargo", ["build", "-p", "ennoia-cli"], {
    cwd: repoRoot,
    env: {
      ...process.env,
      CARGO_TARGET_DIR: buildTargetDir,
    },
    stdio: "inherit",
  });

  if (build.status !== 0) {
    process.exit(build.status ?? 1);
  }
}

if (!existsSync(builtBinary)) {
  console.error(`built CLI binary not found: ${builtBinary}`);
  process.exit(1);
}

mkdirSync(snapshotDir, { recursive: true });
pruneOldSnapshots(snapshotDir, snapshotPath);
copyFileSync(builtBinary, snapshotPath);

const child = spawn(snapshotPath, [command, ...args], {
  cwd: repoRoot,
  stdio: "inherit",
});

let forwarded = false;
const forwardSignal = (signal) => {
  if (forwarded || child.killed) {
    return;
  }
  forwarded = true;
  try {
    child.kill(signal);
  } catch {
    // ignore and let child exit on its own
  }
};

process.on("SIGINT", () => forwardSignal("SIGINT"));
process.on("SIGTERM", () => forwardSignal("SIGTERM"));

child.on("exit", (code, signal) => {
  if (signal) {
    process.exit(1);
  }
  process.exit(code ?? 0);
});

child.on("error", (error) => {
  console.error(error);
  process.exit(1);
});

function isSupportedCommand(value) {
  return value === "dev" || value === "start" || value === "stop";
}

function shouldBuild(currentCommand, binaryPath) {
  return currentCommand !== "stop" || !existsSync(binaryPath);
}

function stopRuntime(values) {
  const target = resolveStopTarget(values);
  const pidPath =
    target.kind === "dev"
      ? path.join(target.home, "data", "system", "dev.pid")
      : path.join(target.home, "data", "system", "server.pid");
  const label = target.kind === "dev" ? "dev runtime" : "server runtime";
  const pid = readPidFile(pidPath);
  if (pid === null) {
    console.log(`${label} is not running (${pidPath} is missing)`);
    return;
  }

  if (!isProcessRunning(pid)) {
    removePidFileIfMatches(pidPath, pid);
    console.log(`${label} is not running; removed stale pid file ${pidPath}`);
    return;
  }

  terminateProcess(pid);
  waitForProcessExit(pid, 8000);
  removePidFileIfMatches(pidPath, pid);
  console.log(`stopped ${label} (pid ${pid})`);
}

function resolveStopTarget(values) {
  if (values.length === 0) {
    const devHome = path.join(repoRoot, ".dev");
    if (existsSync(devHome)) {
      return { kind: "dev", home: devHome };
    }
    return { kind: "runtime", home: resolveRuntimeHome() };
  }
  if (values[0] === "dev" || values[0] === "--dev" || values[0] === "-d") {
    return { kind: "dev", home: path.join(repoRoot, ".dev") };
  }
  return { kind: "runtime", home: values[0] };
}

function resolveRuntimeHome() {
  return process.env.ENNOIA_HOME || defaultHomeDir();
}

function defaultHomeDir() {
  const root = process.env.USERPROFILE || process.env.HOME || ".";
  return path.join(root, ".ennoia");
}

function readPidFile(pidPath) {
  if (!existsSync(pidPath)) {
    return null;
  }
  const value = Number.parseInt(readFileSync(pidPath, "utf8").trim(), 10);
  if (!Number.isFinite(value)) {
    throw new Error(`invalid pid file contents at ${pidPath}`);
  }
  return value;
}

function removePidFileIfMatches(pidPath, pid) {
  try {
    const current = readPidFile(pidPath);
    if (current === pid) {
      unlinkSync(pidPath);
    }
  } catch {
    // ignore pid file races while shutting down
  }
}

function isProcessRunning(pid) {
  if (isWindows) {
    const result = spawnSync("tasklist", ["/FI", `PID eq ${pid}`, "/FO", "CSV", "/NH"], {
      stdio: ["ignore", "pipe", "ignore"],
      encoding: "utf8",
    });
    const stdout = result.stdout ?? "";
    return stdout.includes(`"${pid}"`);
  }

  try {
    process.kill(pid, 0);
    return true;
  } catch {
    return false;
  }
}

function terminateProcess(pid) {
  if (isWindows) {
    let result = spawnSync("taskkill", ["/PID", String(pid), "/T"], {
      stdio: "ignore",
    });
    if (result.status !== 0 && isProcessRunning(pid)) {
      result = spawnSync("taskkill", ["/F", "/PID", String(pid), "/T"], {
        stdio: "ignore",
      });
    }
    if (result.status !== 0 && isProcessRunning(pid)) {
      throw new Error(`taskkill failed for pid ${pid}`);
    }
    return;
  }

  process.kill(pid, "SIGTERM");
}

function forceKillProcess(pid) {
  if (isWindows) {
    spawnSync("taskkill", ["/F", "/PID", String(pid), "/T"], {
      stdio: "ignore",
    });
    return;
  }

  process.kill(pid, "SIGKILL");
}

function waitForProcessExit(pid, timeoutMs) {
  const started = Date.now();
  while (Date.now() - started < timeoutMs) {
    if (!isProcessRunning(pid)) {
      return;
    }
    sleep(120);
  }

  if (isProcessRunning(pid)) {
    forceKillProcess(pid);
  }
}

function sleep(ms) {
  const end = Date.now() + ms;
  while (Date.now() < end) {
    // busy wait is fine here because stop is a short-lived maintenance command
  }
}

function pruneOldSnapshots(dir, keepPath) {
  const files = readdirSync(dir)
    .filter((name) => name.startsWith("ennoia-cli-"))
    .map((name) => path.join(dir, name))
    .filter((filePath) => filePath !== keepPath)
    .sort();
  const excess = Math.max(0, files.length - 8);
  for (const filePath of files.slice(0, excess)) {
    try {
      rmSync(filePath, { force: true });
    } catch {
      // ignore stale locked snapshots on Windows
    }
  }
}

function validateArgs(currentCommand, values) {
  if (values.length === 0) {
    return null;
  }
  if (
    currentCommand === "stop" &&
    (values[0] === "--dev" || values[0] === "-d")
  ) {
    if (values.length === 1) {
      return null;
    }
    return `too many arguments for 'npm run ${currentCommand}'`;
  }
  if (values[0].startsWith("-")) {
    return `unknown option for 'npm run ${currentCommand}': ${values[0]}`;
  }
  if (currentCommand === "dev" || currentCommand === "start") {
    if (values.length === 1) {
      return `unexpected argument for 'npm run ${currentCommand}': ${values[0]}`;
    }
    return `too many arguments for 'npm run ${currentCommand}'`;
  }
  if (values.length === 1) {
    return null;
  }
  return `too many arguments for 'npm run ${currentCommand}'`;
}

function helpText(currentCommand) {
  if (currentCommand === "dev") {
    return `usage: npm run dev

Starts the Ennoia dev runtime.

This command forwards to:
  ennoia dev

The dev runtime always uses the repository-local .dev directory.`;
  }
  if (currentCommand === "start") {
    return `usage: npm run start

Starts the Ennoia runtime using the compiled local CLI binary.

This command forwards to:
  ennoia start

The runtime home is resolved from ENNOIA_HOME or the default ~/.ennoia directory.`;
  }
  return `usage: npm run stop [dev|--dev|-d]
       npm run stop -- [home]

Stops the current Ennoia runtime by reading the local pid file directly.

Without an argument, the repository-local .dev runtime is stopped when ./.dev exists.`;
}

function summaryText() {
  return `usage:
  npm run dev
  npm run start
  npm run stop [dev|--dev|-d]
  npm run stop -- [home]`;
}
