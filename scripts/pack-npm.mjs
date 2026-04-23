import { cpSync, existsSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const packagingDir = resolve(rootDir, "packaging", "npm");
const vendorDir = resolve(packagingDir, "vendor");
const outDir = resolve(rootDir, "dist", "npm");
const isWindows = process.platform === "win32";
const executableName = isWindows ? "ennoia.exe" : "ennoia";
const builtBinary = resolve(rootDir, "target", "release", executableName);

function run(label, command, args, cwd = rootDir, options = {}) {
  console.log(`[pack:npm] ${label}`);
  const result = spawnSync(command, args, {
    cwd,
    stdio: "inherit",
    shell: options.shell ?? false,
  });

  if (result.error) {
    console.error(`[pack:npm] ${label}失败: ${result.error.message}`);
    process.exit(1);
  }

  if (result.status !== 0) {
    process.exit(result.status ?? 1);
  }
}

function syncPackageVersion() {
  const rootPackagePath = resolve(rootDir, "package.json");
  const packagingPackagePath = resolve(packagingDir, "package.json");
  const rootPackage = JSON.parse(readFileSync(rootPackagePath, "utf8"));
  const packagingPackage = JSON.parse(readFileSync(packagingPackagePath, "utf8"));

  if (packagingPackage.version === rootPackage.version) {
    return;
  }

  packagingPackage.version = rootPackage.version;
  writeFileSync(packagingPackagePath, `${JSON.stringify(packagingPackage, null, 2)}\n`, "utf8");
}

syncPackageVersion();
run("构建扩展 UI bundle", "node", ["./scripts/build-extension-ui.mjs"]);
run("构建 release CLI", "cargo", ["build", "--release", "--bin", "ennoia"]);

if (!existsSync(builtBinary)) {
  console.error(`[pack:npm] 未找到已构建二进制: ${builtBinary}`);
  process.exit(1);
}

rmSync(vendorDir, { recursive: true, force: true });
rmSync(outDir, { recursive: true, force: true });

try {
  mkdirSync(vendorDir, { recursive: true });
  cpSync(builtBinary, join(vendorDir, executableName));

  mkdirSync(outDir, { recursive: true });
  run("打包 npm tarball", "npm", ["pack", "--pack-destination", outDir], packagingDir, {
    shell: isWindows,
  });
  console.log(`[pack:npm] 完成，产物目录: ${outDir}`);
} finally {
  rmSync(vendorDir, { recursive: true, force: true });
}
