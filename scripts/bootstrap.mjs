import { spawnSync } from "node:child_process";

import { existsSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const homeDir = homedir();
const cargoBinDir = join(homeDir, ".cargo", "bin");
const localCargoPath =
  process.platform === "win32"
    ? join(cargoBinDir, "cargo.exe")
    : join(cargoBinDir, "cargo");

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

function detectVsDevCmd() {
  if (process.platform !== "win32") {
    return null;
  }

  const programFilesX86 = process.env["ProgramFiles(x86)"] ?? process.env.ProgramFiles;
  const vswherePath = programFilesX86
    ? join(programFilesX86, "Microsoft Visual Studio", "Installer", "vswhere.exe")
    : null;

  if (vswherePath && existsSync(vswherePath)) {
    const result = spawnSync(
      vswherePath,
      [
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-find",
        "Common7\\Tools\\VsDevCmd.bat",
      ],
      { cwd: rootDir, encoding: "utf8" },
    );

    const detected = result.stdout?.trim();
    if (result.status === 0 && detected) {
      return detected;
    }
  }

  const programRoots = [process.env["ProgramFiles(x86)"], process.env.ProgramFiles].filter(Boolean);
  const versions = ["2022", "2019"];
  const editions = ["BuildTools", "Community", "Professional", "Enterprise"];

  for (const programRoot of programRoots) {
    for (const version of versions) {
      for (const edition of editions) {
        const candidate = join(
          programRoot,
          "Microsoft Visual Studio",
          version,
          edition,
          "Common7",
          "Tools",
          "VsDevCmd.bat",
        );

        if (existsSync(candidate)) {
          return candidate;
        }
      }
    }
  }

  return null;
}

function runWindowsCargoCheck() {
  const vsDevCmd = detectVsDevCmd();

  if (!vsDevCmd) {
    console.warn(
      "[bootstrap] 未检测到 Visual Studio Build Tools，已跳过 Rust 校验。安装 C++ Build Tools 后可重新执行 `cargo check --workspace`。",
    );
    return;
  }

  const scriptPath = resolve(tmpdir(), "ennoia-bootstrap-rust-check.cmd");
  const script = [
    "@echo off",
    `call "${vsDevCmd}" -arch=x64 -host_arch=x64`,
    `set PATH=${cargoBinDir};%PATH%`,
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

const shellDir = resolve(rootDir, "web", "apps", "shell");
if (!existsSync(shellDir)) {
  console.error("[bootstrap] 缺少 web/apps/shell 目录，无法继续。");
  process.exit(1);
}

runStep("安装 web/apps/shell 依赖", "bun", ["install", "--cwd", "web/apps/shell"]);
runStep("执行 web/apps/shell typecheck", "bun", ["run", "--cwd", "web/apps/shell", "typecheck"]);

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
