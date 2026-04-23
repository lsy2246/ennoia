import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

const devPort = Number(process.env.ENNOIA_WEB_DEV_PORT ?? process.env.VITE_PORT ?? 5173);
const rootNodeModules = path.resolve(__dirname, "../node_modules");

export default defineConfig({
  envDir: path.resolve(__dirname, ".."),
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@ennoia/api-client": path.resolve(__dirname, "./packages/api-client/src"),
      "@ennoia/contract": path.resolve(__dirname, "./packages/contract/src"),
      "@ennoia/i18n": path.resolve(__dirname, "./packages/i18n/src"),
      "@ennoia/observability": path.resolve(__dirname, "./packages/observability/src"),
      "@ennoia/theme-runtime": path.resolve(__dirname, "./packages/theme-runtime/src"),
      "@ennoia/ui-sdk": path.resolve(__dirname, "./packages/ui-sdk/src"),
      react: path.resolve(rootNodeModules, "react"),
      "react/jsx-runtime": path.resolve(rootNodeModules, "react/jsx-runtime.js"),
      "react-dom": path.resolve(rootNodeModules, "react-dom"),
    },
  },
  server: {
    port: devPort,
    fs: {
      allow: [
        path.resolve(__dirname),
        path.resolve(__dirname, ".."),
      ],
    },
  },
});
