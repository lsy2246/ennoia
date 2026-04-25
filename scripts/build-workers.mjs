import { spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const cargo = process.platform === "win32" ? "cargo.exe" : "cargo";
const conversationBinaryName =
  process.platform === "win32" ? "conversation-service.exe" : "conversation-service";
const memoryBinaryName =
  process.platform === "win32" ? "memory-service.exe" : "memory-service";

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

function copyNativeWorker(packageName, outputName, destination) {
  const source = resolve(rootDir, "target", "release", outputName);
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
  "ennoia-conversation-service",
  "-p",
  "ennoia-memory",
  "--release",
]);
run(cargo, [
  "build",
  "-p",
  "ennoia-workflow-worker",
  "--target",
  "wasm32-unknown-unknown",
  "--release",
]);

copyWorker(
  "ennoia-workflow-worker",
  "ennoia_workflow_worker.wasm",
  "builtins/extensions/workflow/worker/workflow.wasm",
);
copyNativeWorker(
  "ennoia-conversation-service",
  process.platform === "win32"
    ? "ennoia-conversation-service.exe"
    : "ennoia-conversation-service",
  `builtins/extensions/conversation/bin/${conversationBinaryName}`,
);
copyNativeWorker(
  "ennoia-memory",
  process.platform === "win32"
    ? "ennoia-memory-extension.exe"
    : "ennoia-memory-extension",
  `builtins/extensions/memory/bin/${memoryBinaryName}`,
);
