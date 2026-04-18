import type {
  ExtensionPageContribution,
  ExtensionPanelContribution,
} from "../../ui-sdk/src";

const API_BASE = import.meta.env.VITE_ENNOIA_API_URL ?? "http://127.0.0.1:3710";

// ========== Core types (existing) ==========

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
  kind: "Private" | "Space" | "private" | "space";
  owner: { kind: string; id: string };
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
  role: string;
  body: string;
  mentions: string[];
  created_at: string;
};

export type Run = {
  id: string;
  owner: { kind: string; id: string };
  thread_id: string;
  trigger: string;
  stage: string;
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
  owner: { kind: string; id: string };
  run_id: string;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type Memory = {
  id: string;
  owner: { kind: string; id: string };
  namespace: string;
  memory_kind: string;
  stability: string;
  status: string;
  title?: string | null;
  content: string;
  summary?: string | null;
  confidence: number;
  importance: number;
  sources: { kind: string; reference: string }[];
  tags: string[];
  entities: string[];
  created_at: string;
  updated_at: string;
};

export type Job = {
  id: string;
  owner_kind: string;
  owner_id: string;
  job_kind: string;
  schedule_kind: string;
  schedule_value: string;
  status: string;
  next_run_at?: string | null;
  created_at: string;
};

export type ExtensionRegistry = {
  extensions: Array<{ id: string; kind: string; version: string; install_dir: string }>;
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

// ========== Runtime audit ==========

export type RunStageEvent = {
  id: string;
  run_id: string;
  from_stage?: string | null;
  to_stage: string;
  policy_rule_id?: string | null;
  reason?: string | null;
  at: string;
};

export type DecisionSnapshot = {
  id: string;
  run_id?: string | null;
  task_id?: string | null;
  stage: string;
  signals_json: string;
  next_action: string;
  policy_rule_id: string;
  at: string;
};

export type GateRecord = {
  id: string;
  run_id?: string | null;
  task_id?: string | null;
  gate_name: string;
  verdict: string;
  reason?: string | null;
  details_json: string;
  at: string;
};

// ========== Memory requests/responses ==========

export type RememberReceipt = {
  receipt_id: string;
  memory_id: string;
  action: string;
  policy_rule_id?: string | null;
  created_at: string;
};

export type RecallResult = {
  memories: Memory[];
  receipt_id: string;
  mode: string;
  total_chars: number;
};

export type ReviewReceipt = {
  receipt_id: string;
  target_memory_id: string;
  action: string;
  old_status?: string | null;
  new_status: string;
  reviewer: string;
  created_at: string;
};

// ========== Auth ==========

export type AuthedUser = {
  id: string;
  username: string;
  role: string;
  auth_method: string;
};

export type User = {
  id: string;
  username: string;
  display_name?: string | null;
  email?: string | null;
  role: "user" | "admin";
  owner_kind?: string | null;
  owner_id?: string | null;
  created_at: string;
  updated_at: string;
  last_login_at?: string | null;
};

export type Session = {
  id: string;
  user_id: string;
  token_hash: string;
  created_at: string;
  expires_at: string;
  last_seen_at?: string | null;
  user_agent?: string | null;
  ip?: string | null;
};

export type ApiKey = {
  id: string;
  user_id: string;
  key_hash: string;
  label?: string | null;
  scopes: string[];
  created_at: string;
  expires_at?: string | null;
  last_used_at?: string | null;
};

export type LoginResponse = {
  user: User;
  token: string;
  token_kind: "session" | "jwt";
  expires_at: string;
};

export type BootstrapState = {
  completed: boolean;
  admin_created_at?: string | null;
};

export type BootstrapResponse = {
  user: User;
  bootstrap: BootstrapState;
  jwt_secret_generated: boolean;
};

// ========== System config ==========

export type ConfigEntry = {
  key: string;
  payload_json: string;
  enabled: boolean;
  version: number;
  updated_by?: string | null;
  updated_at: string;
};

export type ConfigChangeRecord = {
  id: string;
  config_key: string;
  old_payload_json?: string | null;
  new_payload_json: string;
  changed_by?: string | null;
  changed_at: string;
};

export type AuthConfig = {
  enabled: boolean;
  mode: "none" | "api_key" | "jwt" | "session";
  jwt_secret?: string | null;
  session_ttl_seconds: number;
  protected_paths: string[];
  public_paths: string[];
  allow_registration: boolean;
};

export type RateLimitConfig = {
  enabled: boolean;
  per_ip_rpm: number;
  per_user_rpm: number;
  burst: number;
  exempt_paths: string[];
};

export type CorsConfig = {
  enabled: boolean;
  origins: string[];
  methods: string[];
  credentials: boolean;
  max_age_seconds: number;
};

export type TimeoutConfig = {
  enabled: boolean;
  default_ms: number;
  per_path_ms: Record<string, number>;
};

export type LoggingConfig = {
  enabled: boolean;
  level: string;
  sample_rate: number;
  redact_headers: string[];
};

export type BodyLimitConfig = {
  enabled: boolean;
  max_bytes: number;
  per_path_max: Record<string, number>;
};

export type SystemConfig = {
  auth: AuthConfig;
  rate_limit: RateLimitConfig;
  cors: CorsConfig;
  timeout: TimeoutConfig;
  logging: LoggingConfig;
  body_limit: BodyLimitConfig;
  bootstrap: BootstrapState;
};

// ========== Snapshot ==========

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

// ========== Token management ==========

const TOKEN_KEY = "ennoia_auth_token";

export function getAuthToken(): string | null {
  try {
    return localStorage.getItem(TOKEN_KEY);
  } catch {
    return null;
  }
}

export function setAuthToken(token: string | null) {
  try {
    if (token === null) {
      localStorage.removeItem(TOKEN_KEY);
    } else {
      localStorage.setItem(TOKEN_KEY, token);
    }
  } catch {
    // localStorage unavailable; ignore
  }
}

// ========== Fetch wrapper ==========

export class ApiError extends Error {
  constructor(public status: number, message: string) {
    super(message);
  }
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const headers: Record<string, string> = {
    "content-type": "application/json",
    ...((init?.headers as Record<string, string>) ?? {}),
  };
  const token = getAuthToken();
  if (token) {
    headers["authorization"] = `Bearer ${token}`;
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers,
  });

  if (response.status === 401) {
    setAuthToken(null);
    throw new ApiError(401, "unauthorized");
  }
  if (!response.ok) {
    const body = await response.text().catch(() => "");
    throw new ApiError(response.status, body || `request failed: ${response.status}`);
  }
  if (response.status === 204) {
    return undefined as unknown as T;
  }
  return (await response.json()) as T;
}

// ========== Workspace ==========

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

  return { overview, agents, spaces, threads, runs, tasks, artifacts, memories, jobs, registry };
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
  job_kind: string;
  schedule_kind: string;
  schedule_value: string;
  payload?: unknown;
}) {
  return fetchJson<Job>("/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function loadThreadMessages(threadId: string) {
  return fetchJson<Message[]>(`/api/v1/threads/${threadId}/messages`);
}

// ========== Run detail ==========

export async function loadRunStages(runId: string) {
  return fetchJson<RunStageEvent[]>(`/api/v1/runs/${runId}/stages`);
}

export async function loadRunDecisions(runId: string) {
  return fetchJson<DecisionSnapshot[]>(`/api/v1/runs/${runId}/decisions`);
}

export async function loadRunGates(runId: string) {
  return fetchJson<GateRecord[]>(`/api/v1/runs/${runId}/gates`);
}

export async function loadRunTasks(runId: string) {
  return fetchJson<Task[]>(`/api/v1/runs/${runId}/tasks`);
}

// ========== Memory ==========

export async function listMemories() {
  return fetchJson<Memory[]>("/api/v1/memories");
}

export async function createMemory(payload: {
  owner_kind: string;
  owner_id: string;
  namespace: string;
  memory_kind: string;
  stability: string;
  title?: string;
  content: string;
  summary?: string;
  sources?: { kind: string; reference: string }[];
  tags?: string[];
  entities?: string[];
}) {
  return fetchJson<RememberReceipt>("/api/v1/memories", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function recallMemories(payload: {
  owner_kind: string;
  owner_id: string;
  query_text?: string;
  namespace_prefix?: string;
  mode?: "namespace" | "fts" | "hybrid";
  limit?: number;
}) {
  return fetchJson<RecallResult>("/api/v1/memories/recall", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function reviewMemory(payload: {
  target_memory_id: string;
  reviewer: string;
  action: "approve" | "reject" | "supersede" | "retire";
  notes?: string;
}) {
  return fetchJson<ReviewReceipt>("/api/v1/memories/review", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// ========== Auth ==========

export async function login(payload: { username: string; password: string }) {
  return fetchJson<LoginResponse>("/api/v1/auth/login", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function logout() {
  return fetchJson<void>("/api/v1/auth/logout", { method: "POST" });
}

export async function fetchMe() {
  return fetchJson<AuthedUser>("/api/v1/auth/me");
}

export async function registerUser(payload: {
  username: string;
  password: string;
  display_name?: string;
  email?: string;
}) {
  return fetchJson<{ user: User }>("/api/v1/auth/register", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// ========== Bootstrap ==========

export async function fetchBootstrapState() {
  return fetchJson<BootstrapState>("/api/v1/bootstrap/state");
}

export async function completeBootstrap(payload: {
  admin_username: string;
  admin_password: string;
  admin_display_name?: string;
  auth_mode?: "none" | "api_key" | "jwt" | "session";
  allow_registration?: boolean;
}) {
  return fetchJson<BootstrapResponse>("/api/v1/bootstrap", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

// ========== Config ==========

export async function listConfig() {
  return fetchJson<ConfigEntry[]>("/api/v1/admin/config");
}

export async function getConfig(key: string) {
  return fetchJson<ConfigEntry>(`/api/v1/admin/config/${key}`);
}

export async function putConfig(key: string, payload: unknown, updatedBy?: string) {
  return fetchJson<ConfigEntry>(`/api/v1/admin/config/${key}`, {
    method: "PUT",
    body: JSON.stringify({ payload, updated_by: updatedBy }),
  });
}

export async function getConfigHistory(key: string) {
  return fetchJson<ConfigChangeRecord[]>(`/api/v1/admin/config/${key}/history`);
}

export async function getConfigSnapshot() {
  return fetchJson<SystemConfig>("/api/v1/admin/config/snapshot");
}

// ========== Admin users ==========

export async function adminListUsers() {
  return fetchJson<User[]>("/api/v1/admin/users");
}

export async function adminCreateUser(payload: {
  username: string;
  password: string;
  display_name?: string;
  email?: string;
  role?: "user" | "admin";
}) {
  return fetchJson<User>("/api/v1/admin/users", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function adminUpdateUser(
  id: string,
  payload: { display_name?: string; email?: string; role?: "user" | "admin" },
) {
  return fetchJson<User>(`/api/v1/admin/users/${id}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function adminDeleteUser(id: string) {
  return fetchJson<void>(`/api/v1/admin/users/${id}`, { method: "DELETE" });
}

export async function adminResetPassword(id: string, newPassword: string) {
  return fetchJson<void>(`/api/v1/admin/users/${id}/reset-password`, {
    method: "POST",
    body: JSON.stringify({ new_password: newPassword }),
  });
}

// ========== Admin sessions ==========

export async function adminListSessions() {
  return fetchJson<Session[]>("/api/v1/admin/sessions");
}

export async function adminDeleteSession(id: string) {
  return fetchJson<void>(`/api/v1/admin/sessions/${id}`, { method: "DELETE" });
}

// ========== Admin API keys ==========

export async function adminListApiKeys() {
  return fetchJson<ApiKey[]>("/api/v1/admin/api-keys");
}

export async function adminCreateApiKey(payload: {
  user_id: string;
  label?: string;
  scopes?: string[];
  expires_at?: string;
}) {
  return fetchJson<{ key: ApiKey; raw_key: string }>("/api/v1/admin/api-keys", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function adminDeleteApiKey(id: string) {
  return fetchJson<void>(`/api/v1/admin/api-keys/${id}`, { method: "DELETE" });
}
