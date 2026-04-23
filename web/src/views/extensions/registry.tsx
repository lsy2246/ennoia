import { getExtensionUiModuleUrl } from "@ennoia/api-client";
import type {
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionPageMount,
  ExtensionPanelMount,
  ExtensionUiModule,
} from "@ennoia/ui-sdk";

type LoadedExtensionUiModule = ExtensionUiModule & {
  default?: ExtensionUiModule;
};

const loadedModules = new Map<string, Promise<LoadedExtensionUiModule>>();

export async function loadExtensionUiModule(extensionId: string, generation: number) {
  const url = `${getExtensionUiModuleUrl(extensionId)}?v=${generation}`;
  const cached = loadedModules.get(url);
  if (cached) {
    return cached;
  }

  const pending = import(/* @vite-ignore */ url) as Promise<LoadedExtensionUiModule>;
  loadedModules.set(url, pending);
  return pending;
}

export async function loadExtensionPageMount(
  page: ExtensionPageContribution,
  generation: number,
): Promise<ExtensionPageMount | null> {
  const module = await loadExtensionUiModule(page.extension_id, generation);
  const resolved = module.default ?? module;
  return resolved.pages?.[page.page.mount] ?? null;
}

export async function loadExtensionPanelMount(
  panel: ExtensionPanelContribution,
  generation: number,
): Promise<ExtensionPanelMount | null> {
  const module = await loadExtensionUiModule(panel.extension_id, generation);
  const resolved = module.default ?? module;
  return resolved.panels?.[panel.panel.mount] ?? null;
}
