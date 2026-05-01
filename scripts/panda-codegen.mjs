import { spawnSync } from "node:child_process";
import { createRequire } from "node:module";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const webDir = resolve(rootDir, "web");
const webRequire = createRequire(resolve(webDir, "package.json"));
const pandaPackageJson = webRequire.resolve("@pandacss/dev/package.json");
const pandaBin = join(dirname(pandaPackageJson), "bin.js");

const result = spawnSync(process.execPath, [pandaBin, "codegen", "--clean"], {
  cwd: webDir,
  stdio: "inherit",
});

if (result.status !== 0) {
  process.exit(result.status ?? 1);
}
