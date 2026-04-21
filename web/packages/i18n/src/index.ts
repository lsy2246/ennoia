export type {
  BundleSelector,
  I18nRegistry,
  MessageModule,
  TranslationBundle,
  TranslationMessages,
} from "./types";
export {
  bundleFromModule,
  defineMessages,
  getMessagesForLocale,
} from "./catalog";
export { createI18nRegistry } from "./registry";
export { formatDate, formatDateTime, formatTime, resolveLocalizedText, translate } from "./resolver";

import type { MessageModule, TranslationBundle } from "./types";
import { createI18nRegistry } from "./registry";
import { settingsMessages } from "./modules/settings";
import { shellMessages } from "./modules/shell";
import { webMessages } from "./modules/web";

const BUILTIN_MODULES: MessageModule[] = [
  shellMessages,
  settingsMessages,
  webMessages,
];
export const builtinI18nRegistry = createI18nRegistry(BUILTIN_MODULES);

export function getBuiltinModules(): MessageModule[] {
  return BUILTIN_MODULES;
}

export function getBuiltinNamespaces(): string[] {
  return BUILTIN_MODULES.map((module) => module.namespace);
}

export function getBuiltinBundles(locale: string, namespaces?: string[]): TranslationBundle[] {
  return createBuiltinI18nRegistry().getBundles(locale, { namespaces });
}

export function createBuiltinI18nRegistry() {
  return createI18nRegistry(BUILTIN_MODULES);
}
