import { spawnSync } from "node:child_process";

import { existsSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const localCargoPath =
  process.platform === "win32"
    ? resolve(process.env.USERPROFILE ?? "C:/Users/Administrator", ".cargo/bin/cargo.exe")
    : resolve(process.env.HOME ?? "~", ".cargo/bin/cargo");

function commandExists(command) {
  if (command === "cargo" && existsSync(localCargoPath)) {
    return true;
  }

  const probe = process.platform === "win32" ? "where" : "which";
  return spawnSync(probe, [command], { cwd: rootDir, stdio: "ignore" }).status === 0;
}

function runStep(label, command, args) {
  console.log(`\n[bootstrap] ${label}`);
  const result = spawnSync(command, args, {
    cwd: rootDir,
    stdio: "inherit",
    shell: false,
  });

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function runWindowsCargoCheck() {
  const vsDevCmd = "C:\\Program Files (x86)\\Microsoft Visual Studio\\2022\\BuildTools\\Common7\\Tools\\VsDevCmd.bat";

  if (!existsSync(vsDevCmd)) {
    console.warn(
      "[bootstrap] 未检测到 Visual Studio Build Tools，已跳过 Rust 校验。安装 C++ Build Tools 后可重新执行 `cargo check --workspace`。",
    );
    return;
  }

  const cargoBin = `${process.env.USERPROFILE ?? "C:\\Users\\Administrator"}\\.cargo\\bin`;
  const scriptPath = resolve(tmpdir(), "ennoia-bootstrap-rust-check.cmd");
  const script = [
    "@echo off",
    `call "${vsDevCmd}" -arch=x64 -host_arch=x64`,
    `set PATH=${cargoBin};%PATH%`,
    "cargo check --workspace",
    "",
  ].join("\r\n");

  writeFileSync(scriptPath, script, "ascii");

  try {
    runStep("执行 Rust workspace 检查", "cmd", ["/c", scriptPath]);
  } finally {
    rmSync(scriptPath, { force: true });
  }
}

if (!commandExists("bun")) {
  console.error("[bootstrap] 未检测到 bun，请先安装 Bun 后再执行。");
  process.exit(1);
}

runStep("安装根目录依赖", "bun", ["install"]);

const shellDir = resolve(rootDir, "web", "shell");
if (!existsSync(shellDir)) {
  console.error("[bootstrap] 缺少 web/shell 目录，无法继续。");
  process.exit(1);
}

runStep("安装 web/shell 依赖", "bun", ["install", "--cwd", "web/shell"]);
runStep("执行 web/shell typecheck", "bun", ["run", "--cwd", "web/shell", "typecheck"]);

if (commandExists("cargo")) {
  if (process.platform === "win32") {
    runWindowsCargoCheck();
  } else {
    runStep("执行 Rust workspace 检查", "cargo", ["check", "--workspace"]);
  }
} else {
  console.warn(
    "[bootstrap] 未检测到 cargo，已跳过 Rust 校验。安装 Rust toolchain 后可执行 `cargo check --workspace`。",
  );
}
