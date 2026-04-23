import { existsSync, readdirSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import react from "@vitejs/plugin-react";
import { build } from "vite";

const rootDir = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const webDir = resolve(rootDir, "web");
const rootNodeModules = resolve(rootDir, "node_modules");
const watch = process.argv.includes("--watch");
const explicitRoots = process.argv.filter((arg) => !arg.startsWith("--")).slice(2);

function discoverExtensionRoots() {
  if (explicitRoots.length > 0) {
    return explicitRoots.map((item) => resolve(rootDir, item));
  }
  const builtinsDir = resolve(rootDir, "builtins", "extensions");
  if (!existsSync(builtinsDir)) {
    return [];
  }
  return readdirSync(builtinsDir, { withFileTypes: true })
    .filter((entry) => entry.isDirectory())
    .map((entry) => resolve(builtinsDir, entry.name));
}

function discoverEntry(extensionRoot) {
  for (const candidate of ["ui/entry.tsx", "ui/entry.ts", "ui/entry.jsx", "ui/entry.js"]) {
    const path = resolve(extensionRoot, candidate);
    if (existsSync(path)) {
      return path;
    }
  }
  return null;
}

function extensionId(extensionRoot) {
  return extensionRoot.split(/[\\/]/).at(-1) ?? extensionRoot;
}

function extensionBuildConfig(extensionRoot, entry) {
  return {
    configFile: false,
    root: extensionRoot,
    envDir: rootDir,
    publicDir: false,
    plugins: [react()],
    resolve: {
      alias: {
        "@ennoia/api-client": resolve(webDir, "packages/api-client/src"),
        "@ennoia/contract": resolve(webDir, "packages/contract/src"),
        "@ennoia/i18n": resolve(webDir, "packages/i18n/src"),
        "@ennoia/observability": resolve(webDir, "packages/observability/src"),
        "@ennoia/theme-runtime": resolve(webDir, "packages/theme-runtime/src"),
        "@ennoia/ui-sdk": resolve(webDir, "packages/ui-sdk/src"),
        react: resolve(rootNodeModules, "react"),
        "react/jsx-runtime": resolve(rootNodeModules, "react/jsx-runtime.js"),
        "react-dom": resolve(rootNodeModules, "react-dom"),
      },
    },
    build: {
      target: "es2022",
      sourcemap: true,
      emptyOutDir: true,
      outDir: resolve(extensionRoot, "ui", "dist"),
      lib: {
        entry,
        formats: ["es"],
        fileName: () => "entry.js",
      },
      rollupOptions: {
        output: {
          chunkFileNames: "assets/[name]-[hash].js",
          assetFileNames: "assets/[name]-[hash][extname]",
        },
      },
      watch: watch ? {} : undefined,
    },
  };
}

const buildTargets = discoverExtensionRoots()
  .map((extensionRoot) => ({ extensionRoot, entry: discoverEntry(extensionRoot) }))
  .filter((target) => target.entry);

if (buildTargets.length === 0) {
  console.log("[extension-ui] 没有发现 ui/entry.*，跳过。");
  process.exit(0);
}

for (const target of buildTargets) {
  console.log(`[extension-ui] ${watch ? "监听" : "构建"} ${extensionId(target.extensionRoot)} UI`);
  const result = await build(extensionBuildConfig(target.extensionRoot, target.entry));
  if (watch && typeof result === "object" && "on" in result) {
    result.on("event", (event) => {
      if (event.code === "ERROR") {
        console.error(`[extension-ui] ${extensionId(target.extensionRoot)} 构建失败`, event.error);
      }
      if (event.code === "END") {
        console.log(`[extension-ui] ${extensionId(target.extensionRoot)} 构建完成`);
      }
    });
  }
}
