import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";

export default defineConfig({
  plugins: [react()],
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      "@ennoia/api-client": path.resolve(__dirname, "../../packages/api-client/src"),
      "@ennoia/builtins": path.resolve(__dirname, "../../packages/builtins/src"),
      "@ennoia/contract": path.resolve(__dirname, "../../packages/contract/src"),
      "@ennoia/observability": path.resolve(__dirname, "../../packages/observability/src"),
      "@ennoia/ui-sdk": path.resolve(__dirname, "../../packages/ui-sdk/src"),
    },
  },
  server: {
    port: 5173
  }
});
