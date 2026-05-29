import {
  Children,
  cloneElement,
  createContext,
  Fragment,
  isValidElement,
  useContext,
  type ReactElement,
  type ReactNode,
} from "react";
import { createRoot, type Root } from "react-dom/client";
import ReactMarkdown, { defaultUrlTransform } from "react-markdown";
import remarkGfm from "remark-gfm";
import type {
  ExtensionMessageRenderRequest,
  ExtensionUiModule,
  ExtensionUiRenderHelpers,
} from "@ennoia/ui-sdk";
import { normalizeCodeBlockText } from "../../../../web/src/views/conversations/code-block-content";

const roots = new WeakMap<HTMLElement, Root>();
const MarkdownCodeBlockContext = createContext(false);
const CANONICAL_ARTIFACT_PATH = /^\/api\/agents\/[^/]+\/artifacts\/(.+)$/;
const PARSE_BASE_URL = "http://ennoia.local";

function renderIntoContainer(container: HTMLElement, node: React.ReactNode) {
  let root = roots.get(container);
  if (!root) {
    root = createRoot(container);
    roots.set(container, root);
  }
  root.render(<>{node}</>);
  return {
    unmount() {
      const current = roots.get(container);
      current?.unmount();
      roots.delete(container);
    },
  };
}

function extractNodeText(node: ReactNode): string {
  if (typeof node === "string" || typeof node === "number") {
    return String(node);
  }
  if (Array.isArray(node)) {
    return node.map((item) => extractNodeText(item)).join("");
  }
  if (!isValidElement(node)) {
    return "";
  }

  const element = node as ReactElement<{ children?: ReactNode }>;
  return extractNodeText(element.props.children);
}

function buildSkillMap(skills: ExtensionMessageRenderRequest["skills"]) {
  const skillMap = new Map<string, string>();
  for (const skill of skills) {
    skillMap.set(skill.id.toLowerCase(), skill.id);
    skillMap.set(skill.id.toLowerCase().replace(/\s+/g, "-"), skill.id);
  }
  return skillMap;
}

function buildAllowedMentionMap(
  agents: ExtensionMessageRenderRequest["agents"],
  mentionAgentIds: string[],
) {
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
  request: ExtensionMessageRenderRequest,
) {
  const mentionMap = request.mentionAgentIds.length > 0
    ? buildAllowedMentionMap(request.agents, request.mentionAgentIds)
    : new Map<string, string>();
  const skillMap = buildSkillMap(request.skills);
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

function decorateChildren(
  children: ReactNode,
  request: ExtensionMessageRenderRequest,
): ReactNode {
  return Children.map(children, (child) => {
    if (typeof child === "string") {
      return renderInlineTokens(child, request);
    }
    if (!isValidElement(child)) {
      return child;
    }

    const element = child as ReactElement<{ children?: ReactNode }>;
    if (element.type === "code" || element.type === "pre") {
      return element;
    }

    return cloneElement(element, {
      children: decorateChildren(element.props.children, request),
    });
  });
}

function PlainTextContent({ body, request }: {
  body: string;
  request: ExtensionMessageRenderRequest;
}) {
  const lines = body.split("\n");
  return (
    <div className="message-plain">
      {lines.map((line, index) => (
        <Fragment key={`line:${index}`}>
          {renderInlineTokens(line, request)}
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

function MarkdownPreNode({ children }: { children?: ReactNode }) {
  return (
    <MarkdownCodeBlockContext.Provider value={true}>
      {children}
    </MarkdownCodeBlockContext.Provider>
  );
}

function MarkdownCodeNode({
  className,
  children,
}: {
  className?: string;
  children?: ReactNode;
}) {
  const isBlock = useContext(MarkdownCodeBlockContext);
  const raw = isBlock
    ? normalizeCodeBlockText(extractNodeText(children))
    : extractNodeText(children).replace(/\n$/, "");
  if (!isBlock) {
    return <code className="message-code-inline">{raw}</code>;
  }

  const language = className?.replace("language-", "").toLowerCase() ?? "";
  if (["mermaid", "diagram", "flowchart"].includes(language)) {
    return <DiagramContent body={`\`\`\`${language}\n${raw}\n\`\`\``} />;
  }

  return (
    <pre className="message-pre">
      <code>{raw}</code>
    </pre>
  );
}

function normalizeMarkdownBody(body: string) {
  const normalized = body
    .replace(/\$\s*\\rightarrow\s*\$/g, "→")
    .replace(/\\rightarrow/g, "→")
    .replace(/\$\s*\\to\s*\$/g, "→")
    .replace(/\\to/g, "→")
    .replace(/\$\s*\\leftarrow\s*\$/g, "←")
    .replace(/\\leftarrow/g, "←")
    .replace(/\$\s*\\uparrow\s*\$/g, "↑")
    .replace(/\\uparrow/g, "↑")
    .replace(/\$\s*\\downarrow\s*\$/g, "↓")
    .replace(/\\downarrow/g, "↓")
    .replace(/\$([←→↑↓,\s()]+)\$/g, "$1");

  const fencedCount = (normalized.match(/```/g) ?? []).length;
  if (fencedCount % 2 === 1) {
    return {
      body: normalized,
      fallbackToPlain: true,
    };
  }

  const inlineTickCount = (normalized.match(/`/g) ?? []).length - fencedCount * 3;
  if (inlineTickCount % 2 === 1) {
    return {
      body: normalized,
      fallbackToPlain: true,
    };
  }

  return {
    body: normalized,
    fallbackToPlain: false,
  };
}

function parseCanonicalArtifactUrl(rawUrl: string | null | undefined) {
  const value = rawUrl?.trim() ?? "";
  if (!value.startsWith("/api/agents/")) {
    return undefined;
  }

  let parsed: URL;
  try {
    parsed = new URL(value, PARSE_BASE_URL);
  } catch {
    return undefined;
  }

  const match = parsed.pathname.match(CANONICAL_ARTIFACT_PATH);
  const artifactPath = match?.[1];
  if (!artifactPath) {
    return undefined;
  }

  return {
    artifactPath,
    path: `${parsed.pathname}${parsed.search}${parsed.hash}`,
  };
}

function getMessageAttachmentDownloadName(rawUrl: string | null | undefined) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }

  const filename = artifact.artifactPath.split("/").filter(Boolean).at(-1);
  if (!filename) {
    return "artifact";
  }

  try {
    return decodeURIComponent(filename);
  } catch {
    return filename;
  }
}

function absoluteApiUrl(helpers: ExtensionUiRenderHelpers, path: string) {
  return `${helpers.apiBaseUrl.replace(/\/$/, "")}${path}`;
}

function resolveMessageAttachmentUrl(
  rawUrl: string | null | undefined,
  helpers: ExtensionUiRenderHelpers,
) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }
  return absoluteApiUrl(helpers, artifact.path);
}

function resolveMessageAttachmentDownloadUrl(
  rawUrl: string | null | undefined,
  helpers: ExtensionUiRenderHelpers,
) {
  const artifact = parseCanonicalArtifactUrl(rawUrl);
  if (!artifact) {
    return undefined;
  }

  const url = new URL(artifact.path, PARSE_BASE_URL);
  url.searchParams.set("download", "1");
  return absoluteApiUrl(helpers, `${url.pathname}${url.search}${url.hash}`);
}

function createMessageAttachmentDownloadClickHandler(url: string, filename?: string) {
  return async (event: { preventDefault: () => void }) => {
    event.preventDefault();
    const response = await fetch(url);
    if (!response.ok) {
      throw new Error(`download failed: ${response.status}`);
    }
    const blob = await response.blob();
    const objectUrl = URL.createObjectURL(blob);
    const link = document.createElement("a");
    link.href = objectUrl;
    link.download = filename || "artifact";
    link.style.display = "none";
    document.body.appendChild(link);
    link.click();
    link.remove();
    URL.revokeObjectURL(objectUrl);
  };
}

function resolveMessageAttachmentLinkProps(
  rawUrl: string | null | undefined,
  helpers: ExtensionUiRenderHelpers,
) {
  const downloadName = getMessageAttachmentDownloadName(rawUrl);
  const downloadHref = resolveMessageAttachmentDownloadUrl(rawUrl, helpers);
  if (downloadHref) {
    return {
      download: downloadName,
      href: downloadHref,
      onClick: createMessageAttachmentDownloadClickHandler(downloadHref, downloadName),
    };
  }

  return {
    href: rawUrl ?? undefined,
    rel: "noreferrer",
    target: "_blank",
  };
}

function transformMarkdownUrl(url: string) {
  return defaultUrlTransform(url);
}

function MarkdownContent({ request, helpers }: {
  request: ExtensionMessageRenderRequest;
  helpers: ExtensionUiRenderHelpers;
}) {
  const normalized = normalizeMarkdownBody(request.body);
  if (normalized.fallbackToPlain) {
    return <PlainTextContent body={normalized.body} request={{ ...request, body: normalized.body }} />;
  }

  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      urlTransform={transformMarkdownUrl}
      components={{
        h1: ({ children }) => <h1 className="message-markdown__heading message-markdown__heading--1">{decorateChildren(children, request)}</h1>,
        h2: ({ children }) => <h2 className="message-markdown__heading message-markdown__heading--2">{decorateChildren(children, request)}</h2>,
        h3: ({ children }) => <h3 className="message-markdown__heading message-markdown__heading--3">{decorateChildren(children, request)}</h3>,
        p: ({ children }) => <p className="message-markdown__paragraph">{decorateChildren(children, request)}</p>,
        ul: ({ children }) => <ul className="message-markdown__list">{decorateChildren(children, request)}</ul>,
        ol: ({ children }) => <ol className="message-markdown__list message-markdown__list--ordered">{decorateChildren(children, request)}</ol>,
        li: ({ children }) => <li className="message-markdown__item">{decorateChildren(children, request)}</li>,
        blockquote: ({ children }) => <blockquote className="message-markdown__quote">{decorateChildren(children, request)}</blockquote>,
        table: ({ children }) => <div className="message-markdown__table-wrap"><table className="message-markdown__table">{decorateChildren(children, request)}</table></div>,
        th: ({ children }) => <th>{decorateChildren(children, request)}</th>,
        td: ({ children }) => <td>{decorateChildren(children, request)}</td>,
        pre: ({ children }) => <MarkdownPreNode>{children}</MarkdownPreNode>,
        a: ({ children, href }) => {
          const linkProps = resolveMessageAttachmentLinkProps(href, helpers);
          return (
            <a
              className="message-markdown__link"
              {...linkProps}
            >
              {decorateChildren(children, request)}
            </a>
          );
        },
        img: ({ alt, src, title }) => (
          <img
            className="message-markdown__image"
            src={resolveMessageAttachmentUrl(src, helpers) ?? src}
            alt={alt ?? ""}
            title={title}
            loading="lazy"
          />
        ),
        code: (props) => (
          <MarkdownCodeNode
            className={"className" in props ? props.className : undefined}
            children={"children" in props ? props.children : undefined}
          />
        ),
      }}
    >
      {normalized.body}
    </ReactMarkdown>
  );
}

const extensionUi: ExtensionUiModule = {
  messageRenderers: {
    "markdown-renderer.markdown": (container, context) =>
      renderIntoContainer(
        container,
        <MarkdownContent request={context.request} helpers={context.helpers} />,
      ),
  },
};

export default extensionUi;
