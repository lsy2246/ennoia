export type TranslationMessages = Record<string, string>;

export type TranslationBundle = {
  locale: string;
  namespace: string;
  messages: TranslationMessages;
  source: string;
  version?: string;
  resolved_locale?: string;
};

export type MessageModule = {
  namespace: string;
  messages: Record<string, TranslationMessages>;
  source?: string;
  version?: string;
  fallback_locale?: string;
};

export type BundleSelector = {
  namespaces?: string[];
};

export type I18nRegistry = {
  registerModule: (module: MessageModule) => void;
  registerBundles: (bundles: TranslationBundle[]) => void;
  clearRuntimeBundles: () => void;
  getBundles: (locale: string, selector?: BundleSelector) => TranslationBundle[];
  getNamespaces: () => string[];
  getRevision: () => number;
};
