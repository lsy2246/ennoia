import type {
  ExtensionDiagnostic,
  ExtensionLocaleContribution,
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionProviderContribution,
  ExtensionThemeContribution,
  LocalizedText,
} from "@ennoia/ui-sdk";
import type { ApiErrorBody } from "@ennoia/contract";
import { createLogger } from "@ennoia/observability";

const API_BASE = import.meta.env.VITE_ENNOIA_API_URL ?? "http://127.0.0.1:3710";
const logger = createLogger("api-client");

export function getApiBaseUrl() {
  return API_BASE;
}

export type BootstrapState = {
  is_initialized: boolean;
  initialized_at?: string | null;
};

export type UiPreference = {
  locale?: string | null;
  theme_id?: string | null;
  time_zone?: string | null;
  date_style?: string | null;
  density?: string | null;
  motion?: string | null;
  version: number;
  updated_at: string;
};

export type UiPreferenceRecord = {
  subject_id: string;
  preference: UiPreference;
};

export type UiConfig = {
  web_title: LocalizedText;
  default_theme: string;
  default_locale: string;
  fallback_locale: string;
  available_locales: string[];
  dock_persistence: boolean;
  default_page: string;
  show_command_palette: boolean;
};

export type UiRuntime = {
  ui_config: UiConfig;
  registry: {
    pages: ExtensionPageContribution[];
    panels: ExtensionPanelContribution[];
    themes: ExtensionThemeContribution[];
    locales: ExtensionLocaleContribution[];
    providers: ExtensionProviderContribution[];
  };
  instance_preference?: UiPreferenceRecord | null;
  space_preferences: UiPreferenceRecord[];
  versions: {
    registry: number;
    preferences: number;
  };
};

export type UiMessageBundle = {
  locale: string;
  resolved_locale: string;
  namespace: string;
  messages: Record<string, string>;
  source: string;
  version: string;
};

export type UiMessagesResponse = {
  locale: string;
  fallback_locale: string;
  bundles: UiMessageBundle[];
};

export type WorkspaceProfile = {
  id: string;
  display_name: string;
  locale: string;
  time_zone: string;
  default_space_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type AppConfig = {
  app_name: string;
  mode: string;
  workspace_root: string;
  database_mode: string;
  database_url: string;
  extensions_scan_dir: string;
  agents_scan_dir: string;
  scheduler_tick_ms: number;
  default_mention_mode: string;
};

export type BootstrapSetupResponse = {
  bootstrap: BootstrapState;
  profile: WorkspaceProfile;
  preference: UiPreferenceRecord;
};

export type AgentProfile = {
  id: string;
  display_name: string;
  description: string;
  system_prompt: string;
  provider_id: string;
  model_id: string;
  reasoning_effort: string;
  workspace_root: string;
  skills: string[];
  enabled: boolean;
  kind?: string;
  workspace_mode?: string;
  default_model?: string;
  skills_dir?: string;
  workspace_dir?: string;
  artifacts_dir?: string;
};

export type SkillConfig = {
  id: string;
  display_name: string;
  description: string;
  source: string;
  entry: string;
  tags: string[];
  enabled: boolean;
};

export type ProviderConfig = {
  id: string;
  display_name: string;
  kind: string;
  description: string;
  base_url: string;
  api_key_env: string;
  default_model: string;
  available_models: string[];
  enabled: boolean;
};

export type MemorySource = {
  kind: string;
  reference: string;
};

export type MemoryRecord = {
  id: string;
  owner: { kind: string; id: string };
  namespace: string;
  memory_kind: string;
  stability: string;
  status: string;
  superseded_by?: string | null;
  title?: string | null;
  content: string;
  summary?: string | null;
  confidence: number;
  importance: number;
  valid_from?: string | null;
  valid_to?: string | null;
  sources: MemorySource[];
  tags: string[];
  entities: string[];
  created_at: string;
  updated_at: string;
};

export type RecallResult = {
  memories: MemoryRecord[];
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

export type ChatThread = {
  id: string;
  topology: "direct" | "group";
  owner: { kind: string; id: string };
  space_id?: string | null;
  title: string;
  participants: string[];
  default_lane_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ChatLane = {
  id: string;
  conversation_id: string;
  space_id?: string | null;
  name: string;
  lane_type: string;
  status: string;
  goal: string;
  participants: string[];
  created_at: string;
  updated_at: string;
};

export type ChatMessage = {
  id: string;
  conversation_id: string;
  lane_id?: string | null;
  sender: string;
  role: "operator" | "agent" | "system" | "tool";
  body: string;
  mentions: string[];
  created_at: string;
};

export type ExecutionRun = {
  id: string;
  owner: { kind: string; id: string };
  conversation_id: string;
  lane_id?: string | null;
  trigger: string;
  stage: string;
  goal: string;
  created_at: string;
  updated_at: string;
};

export type ExecutionStep = {
  id: string;
  run_id: string;
  conversation_id: string;
  lane_id?: string | null;
  task_kind: string;
  title: string;
  assigned_agent_id: string;
  status: string;
  created_at: string;
  updated_at: string;
};

export type RunOutput = {
  id: string;
  owner: { kind: string; id: string };
  run_id: string;
  conversation_id?: string | null;
  lane_id?: string | null;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type ChatThreadDetail = {
  thread: ChatThread;
  lanes: ChatLane[];
  messages: ChatMessage[];
  runs: ExecutionRun[];
  tasks: ExecutionStep[];
  outputs: RunOutput[];
};

export type ChatSendResponse = {
  conversation: ChatThread;
  lane: ChatLane;
  message: ChatMessage;
  run: ExecutionRun;
  tasks: ExecutionStep[];
  artifacts: RunOutput[];
};

export type TaskJob = {
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

export type TaskJobDetail = {
  id: string;
  owner_kind: string;
  owner_id: string;
  job_kind: string;
  schedule_kind: string;
  schedule_value: string;
  payload_json: string;
  status: string;
  retry_count: number;
  max_retries: number;
  last_run_at?: string | null;
  next_run_at?: string | null;
  error?: string | null;
  created_at: string;
  updated_at: string;
};

export type ExtensionRuntimeState = {
  id: string;
  name: string;
  enabled: boolean;
  status: string;
  version: string;
  kind: string;
  source_mode: string;
  install_dir: string;
  source_root: string;
  diagnostics: ExtensionDiagnostic[];
};

export type ExtensionRuntimeEvent = {
  event_id: string;
  extension_id?: string | null;
  generation: number;
  event: string;
  health?: string | null;
  summary: string;
  diagnostics: ExtensionDiagnostic[];
  occurred_at: string;
};

export type ExtensionDetail = {
  id: string;
  name: string;
  kind: string;
  version: string;
  source_mode: string;
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  diagnostics: ExtensionDiagnostic[];
  frontend?: {
    kind: string;
    entry: string;
    hmr: boolean;
  } | null;
  backend?: {
    kind: string;
    runtime: string;
    entry: string;
    command?: string | null;
    healthcheck?: string | null;
    status: string;
    pid?: number | null;
  } | null;
};

export type SystemLog = {
  id: string;
  kind: string;
  source: string;
  level: string;
  title: string;
  summary: string;
  details?: string | null;
  run_id?: string | null;
  task_id?: string | null;
  at: string;
};

export type RuntimeConfigEntry = {
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

export type SystemConfig = {
  rate_limit: {
    enabled: boolean;
    per_ip_rpm: number;
    per_user_rpm: number;
    burst: number;
    exempt_paths: string[];
  };
  cors: {
    enabled: boolean;
    origins: string[];
    methods: string[];
    credentials: boolean;
    max_age_seconds: number;
  };
  timeout: {
    enabled: boolean;
    default_ms: number;
    per_path_ms: Record<string, number>;
  };
  logging: {
    enabled: boolean;
    level: string;
    sample_rate: number;
    redact_headers: string[];
  };
  body_limit: {
    enabled: boolean;
    max_bytes: number;
    per_path_max: Record<string, number>;
  };
  bootstrap: BootstrapState;
};

export class ApiError extends Error {
  constructor(
    public status: number,
    public code: ApiErrorBody["code"],
    message: string,
    public requestId?: string | null,
  ) {
    super(message);
  }
}

async function fetchJson<T>(path: string, init?: RequestInit): Promise<T> {
  const headers = new Headers(init?.headers);
  const method = (init?.method ?? "GET").toUpperCase();
  if (shouldAttachJsonContentType(method, init?.body, headers)) {
    headers.set("content-type", "application/json");
  }

  const response = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers,
  });

  if (!response.ok) {
    const body = await response.text().catch(() => "");
    let parsed: ApiErrorBody | null = null;
    try {
      parsed = JSON.parse(body) as ApiErrorBody;
    } catch {
      parsed = null;
    }
    if (parsed) {
      logger.warn("request failed", {
        path,
        status: response.status,
        code: parsed.code,
        request_id: parsed.request_id,
      });
      throw new ApiError(
        response.status,
        parsed.code,
        parsed.message || `request failed: ${response.status}`,
        parsed.request_id,
      );
    }
    throw new ApiError(response.status, "INTERNAL", body || `request failed: ${response.status}`);
  }

  if (response.status === 204) {
    return undefined as T;
  }

  return (await response.json()) as T;
}

function shouldAttachJsonContentType(
  method: string,
  body: RequestInit["body"],
  headers: Headers,
) {
  if (headers.has("content-type")) {
    return false;
  }
  if (method === "GET" || method === "HEAD" || body == null) {
    return false;
  }
  if (typeof FormData !== "undefined" && body instanceof FormData) {
    return false;
  }
  if (typeof URLSearchParams !== "undefined" && body instanceof URLSearchParams) {
    return false;
  }
  if (typeof Blob !== "undefined" && body instanceof Blob) {
    return false;
  }
  if (body instanceof ArrayBuffer || ArrayBuffer.isView(body)) {
    return false;
  }
  return true;
}

function toQueryString(input: Record<string, string | number | null | undefined>) {
  const params = new URLSearchParams();
  for (const [key, value] of Object.entries(input)) {
    if (value === undefined || value === null || value === "") {
      continue;
    }
    params.set(key, String(value));
  }
  const qs = params.toString();
  return qs ? `?${qs}` : "";
}

export async function fetchBootstrapStatus() {
  return fetchJson<BootstrapState>("/api/v1/bootstrap/status");
}

export async function bootstrapSetup(payload: {
  display_name?: string;
  locale?: string;
  time_zone?: string;
  default_space_id?: string;
  theme_id?: string;
  date_style?: string;
  density?: string;
  motion?: string;
}) {
  return fetchJson<BootstrapSetupResponse>("/api/v1/bootstrap/setup", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function fetchUiRuntime() {
  return fetchJson<UiRuntime>("/api/v1/ui/runtime");
}

export async function fetchUiMessages(locale: string, namespaces: string[] = []) {
  const params = new URLSearchParams({ locale });
  if (namespaces.length > 0) {
    params.set("namespaces", namespaces.join(","));
  }
  return fetchJson<UiMessagesResponse>(`/api/v1/ui/messages?${params.toString()}`);
}

export async function fetchRuntimeProfile() {
  return fetchJson<WorkspaceProfile | null>("/api/v1/runtime/profile");
}

export async function saveRuntimeProfile(payload: {
  display_name?: string | null;
  locale?: string | null;
  time_zone?: string | null;
  default_space_id?: string | null;
}) {
  return fetchJson<WorkspaceProfile>("/api/v1/runtime/profile", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchRuntimePreferences() {
  return fetchJson<UiPreferenceRecord | null>("/api/v1/runtime/preferences");
}

export async function saveInstanceUiPreferences(payload: {
  locale?: string | null;
  theme_id?: string | null;
  time_zone?: string | null;
  date_style?: string | null;
  density?: string | null;
  motion?: string | null;
}) {
  return fetchJson<UiPreferenceRecord>("/api/v1/runtime/preferences", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function fetchAppConfig() {
  return fetchJson<AppConfig>("/api/v1/runtime/app-config");
}

export async function saveAppConfig(payload: AppConfig) {
  return fetchJson<AppConfig>("/api/v1/runtime/app-config", {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function listAgents() {
  return fetchJson<AgentProfile[]>("/api/v1/agents");
}

export async function getAgent(agentId: string) {
  return fetchJson<AgentProfile>(`/api/v1/agents/${agentId}`);
}

export async function createAgent(payload: AgentProfile) {
  return fetchJson<AgentProfile>("/api/v1/agents", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateAgent(agentId: string, payload: AgentProfile) {
  return fetchJson<AgentProfile>(`/api/v1/agents/${agentId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteAgent(agentId: string) {
  return fetchJson<void>(`/api/v1/agents/${agentId}`, { method: "DELETE" });
}

export async function listSkills() {
  return fetchJson<SkillConfig[]>("/api/v1/skills");
}

export async function createSkill(payload: SkillConfig) {
  return fetchJson<SkillConfig>("/api/v1/skills", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateSkill(skillId: string, payload: SkillConfig) {
  return fetchJson<SkillConfig>(`/api/v1/skills/${skillId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteSkill(skillId: string) {
  return fetchJson<void>(`/api/v1/skills/${skillId}`, { method: "DELETE" });
}

export async function listProviders() {
  return fetchJson<ProviderConfig[]>("/api/v1/providers");
}

export async function createProvider(payload: ProviderConfig) {
  return fetchJson<ProviderConfig>("/api/v1/providers", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateProvider(providerId: string, payload: ProviderConfig) {
  return fetchJson<ProviderConfig>(`/api/v1/providers/${providerId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteProvider(providerId: string) {
  return fetchJson<void>(`/api/v1/providers/${providerId}`, { method: "DELETE" });
}

export async function listChats() {
  return fetchJson<ChatThread[]>("/api/v1/conversations");
}

export async function createChat(payload: {
  topology: "direct" | "group";
  title?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<{ conversation: ChatThread; default_lane: ChatLane }>("/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteChat(chatId: string) {
  return fetchJson<void>(`/api/v1/conversations/${chatId}`, { method: "DELETE" });
}

export async function getChat(chatId: string): Promise<ChatThreadDetail> {
  const [detail, messages, runs] = await Promise.all([
    fetchJson<{ conversation: ChatThread; lanes: ChatLane[] }>(`/api/v1/conversations/${chatId}`),
    fetchJson<ChatMessage[]>(`/api/v1/conversations/${chatId}/messages`),
    fetchJson<ExecutionRun[]>(`/api/v1/conversations/${chatId}/runs`),
  ]);
  const taskBuckets = await Promise.all(
    runs.map((run) => fetchJson<ExecutionStep[]>(`/api/v1/runs/${run.id}/tasks`)),
  );
  const outputBuckets = await Promise.all(
    runs.map((run) => fetchJson<RunOutput[]>(`/api/v1/runs/${run.id}/artifacts`)),
  );
  return {
    thread: detail.conversation,
    lanes: detail.lanes,
    messages,
    runs,
    tasks: taskBuckets.flat(),
    outputs: outputBuckets.flat(),
  };
}

export async function sendChatMessage(
  chatId: string,
  payload: {
    lane_id?: string;
    body: string;
    goal?: string;
    addressed_agents?: string[];
  },
) {
  return fetchJson<ChatSendResponse>(`/api/v1/conversations/${chatId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listTaskJobs() {
  return fetchJson<TaskJob[]>("/api/v1/jobs");
}

export async function listRuns() {
  return fetchJson<ExecutionRun[]>("/api/v1/runs");
}

export async function getTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}`);
}

export async function createTaskJob(payload: {
  owner_kind: string;
  owner_id: string;
  job_kind?: string;
  schedule_kind: string;
  schedule_value: string;
  payload?: unknown;
  max_retries?: number;
  run_at?: string;
}) {
  return fetchJson<TaskJobDetail>("/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function updateTaskJob(
  jobId: string,
  payload: {
    job_kind?: string;
    schedule_kind?: string;
    schedule_value?: string;
    payload?: unknown;
    max_retries?: number;
    run_at?: string;
  },
) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function deleteTaskJob(jobId: string) {
  return fetchJson<void>(`/api/v1/jobs/${jobId}`, { method: "DELETE" });
}

export async function runTaskJobNow(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/run`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function enableTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/enable`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function disableTaskJob(jobId: string) {
  return fetchJson<TaskJobDetail>(`/api/v1/jobs/${jobId}/disable`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function listExtensions() {
  return fetchJson<ExtensionRuntimeState[]>("/api/v1/extensions");
}

export async function getExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}`);
}

export async function getExtensionDiagnostics(extensionId: string) {
  return fetchJson<ExtensionDiagnostic[]>(`/api/v1/extensions/${extensionId}/diagnostics`);
}

export async function setExtensionEnabled(extensionId: string, enabled: boolean) {
  return fetchJson<ExtensionRuntimeState>(`/api/v1/extensions/${extensionId}/enabled`, {
    method: "PUT",
    body: JSON.stringify({ enabled }),
  });
}

export async function reloadExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}/reload`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function restartExtension(extensionId: string) {
  return fetchJson<ExtensionDetail>(`/api/v1/extensions/${extensionId}/restart`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function getExtensionLogs(extensionId: string) {
  const response = await fetch(`${API_BASE}/api/v1/extensions/${extensionId}/logs`);
  return response.text();
}

export async function listExtensionEvents(limit = 100) {
  return fetchJson<ExtensionRuntimeEvent[]>(
    `/api/v1/extensions/events${toQueryString({ limit })}`,
  );
}

export function getExtensionFrontendModuleUrl(extensionId: string) {
  return `${API_BASE}/api/v1/extensions/${extensionId}/frontend/module`;
}

export function getExtensionThemeStylesheetUrl(extensionId: string, themeId: string) {
  return `${API_BASE}/api/v1/extensions/${extensionId}/themes/${encodeURIComponent(themeId)}/stylesheet`;
}

export async function listLogs(
  limit = 100,
  filters?: {
    q?: string;
    level?: string;
    source?: string;
  },
) {
  return fetchJson<SystemLog[]>(
    `/api/v1/logs${toQueryString({
      limit,
      q: filters?.q,
      level: filters?.level,
      source: filters?.source,
    })}`,
  );
}

export async function reportFrontendLog(payload: {
  level: string;
  title: string;
  summary: string;
  source?: string;
  details?: string;
  at?: string;
}) {
  return fetchJson<void>("/api/v1/logs/frontend", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listMemories() {
  return fetchJson<MemoryRecord[]>("/api/v1/memories");
}

export async function recallMemories(payload: {
  owner_kind: string;
  owner_id: string;
  query_text?: string;
  namespace_prefix?: string;
  memory_kind?: string;
  mode?: string;
  limit?: number;
  thread_id?: string;
  run_id?: string;
}) {
  return fetchJson<RecallResult>("/api/v1/memories/recall", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function reviewMemory(payload: {
  target_memory_id: string;
  reviewer: string;
  action: string;
  notes?: string;
}) {
  return fetchJson<ReviewReceipt>("/api/v1/memories/review", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listConfig() {
  return fetchJson<RuntimeConfigEntry[]>("/api/v1/runtime/config");
}

export async function getConfig(key: string) {
  return fetchJson<RuntimeConfigEntry>(`/api/v1/runtime/config/${key}`);
}

export async function putConfig(key: string, payload: unknown, updatedBy?: string) {
  return fetchJson<RuntimeConfigEntry>(`/api/v1/runtime/config/${key}`, {
    method: "PUT",
    body: JSON.stringify({ payload, updated_by: updatedBy }),
  });
}

export async function getConfigHistory(key: string) {
  return fetchJson<ConfigChangeRecord[]>(`/api/v1/runtime/config/${key}/history`);
}

export async function getConfigSnapshot() {
  return fetchJson<SystemConfig>("/api/v1/runtime/config/snapshot");
}
