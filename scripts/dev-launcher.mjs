import { spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, rmSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const repoRoot = path.resolve(__dirname, "..");
const args = process.argv.slice(2);
const wantsHelp = args.includes("--help") || args.includes("-h");
const isWindows = process.platform === "win32";
const binaryName = isWindows ? "ennoia.exe" : "ennoia";
const buildTargetDir = path.join(repoRoot, "target", "ennoia-dev-cli-build");
const builtBinary = path.join(buildTargetDir, "debug", binaryName);
const snapshotDir = path.join(repoRoot, "target", "ennoia-dev-cli-snapshots");
const snapshotPath = path.join(
  snapshotDir,
  isWindows ? `ennoia-dev-${Date.now()}.exe` : `ennoia-dev-${Date.now()}`,
);

if (wantsHelp) {
  console.log(helpText());
  process.exit(0);
}

const validationError = validateArgs(args);
if (validationError) {
  console.error(`${validationError}\n\n${helpText()}`);
  process.exit(1);
}

const build = spawnSync("cargo", ["build", "-p", "ennoia-cli"], {
  cwd: repoRoot,
  env: {
    ...process.env,
    CARGO_TARGET_DIR: buildTargetDir,
    CARGO_INCREMENTAL: "0",
  },
  stdio: "inherit",
});

if (build.status !== 0) {
  process.exit(build.status ?? 1);
}

if (!existsSync(builtBinary)) {
  console.error(`built CLI binary not found: ${builtBinary}`);
  process.exit(1);
}

mkdirSync(snapshotDir, { recursive: true });
pruneOldSnapshots(snapshotDir, snapshotPath);
copyFileSync(builtBinary, snapshotPath);

const child = spawn(snapshotPath, ["dev", ...args], {
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

function pruneOldSnapshots(dir, keepPath) {
  const files = readdirSync(dir)
    .filter((name) => name.startsWith("ennoia-dev-"))
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

function validateArgs(values) {
  if (values.length === 0) {
    return null;
  }
  if (values.length > 1) {
    return "too many arguments for 'bun dev'";
  }
  if (values[0].startsWith("-")) {
    return `unknown option for 'bun dev': ${values[0]}`;
  }
  return null;
}

function helpText() {
  return `usage: bun dev [home]

Starts the Ennoia dev runtime.

Arguments:
  home    Optional Ennoia home directory. If omitted, ENNOIA_HOME or ~/.ennoia is used.

This command forwards to:
  ennoia dev [home]`;
}
