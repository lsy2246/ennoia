import type { ComponentType } from "react";
import type { ExtensionPageContribution } from "@ennoia/ui-sdk";

type ExtensionPageModule = {
  default: ComponentType;
};

const builtinExtensionPageModules = import.meta.glob<ExtensionPageModule>(
  "../../../../builtins/extensions/*/ui/page/Page.tsx",
);

export async function loadExtensionPageComponent(page: ExtensionPageContribution) {
  const modulePath = `../../../../builtins/extensions/${page.extension_id}/ui/page/Page.tsx`;
  const loadModule = builtinExtensionPageModules[modulePath];
  if (!loadModule) {
    return null;
  }

  const module = await loadModule();
  return module.default ?? null;
}
