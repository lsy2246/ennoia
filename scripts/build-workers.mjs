import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";

function run(command, args) {
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: false,
  });
  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function copyWorker(packageName, outputName, destination) {
  const source = resolve(
    rootDir,
    "target",
    "wasm32-unknown-unknown",
    "release",
    outputName,
  );
  if (!existsSync(source)) {
    console.error(`[workers] missing build output: ${source}`);
    process.exit(1);
  }
  const target = resolve(rootDir, destination);
  mkdirSync(dirname(target), { recursive: true });
  copyFileSync(source, target);
  console.log(`[workers] ${packageName} -> ${destination}`);
}

run("rustup", ["target", "add", "wasm32-unknown-unknown"]);
run(cargo, [
  "build",
  "-p",
  "ennoia-memory-worker",
  "-p",
  "ennoia-workflow-worker",
  "--target",
  "wasm32-unknown-unknown",
  "--release",
]);

copyWorker(
  "ennoia-memory-worker",
  "ennoia_memory_worker.wasm",
  "builtins/extensions/memory/worker/memory.wasm",
);
copyWorker(
  "ennoia-workflow-worker",
  "ennoia_workflow_worker.wasm",
  "builtins/extensions/workflow/worker/workflow.wasm",
);
