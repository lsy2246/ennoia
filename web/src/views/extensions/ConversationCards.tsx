import { useEffect, useMemo, useRef } from "react";
import { apiUrl } from "@ennoia/api-client";
import type { ExtensionSurfaceContribution } from "@ennoia/ui-sdk";

import { useUiHelpers, useUiStore } from "@/stores/ui";
import { loadExtensionConversationCardMount } from "@/views/extensions/registry";

type ConversationExtensionCardsProps = {
  conversationId: string;
};

function ConversationExtensionCardMount({
  conversationId,
  generation,
  surface,
}: {
  conversationId: string;
  generation: number;
  surface: ExtensionSurfaceContribution;
}) {
  const helpers = useUiHelpers();
  const themeId = useUiStore((state) => state.themeId);
  const { formatDate, formatDateTime, formatTime, locale, t } = helpers;
  const containerRef = useRef<HTMLDivElement | null>(null);

  useEffect(() => {
    let cancelled = false;
    let cleanup: (() => void | Promise<void>) | undefined;
    const container = containerRef.current;
    if (!container) {
      return () => {
        cancelled = true;
      };
    }

    container.replaceChildren();
    void loadExtensionConversationCardMount(surface, generation)
      .then(async (mount) => {
        if (cancelled) {
          return;
        }
        if (!mount) {
          return;
        }
        const handle = await mount(container, {
          kind: "conversation_card",
          extensionId: surface.extension_id,
          mount: surface.surface.mount,
          surface,
          conversationId,
          helpers: {
            locale,
            themeId,
            apiBaseUrl: apiUrl(""),
            t,
            formatDateTime,
            formatDate,
            formatTime,
          },
        });
        if (!cancelled) {
          cleanup = handle?.unmount;
        }
      })
      .catch(() => {});

    return () => {
      cancelled = true;
      void cleanup?.();
    };
  }, [conversationId, formatDate, formatDateTime, formatTime, generation, locale, surface, t, themeId]);

  return (
    <>
      <div
        ref={containerRef}
        className="session-extension-card"
        data-extension-conversation-card={surface.surface.mount}
      />
    </>
  );
}

export function ConversationExtensionCards({ conversationId }: ConversationExtensionCardsProps) {
  const { runtime } = useUiHelpers();
  const generation = runtime?.versions.registry ?? 0;
  const surfaces = useMemo(
    () =>
      [...(runtime?.registry.surfaces ?? [])]
        .filter((item) => item.surface.kind === "conversation_card")
        .sort((left, right) =>
          (right.surface.priority ?? 0) - (left.surface.priority ?? 0)
            || left.extension_id.localeCompare(right.extension_id)),
    [runtime?.registry.surfaces],
  );

  if (surfaces.length === 0) {
    return null;
  }

  return (
    <section className="session-extension-zone">
      {surfaces.map((surface) => (
        <ConversationExtensionCardMount
          key={surface.surface.id}
          conversationId={conversationId}
          generation={generation}
          surface={surface}
        />
      ))}
    </section>
  );
}
