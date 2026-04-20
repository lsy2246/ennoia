import { useParams } from "@tanstack/react-router";
import { useEffect, useMemo, useState, type ComponentType } from "react";

import {
  getExtensionDetail,
  getExtensionFrontendModuleUrl,
} from "@ennoia/api-client";
import type { ExtensionRuntimeExtension } from "@ennoia/ui-sdk";

import { PageHeader } from "@/components/PageHeader";
import { useUiHelpers } from "@/stores/ui";

type LoadedModule = Record<string, unknown>;

function pickMountComponent(
  module: LoadedModule,
  mount: string,
): ComponentType<Record<string, unknown>> | null {
  const fromPages = (module.pages as Record<string, unknown> | undefined)?.[mount];
  if (typeof fromPages === "function") {
    return fromPages as ComponentType<Record<string, unknown>>;
  }

  const fromMounts = (module.mounts as Record<string, unknown> | undefined)?.[mount];
  if (typeof fromMounts === "function") {
    return fromMounts as ComponentType<Record<string, unknown>>;
  }

  if (typeof module[mount] === "function") {
    return module[mount] as ComponentType<Record<string, unknown>>;
  }

  if (typeof module.default === "function") {
    return module.default as ComponentType<Record<string, unknown>>;
  }

  return null;
}

export function DynamicExtensionPage() {
  const { extensionId, pageId } = useParams({ strict: false });
  const { resolveText } = useUiHelpers();
  const [extension, setExtension] = useState<ExtensionRuntimeExtension | null>(null);
  const [Component, setComponent] = useState<ComponentType<Record<string, unknown>> | null>(null);
  const [error, setError] = useState<string | null>(null);

  const resolvedExtensionId = extensionId ?? "";
  const resolvedPageId = pageId ?? "";

  useEffect(() => {
    let disposed = false;
    async function load() {
      try {
        setError(null);
        setComponent(null);
        const detail = await getExtensionDetail(resolvedExtensionId);
        if (disposed) return;
        setExtension(detail);
        const page = detail.pages.find((item) => item.id === resolvedPageId);
        if (!page) {
          throw new Error(`page '${resolvedPageId}' not found in extension '${resolvedExtensionId}'`);
        }

        const moduleUrl = getExtensionFrontendModuleUrl(resolvedExtensionId);
        const loaded = (await import(/* @vite-ignore */ moduleUrl)) as LoadedModule;
        if (disposed) return;

        const picked = pickMountComponent(loaded, page.mount);
        if (!picked) {
          throw new Error(`mount '${page.mount}' not exported by module`);
        }
        setComponent(() => picked);
      } catch (nextError) {
        if (!disposed) {
          setError(String(nextError));
        }
      }
    }

    if (resolvedExtensionId && resolvedPageId) {
      void load();
    }
    return () => {
      disposed = true;
    };
  }, [resolvedExtensionId, resolvedPageId]);

  const page = useMemo(
    () => extension?.pages.find((item) => item.id === resolvedPageId) ?? null,
    [extension, resolvedPageId],
  );

  return (
    <div className="page">
      <PageHeader
        title={page ? resolveText(page.title) : resolvedExtensionId}
        description={page ? `${page.route} · ${page.mount}` : resolvedExtensionId}
        meta={extension ? [extension.name, extension.health, extension.source_mode] : []}
      />

      {error && <div className="error">{error}</div>}

      {Component && page ? (
        <div className="card">
          <Component extension={extension} page={page} />
        </div>
      ) : !error ? (
        <p>Loading extension module…</p>
      ) : null}
    </div>
  );
}
