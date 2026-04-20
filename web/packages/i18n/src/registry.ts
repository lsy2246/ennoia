import { bundleFromModule } from "./catalog";
import type { BundleSelector, I18nRegistry, MessageModule, TranslationBundle } from "./types";

class MemoryI18nRegistry implements I18nRegistry {
  private readonly modules = new Map<string, MessageModule>();
  private readonly runtimeBundles = new Map<string, Map<string, TranslationBundle>>();
  private revision = 0;

  registerModule(module: MessageModule) {
    this.modules.set(module.namespace, module);
    this.revision += 1;
  }

  registerBundles(bundles: TranslationBundle[]) {
    for (const bundle of bundles) {
      const localeBuckets =
        this.runtimeBundles.get(bundle.locale) ?? new Map<string, TranslationBundle>();
      localeBuckets.set(bundle.namespace, bundle);
      this.runtimeBundles.set(bundle.locale, localeBuckets);
    }
    if (bundles.length > 0) {
      this.revision += 1;
    }
  }

  clearRuntimeBundles() {
    if (this.runtimeBundles.size === 0) {
      return;
    }
    this.runtimeBundles.clear();
    this.revision += 1;
  }

  getBundles(locale: string, selector?: BundleSelector): TranslationBundle[] {
    const namespaces = selector?.namespaces ?? this.getNamespaces();
    const runtimeLocaleBuckets = this.runtimeBundles.get(locale);

    return namespaces.flatMap((namespace) => {
      const runtimeBundle = runtimeLocaleBuckets?.get(namespace);
      const module = this.modules.get(namespace);
      if (!module) {
        return runtimeBundle ? [runtimeBundle] : [];
      }

      const builtinBundle = bundleFromModule(locale, module);
      return runtimeBundle ? [runtimeBundle, builtinBundle] : [builtinBundle];
    });
  }

  getNamespaces(): string[] {
    const namespaces = new Set<string>(this.modules.keys());
    for (const localeBuckets of this.runtimeBundles.values()) {
      for (const namespace of localeBuckets.keys()) {
        namespaces.add(namespace);
      }
    }
    return [...namespaces];
  }

  getRevision() {
    return this.revision;
  }
}

export function createI18nRegistry(seedModules: MessageModule[] = []): I18nRegistry {
  const registry = new MemoryI18nRegistry();
  for (const module of seedModules) {
    registry.registerModule(module);
  }
  return registry;
}
