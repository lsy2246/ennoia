import type { MessageModule, TranslationBundle, TranslationMessages } from "./types";

export function defineMessages(
  namespace: string,
  messages: Record<string, TranslationMessages>,
): MessageModule {
  return {
    namespace,
    messages,
    source: "builtin",
    version: "1",
    fallback_locale: "en-US",
  };
}

function findLocaleVariant(
  locale: string,
  messages: Record<string, TranslationMessages>,
  fallbackLocale = "en-US",
) {
  const normalized = locale.toLowerCase();
  const language = normalized.split("-")[0];
  const fallbackLanguage = fallbackLocale.toLowerCase().split("-")[0];
  const candidates = Object.keys(messages);

  const exact =
    candidates.find((candidate) => candidate.toLowerCase() === normalized) ??
    candidates.find((candidate) => candidate.toLowerCase().split("-")[0] === language) ??
    candidates.find((candidate) => candidate.toLowerCase() === fallbackLocale.toLowerCase()) ??
    candidates.find((candidate) => candidate.toLowerCase().split("-")[0] === fallbackLanguage) ??
    candidates.find((candidate) => candidate.toLowerCase() === "en-us") ??
    candidates[0];

  if (!exact) {
    return {
      resolved_locale: fallbackLocale,
      messages: {},
    };
  }

  return {
    resolved_locale: exact,
    messages: messages[exact] ?? {},
  };
}

export function bundleFromModule(locale: string, module: MessageModule): TranslationBundle {
  const resolved = findLocaleVariant(locale, module.messages, module.fallback_locale);
  return {
    locale,
    namespace: module.namespace,
    messages: resolved.messages,
    source: module.source ?? "builtin",
    version: module.version,
    resolved_locale: resolved.resolved_locale,
  };
}

export function getMessagesForLocale(
  locale: string,
  modules: MessageModule[],
): TranslationMessages {
  return modules.reduce<TranslationMessages>((accumulator, module) => {
    const bundle = bundleFromModule(locale, module);
    return { ...accumulator, ...bundle.messages };
  }, {});
}
