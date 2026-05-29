import {
  apiUrl,
  type AgentProfile,
  type SkillConfig,
} from "@ennoia/api-client";
import {
  Fragment,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";

import type {
  ExtensionMessageRendererContribution,
  ExtensionMessageRendererMount,
  ExtensionViewHandle,
} from "@ennoia/ui-sdk";
import { useUiHelpers, useUiStore } from "@/stores/ui";
import { loadExtensionMessageRendererMount } from "@/views/extensions/registry";

import type { ChatEntryFormat } from "./chat-types";
import { normalizeCodeBlockText } from "./code-block-content";

function buildSkillMap(skills: SkillConfig[]) {
  const skillMap = new Map<string, string>();
  for (const skill of skills) {
    skillMap.set(skill.id.toLowerCase(), skill.id);
    skillMap.set(skill.id.toLowerCase().replace(/\s+/g, "-"), skill.id);
  }
  return skillMap;
}

function buildAllowedMentionMap(agents: AgentProfile[], mentionAgentIds: string[]) {
  const allowedIds = new Set(mentionAgentIds.map((item) => item.toLowerCase()));
  const mentionMap = new Map<string, string>();
  for (const agent of agents) {
    if (!allowedIds.has(agent.id.toLowerCase())) {
      continue;
    }
    mentionMap.set(agent.id.toLowerCase(), agent.display_name);
    mentionMap.set(agent.display_name.toLowerCase(), agent.display_name);
    mentionMap.set(agent.display_name.toLowerCase().replace(/\s+/g, "-"), agent.display_name);
  }
  return mentionMap;
}

function renderInlineTokens(
  text: string,
  agents: AgentProfile[],
  skills: SkillConfig[],
  mentionAgentIds: string[],
) {
  const mentionMap = mentionAgentIds.length > 0
    ? buildAllowedMentionMap(agents, mentionAgentIds)
    : new Map<string, string>();
  const skillMap = buildSkillMap(skills);
  const parts = text.split(/([@/][\p{L}\p{N}_.-]+)/gu);

  return parts.map((part, index) => {
    const mentionMatch = part.match(/^@([\p{L}\p{N}_.-]+)$/u);
    if (mentionMatch) {
      const label = mentionMap.get(mentionMatch[1].toLowerCase());
      if (label) {
        return (
          <span key={`mention:${index}`} className="message-inline-mention">
            @{label}
          </span>
        );
      }
    }

    const skillMatch = part.match(/^\/([\p{L}\p{N}_.-]+)$/u);
    if (skillMatch) {
      const label = skillMap.get(skillMatch[1].toLowerCase());
      if (label) {
        return (
          <span key={`skill:${index}`} className="message-inline-skill">
            /{label}
          </span>
        );
      }
    }

    return <Fragment key={`text:${index}`}>{part}</Fragment>;
  });
}

function PlainTextContent({ body, agents, skills, mentionAgentIds }: {
  body: string;
  agents: AgentProfile[];
  skills: SkillConfig[];
  mentionAgentIds: string[];
}) {
  const lines = body.split("\n");
  return (
    <div className="message-plain">
      {lines.map((line, index) => (
        <Fragment key={`line:${index}`}>
          {renderInlineTokens(line, agents, skills, mentionAgentIds)}
          {index < lines.length - 1 ? <br /> : null}
        </Fragment>
      ))}
    </div>
  );
}

function CodeContent({ body }: { body: string }) {
  const normalized = normalizeCodeBlockText(body);
  return (
    <pre className="message-pre">
      <code>{normalized}</code>
    </pre>
  );
}

function JsonContent({ body }: { body: string }) {
  try {
    const parsed = JSON.parse(body);
    return (
      <pre className="message-pre message-pre--json">
        <code>{JSON.stringify(parsed, null, 2)}</code>
      </pre>
    );
  } catch {
    return <CodeContent body={body} />;
  }
}

function DiagramContent({ body }: { body: string }) {
  const normalized = normalizeCodeBlockText(body);
  return (
    <div className="message-diagram">
      <div className="message-diagram__header">Mermaid</div>
      <pre className="message-pre">
        <code>{normalized}</code>
      </pre>
    </div>
  );
}

function isPromiseLike<T>(value: unknown): value is PromiseLike<T> {
  return Boolean(value && typeof (value as { then?: unknown }).then === "function");
}

function cleanupHandle(handle: void | ExtensionViewHandle | null | undefined) {
  if (handle?.unmount) {
    void handle.unmount();
  }
}

function callMessageRendererMount(
  mount: ExtensionMessageRendererMount,
  container: HTMLElement,
  context: Parameters<ExtensionMessageRendererMount>[1],
  onMounted: () => void,
) {
  const result = mount(container, context);
  if (isPromiseLike<void | ExtensionViewHandle>(result)) {
    return result.then((handle) => {
      onMounted();
      return handle;
    });
  }
  onMounted();
  return result;
}

function selectMessageRenderer(
  renderers: ExtensionMessageRendererContribution[],
  format: ChatEntryFormat,
) {
  const candidates = renderers.filter((item) => item.renderer.format === format);
  candidates.sort((left, right) =>
    right.renderer.priority - left.renderer.priority
    || left.extension_id.localeCompare(right.extension_id)
    || left.renderer.id.localeCompare(right.renderer.id)
  );
  return candidates[0] ?? null;
}

function ExtensionMessageContent({ body, format, role, agents, skills, mentionAgentIds }: {
  body: string;
  format: ChatEntryFormat;
  role: "operator" | "agent" | "system" | "tool";
  agents: AgentProfile[];
  skills: SkillConfig[];
  mentionAgentIds: string[];
}) {
  const runtime = useUiStore((state) => state.runtime);
  const themeId = useUiStore((state) => state.themeId);
  const { formatDate, formatDateTime, formatTime, locale, t } = useUiHelpers();
  const [renderState, setRenderState] = useState<"loading" | "mounted" | "fallback">("loading");
  const containerRef = useRef<HTMLDivElement | null>(null);
  const generation = runtime?.versions.registry ?? 0;
  const renderer = useMemo(
    () => selectMessageRenderer(runtime?.registry.message_renderers ?? [], format),
    [format, runtime?.registry.message_renderers],
  );

  useEffect(() => {
    const container = containerRef.current;
    if (!container || !renderer) {
      setRenderState("fallback");
      return undefined;
    }

    let disposed = false;
    let mountedHandle: ExtensionViewHandle | null | undefined;
    setRenderState("loading");
    container.replaceChildren();

    void loadExtensionMessageRendererMount(renderer, generation)
      .then((mount) => {
        if (disposed) {
          return null;
        }
        if (!mount) {
          setRenderState("fallback");
          return null;
        }
        return callMessageRendererMount(
          mount,
          container,
          {
            kind: "message_renderer",
            extensionId: renderer.extension_id,
            mount: renderer.renderer.mount,
            renderer,
            helpers: {
              locale,
              themeId,
              apiBaseUrl: apiUrl(""),
              t,
              formatDateTime,
              formatDate,
              formatTime,
            },
            request: {
              body,
              format,
              role,
              agents,
              skills,
              mentionAgentIds,
            },
          },
          () => {
            if (!disposed) {
              setRenderState("mounted");
            }
          },
        );
      })
      .then((handle) => {
        if (disposed) {
          cleanupHandle(handle);
          return;
        }
        mountedHandle = handle ?? null;
      })
      .catch(() => {
        if (!disposed) {
          setRenderState("fallback");
        }
      });

    return () => {
      disposed = true;
      cleanupHandle(mountedHandle);
      container.replaceChildren();
    };
  }, [
    agents,
    body,
    formatDate,
    formatDateTime,
    formatTime,
    format,
    generation,
    locale,
    mentionAgentIds,
    renderer,
    role,
    skills,
    t,
    themeId,
  ]);

  return (
    <>
      <div
        ref={containerRef}
        className="message-extension-renderer"
        hidden={renderState !== "mounted"}
      />
      {renderState === "fallback" ? (
        <PlainTextContent
          body={body}
          agents={agents}
          skills={skills}
          mentionAgentIds={mentionAgentIds}
        />
      ) : null}
    </>
  );
}

export function ChatContent({ body, format, role = "agent", agents, skills, mentionAgentIds = [] }: {
  body: string;
  format: ChatEntryFormat;
  role?: "operator" | "agent" | "system" | "tool";
  agents: AgentProfile[];
  skills: SkillConfig[];
  mentionAgentIds?: string[];
}) {
  if (format === "code") {
    return <CodeContent body={body} />;
  }
  if (format === "json") {
    return <JsonContent body={body} />;
  }
  if (format === "diagram") {
    return <DiagramContent body={body} />;
  }
  if (format === "plain") {
    return <PlainTextContent body={body} agents={agents} skills={skills} mentionAgentIds={mentionAgentIds} />;
  }
  return (
    <ExtensionMessageContent
      body={body}
      format={format}
      role={role}
      agents={agents}
      skills={skills}
      mentionAgentIds={mentionAgentIds}
    />
  );
}
