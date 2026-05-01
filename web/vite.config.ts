import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { buildWebModuleAliases } from "../scripts/web-build-shared.mjs";

function requireEnv(name: string) {
  const value = process.env[name]?.trim();
  if (!value) {
    throw new Error(`${name} is required when running the Ennoia web dev server.`);
  }
  return value;
}

function requirePort(name: string) {
  const value = Number.parseInt(requireEnv(name), 10);
  if (!Number.isInteger(value) || value <= 0) {
    throw new Error(`${name} must be a positive integer.`);
  }
  return value;
}

export default defineConfig(({ command }) => {
  const isServe = command === "serve";
  const devHost = isServe ? requireEnv("ENNOIA_WEB_DEV_HOST") : undefined;
  const devPort = isServe ? requirePort("ENNOIA_WEB_DEV_PORT") : undefined;
  const apiBaseUrl = isServe ? requireEnv("VITE_ENNOIA_API_URL") : undefined;

  return {
    envDir: path.resolve(__dirname, ".."),
    envPrefix: ["VITE_", "ENNOIA_"],
    plugins: [react()],
    resolve: {
      alias: [
        { find: "@", replacement: path.resolve(__dirname, "./src") },
        ...buildWebModuleAliases(__dirname),
      ],
    },
    server: isServe
      ? {
          host: devHost,
          port: devPort,
          proxy: {
            "/api": {
              target: apiBaseUrl,
              changeOrigin: true,
            },
            "/health": {
              target: apiBaseUrl,
              changeOrigin: true,
            },
          },
          fs: {
            allow: [
              path.resolve(__dirname),
              path.resolve(__dirname, ".."),
            ],
          },
        }
      : undefined,
  };
});
