import {
  Children,
  cloneElement,
  Fragment,
  isValidElement,
  type ReactElement,
  type ReactNode,
} from "react";

import type { AgentProfile, SkillConfig } from "@ennoia/api-client";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";

import type { ChatEntryFormat } from "./chat-types";

function buildSkillMap(skills: SkillConfig[]) {
  const skillMap = new Map<string, string>();
  for (const skill of skills) {
    skillMap.set(skill.id.toLowerCase(), skill.display_name);
    skillMap.set(skill.display_name.toLowerCase(), skill.display_name);
    skillMap.set(skill.display_name.toLowerCase().replace(/\s+/g, "-"), skill.display_name);
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

function decorateChildren(
  children: ReactNode,
  agents: AgentProfile[],
  skills: SkillConfig[],
  mentionAgentIds: string[],
): ReactNode {
  return Children.map(children, (child) => {
    if (typeof child === "string") {
      return renderInlineTokens(child, agents, skills, mentionAgentIds);
    }
    if (!isValidElement(child)) {
      return child;
    }

    const element = child as ReactElement<{ children?: ReactNode }>;
    if (element.type === "code" || element.type === "pre") {
      return element;
    }

    return cloneElement(element, {
      children: decorateChildren(element.props.children, agents, skills, mentionAgentIds),
    });
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
  const normalized = body.replace(/^```[\w-]*\n?/, "").replace(/\n?```$/, "");
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
  const normalized = body
    .replace(/^```(?:mermaid|diagram|flowchart)\n?/i, "")
    .replace(/\n?```$/, "");
  return (
    <div className="message-diagram">
      <div className="message-diagram__header">Mermaid</div>
      <pre className="message-pre">
        <code>{normalized}</code>
      </pre>
    </div>
  );
}

function MarkdownContent({ body, agents, skills, mentionAgentIds }: {
  body: string;
  agents: AgentProfile[];
  skills: SkillConfig[];
  mentionAgentIds: string[];
}) {
  return (
    <ReactMarkdown
      remarkPlugins={[remarkGfm]}
      components={{
        h1: ({ children }) => <h1 className="message-markdown__heading message-markdown__heading--1">{decorateChildren(children, agents, skills, mentionAgentIds)}</h1>,
        h2: ({ children }) => <h2 className="message-markdown__heading message-markdown__heading--2">{decorateChildren(children, agents, skills, mentionAgentIds)}</h2>,
        h3: ({ children }) => <h3 className="message-markdown__heading message-markdown__heading--3">{decorateChildren(children, agents, skills, mentionAgentIds)}</h3>,
        p: ({ children }) => <p className="message-markdown__paragraph">{decorateChildren(children, agents, skills, mentionAgentIds)}</p>,
        ul: ({ children }) => <ul className="message-markdown__list">{decorateChildren(children, agents, skills, mentionAgentIds)}</ul>,
        ol: ({ children }) => <ol className="message-markdown__list message-markdown__list--ordered">{decorateChildren(children, agents, skills, mentionAgentIds)}</ol>,
        li: ({ children }) => <li className="message-markdown__item">{decorateChildren(children, agents, skills, mentionAgentIds)}</li>,
        blockquote: ({ children }) => <blockquote className="message-markdown__quote">{decorateChildren(children, agents, skills, mentionAgentIds)}</blockquote>,
        table: ({ children }) => <div className="message-markdown__table-wrap"><table className="message-markdown__table">{decorateChildren(children, agents, skills, mentionAgentIds)}</table></div>,
        th: ({ children }) => <th>{decorateChildren(children, agents, skills, mentionAgentIds)}</th>,
        td: ({ children }) => <td>{decorateChildren(children, agents, skills, mentionAgentIds)}</td>,
        a: ({ children, href }) => (
          <a className="message-markdown__link" href={href} target="_blank" rel="noreferrer">
            {decorateChildren(children, agents, skills, mentionAgentIds)}
          </a>
        ),
        code: (props) => {
          const isInline = "inline" in props && props.inline === true;
          const className = "className" in props ? props.className : undefined;
          const children = "children" in props ? props.children : undefined;
          const raw = String(children).replace(/\n$/, "");
          if (isInline) {
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
        },
      }}
    >
      {body}
    </ReactMarkdown>
  );
}

export function ChatContent({ body, format, agents, skills, mentionAgentIds = [] }: {
  body: string;
  format: ChatEntryFormat;
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
  return <MarkdownContent body={body} agents={agents} skills={skills} mentionAgentIds={mentionAgentIds} />;
}
