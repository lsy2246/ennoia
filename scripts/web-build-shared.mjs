import { createRequire } from "node:module";
import { resolve } from "node:path";

export function createWebPackageRequire(webDir) {
  return createRequire(resolve(webDir, "package.json"));
}

export function buildWebModuleAliases(webDir) {
  const webRequire = createWebPackageRequire(webDir);
  const reactEntry = webRequire.resolve("react");
  const reactJsxRuntimeEntry = webRequire.resolve("react/jsx-runtime");
  const reactDomEntry = webRequire.resolve("react-dom");
  const reactDomClientEntry = webRequire.resolve("react-dom/client");

  return [
    { find: "@ennoia/api-client", replacement: resolve(webDir, "packages/api-client/src") },
    { find: "@ennoia/contract", replacement: resolve(webDir, "packages/contract/src") },
    { find: "@ennoia/i18n", replacement: resolve(webDir, "packages/i18n/src") },
    { find: "@ennoia/observability", replacement: resolve(webDir, "packages/observability/src") },
    { find: "@ennoia/theme-runtime", replacement: resolve(webDir, "packages/theme-runtime/src") },
    { find: "@ennoia/ui-sdk", replacement: resolve(webDir, "packages/ui-sdk/src") },
    { find: /^react\/jsx-runtime$/, replacement: reactJsxRuntimeEntry },
    { find: /^react-dom\/client$/, replacement: reactDomClientEntry },
    { find: /^react-dom$/, replacement: reactDomEntry },
    { find: /^react$/, replacement: reactEntry },
  ];
}
