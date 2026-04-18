import type {
  ExtensionPageContribution,
  ExtensionPanelContribution,
} from "../../ui-sdk/src";

const API_BASE = import.meta.env.VITE_ENNOIA_API_URL ?? "http://127.0.0.1:3710";

export type Overview = {
  app_name: string;
  shell_title: string;
  default_theme: string;
  modules: string[];
  counts: Record<string, number>;
};

export type Agent = {
  id: string;
  display_name: string;
  default_model: string;
};

export type Space = {
  id: string;
  display_name: string;
  default_agents: string[];
};

export type Thread = {
  id: string;
  kind: "Private" | "Space";
  owner: {
    kind: "Global" | "Agent" | "Space";
    id: string;
  };
  space_id?: string | null;
  title: string;
  participants: string[];
  created_at: string;
  updated_at: string;
};

export type Message = {
  id: string;
  thread_id: string;
  sender: string;
  role: "User" | "Agent" | "System";
  body: string;
  mentions: string[];
  created_at: string;
};

export type Run = {
  id: string;
  owner: {
    kind: string;
    id: string;
  };
  thread_id: string;
  trigger: string;
  status: string;
  goal: string;
  created_at: string;
  updated_at: string;
};

export type Task = {
  id: string;
  run_id: string;
  task_kind: string;
  title: string;
  assigned_agent_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type Artifact = {
  id: string;
  owner: {
    kind: string;
    id: string;
  };
  run_id: string;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type Memory = {
  id: string;
  summary: string;
  source: string;
  thread_id?: string | null;
  run_id?: string | null;
  created_at: string;
  owner: {
    kind: "Global" | "Agent" | "Space";
    id: string;
  };
};

export type Job = {
  id: string;
  owner_kind: string;
  owner_id: string;
  schedule_kind: string;
  schedule_value: string;
  description: string;
  status: string;
};

export type ExtensionRegistry = {
  extensions: Array<{
    id: string;
    kind: string;
    version: string;
    install_dir: string;
  }>;
  pages: ExtensionPageContribution[];
  panels: ExtensionPanelContribution[];
};

export type ConversationEnvelope = {
  thread: Thread;
  message: Message;
  run: Run;
  tasks: Task[];
  artifacts: Artifact[];
};

export type WorkspaceSnapshot = {
  overview: Overview;
  agents: Agent[];
  spaces: Space[];
  threads: Thread[];
  runs: Run[];
  tasks: Task[];
  artifacts: Artifact[];
  memories: Memory[];
  jobs: Job[];
  registry: ExtensionRegistry;
};

export async function loadWorkspaceSnapshot(): Promise<WorkspaceSnapshot> {
  const [
    overview,
    agents,
    spaces,
    threads,
    runs,
    tasks,
    artifacts,
    memories,
    jobs,
    registry,
  ] = await Promise.all([
    fetchJson<Overview>("/api/v1/overview"),
    fetchJson<Agent[]>("/api/v1/agents"),
    fetchJson<Space[]>("/api/v1/spaces"),
    fetchJson<Thread[]>("/api/v1/threads"),
    fetchJson<Run[]>("/api/v1/runs"),
    fetchJson<Task[]>("/api/v1/tasks"),
    fetchJson<Artifact[]>("/api/v1/artifacts"),
    fetchJson<Memory[]>("/api/v1/memories"),
    fetchJson<Job[]>("/api/v1/jobs"),
    fetchJson<ExtensionRegistry>("/api/v1/extensions/registry"),
  ]);

  return {
    overview,
    agents,
    spaces,
    threads,
    runs,
    tasks,
    artifacts,
    memories,
    jobs,
    registry,
  };
}

export async function sendPrivateMessage(payload: {
  agent_id: string;
  body: string;
  goal: string;
}) {
  return fetchJson<ConversationEnvelope>("/api/v1/threads/private/messages", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function sendSpaceMessage(payload: {
  space_id: string;
  addressed_agents: string[];
  body: string;
  goal: string;
}) {
  return fetchJson<ConversationEnvelope>("/api/v1/threads/space/messages", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function createJob(payload: {
  owner_kind: string;
  owner_id: string;
  schedule_kind: string;
  schedule_value: string;
  description: string;
}) {
  return fetchJson<Job>("/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function loadThreadMessages(threadId: string) {
  return fetchJson<Message[]>(`/api/v1/threads/${threadId}/messages`);
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const response = await fetch(`${API_BASE}${path}`, {
    headers: {
      "content-type": "application/json",
    },
    ...init,
  });

  if (!response.ok) {
    throw new Error(`request failed: ${response.status}`);
  }

  return (await response.json()) as T;
}
