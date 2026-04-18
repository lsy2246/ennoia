import type {
  Agent,
  Artifact,
  Message,
  Run,
  Space,
  Task,
  Thread,
  WorkspaceSnapshot,
} from "./api";
import type {
  ExtensionPageContribution,
  ExtensionPageDescriptor,
  ExtensionPanelDescriptor,
  PanelSlot,
} from "../../ui-sdk/src";

export type ViewMode = "private" | "space" | "extensions";

export const emptySnapshot: WorkspaceSnapshot = {
  overview: {
    app_name: "Ennoia",
    shell_title: "Ennoia",
    default_theme: "system",
    modules: [],
    counts: {},
  },
  agents: [],
  spaces: [],
  threads: [],
  runs: [],
  tasks: [],
  artifacts: [],
  memories: [],
  jobs: [],
  registry: {
    extensions: [],
    pages: [],
    panels: [],
  },
};

export function pickThreadId(
  threads: Thread[],
  currentThreadId: string,
  activeView: ViewMode,
) {
  if (threads.some((thread) => thread.id === currentThreadId)) {
    return currentThreadId;
  }

  if (activeView === "space") {
    return threads.find((thread) => thread.kind === "Space")?.id ?? threads[0]?.id ?? "";
  }

  return threads.find((thread) => thread.kind === "Private")?.id ?? threads[0]?.id ?? "";
}

export function fallbackPageDescriptor(
  page: ExtensionPageContribution | undefined,
): ExtensionPageDescriptor {
  if (!page) {
    return {
      mount: "unknown",
      eyebrow: "Extension",
      summary: "等待 extension page 挂载。",
      highlights: ["registry-driven", "page mount", "shell surface"],
    };
  }

  return {
    mount: page.page.mount,
    eyebrow: page.extension_id,
    summary: `${page.page.title} 正在通过 registry page mount 接入主壳。`,
    highlights: [page.page.mount, page.page.route, page.extension_version],
  };
}

export function fallbackPanelDescriptor(
  mount: string,
  slot: string,
): ExtensionPanelDescriptor {
  return {
    mount,
    summary: `${mount} 正在通过 ${slot} slot 接入主壳面板容器。`,
    slot: slot as PanelSlot,
    metricLabel: "Mounted items",
  };
}

export function panelMetric(
  mount: string,
  messages: Message[],
  runs: Run[],
  tasks: Task[],
  artifacts: Artifact[],
) {
  if (mount === "observatory.timeline.panel") {
    return messages.length + runs.length + tasks.length + artifacts.length;
  }

  if (mount === "github.activity.panel") {
    return tasks.length;
  }

  return runs.length;
}

export function buildPrivateDraft(agent: Agent | undefined) {
  const agentLabel = agent?.display_name ?? "当前 Agent";
  return {
    goal: `推进 ${agentLabel} 当前任务`,
    body: `请基于当前线程上下文，为 ${agentLabel} 输出本轮执行计划、关键风险和交付节奏。`,
  };
}

export function buildSpaceDraft(space: Space | undefined, addressedAgents: Agent[]) {
  const spaceLabel = space?.display_name ?? "当前 Space";
  const agentLabel =
    addressedAgents.length > 0
      ? addressedAgents.map((agent) => agent.display_name).join("、")
      : "相关 Agent";

  return {
    goal: `协同推进 ${spaceLabel}`,
    body: `请 ${agentLabel} 基于 ${spaceLabel} 当前上下文协同拆解下一阶段任务，并同步依赖、风险和交付节奏。`,
  };
}

export function buildJobDescription(space: Space | undefined) {
  const spaceLabel = space?.display_name ?? "workspace";
  return `${spaceLabel} review`;
}

export function reconcileSelectedSpaceAgents(
  space: Space | undefined,
  agents: Agent[],
  current: string[],
) {
  const knownAgentIds = new Set(agents.map((agent) => agent.id));
  const preferred = (space?.default_agents ?? []).filter((agentId) =>
    knownAgentIds.has(agentId),
  );
  const selected = current.filter((agentId) => knownAgentIds.has(agentId));

  if (selected.length > 0) {
    return selected;
  }

  if (preferred.length > 0) {
    return preferred;
  }

  return agents.slice(0, 2).map((agent) => agent.id);
}
