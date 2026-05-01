import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const [pkgName, relativeBinPath, ...args] = process.argv.slice(2);

if (!pkgName || !relativeBinPath) {
  console.error("usage: node scripts/run-web-bin.mjs <package> <relative-bin-path> [...args]");
  process.exit(2);
}

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const webDir = resolve(rootDir, "web");
const webRequire = createRequire(resolve(webDir, "package.json"));
const packageJsonPath = webRequire.resolve(`${pkgName}/package.json`);
const binPath = join(dirname(packageJsonPath), relativeBinPath);

const result = spawnSync(process.execPath, [binPath, ...args], {
  cwd: webDir,
  stdio: "inherit",
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
