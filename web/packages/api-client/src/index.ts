import type {
  ExtensionCommandContribution,
  ExtensionDiagnostic,
  ExtensionHookContribution,
  ExtensionLocaleContribution,
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionProviderContribution,
  ExtensionRuntimeExtension,
  ExtensionRuntimeSnapshot,
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

export type Overview = {
  app_name: string;
  shell_title: LocalizedText;
  default_theme: string;
  modules: string[];
  counts: Record<string, number>;
};

export type Agent = {
  id: string;
  display_name: string;
  kind: string;
  workspace_mode: string;
  default_model: string;
  skills_dir: string;
  workspace_dir: string;
  artifacts_dir: string;
};

export type Space = {
  id: string;
  display_name: string;
  description: string;
  primary_goal: string;
  mention_policy: string;
  default_agents: string[];
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

export type Conversation = {
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

export type Lane = {
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

export type Message = {
  id: string;
  conversation_id: string;
  lane_id?: string | null;
  sender: string;
  role: string;
  body: string;
  mentions: string[];
  created_at: string;
};

export type Run = {
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

export type Task = {
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

export type Artifact = {
  id: string;
  owner: { kind: string; id: string };
  run_id: string;
  conversation_id?: string | null;
  lane_id?: string | null;
  kind: string;
  relative_path: string;
  created_at: string;
};

export type Handoff = {
  id: string;
  from_lane_id: string;
  to_lane_id: string;
  from_agent_id?: string | null;
  to_agent_id?: string | null;
  summary: string;
  instructions: string;
  status: string;
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

export type LogRecord = {
  id: string;
  kind: string;
  level: string;
  title: string;
  summary: string;
  run_id?: string | null;
  task_id?: string | null;
  at: string;
};

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

export type BootstrapState = {
  is_initialized: boolean;
  initialized_at?: string | null;
};

export type BootstrapSetupResponse = {
  bootstrap: BootstrapState;
  profile: WorkspaceProfile;
  preference: UiPreferenceRecord;
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
  shell_title: LocalizedText;
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
  rate_limit: RateLimitConfig;
  cors: CorsConfig;
  timeout: TimeoutConfig;
  logging: LoggingConfig;
  body_limit: BodyLimitConfig;
  bootstrap: BootstrapState;
};

export type ConversationCreateResponse = {
  conversation: Conversation;
  default_lane: Lane;
};

export type ConversationDetailResponse = {
  conversation: Conversation;
  lanes: Lane[];
};

export type ConversationEnvelope = {
  conversation: Conversation;
  lane: Lane;
  message: Message;
  run: Run;
  tasks: Task[];
  artifacts: Artifact[];
};

export type WorkspaceSnapshot = {
  overview: Overview;
  profile: WorkspaceProfile | null;
  agents: Agent[];
  spaces: Space[];
  conversations: Conversation[];
  runs: Run[];
  tasks: Task[];
  artifacts: Artifact[];
  memories: Memory[];
  jobs: Job[];
  registry: ExtensionRuntimeSnapshot;
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
  const headers: Record<string, string> = {
    "content-type": "application/json",
    ...((init?.headers as Record<string, string>) ?? {}),
  };

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
    return undefined as unknown as T;
  }

  return (await response.json()) as T;
}

export async function loadWorkspaceSnapshot(): Promise<WorkspaceSnapshot> {
  const [
    overview,
    profile,
    agents,
    spaces,
    conversations,
    runs,
    tasks,
    artifacts,
    memories,
    jobs,
    registry,
  ] = await Promise.all([
    fetchJson<Overview>("/api/v1/overview"),
    fetchJson<WorkspaceProfile | null>("/api/v1/runtime/profile"),
    fetchJson<Agent[]>("/api/v1/agents"),
    fetchJson<Space[]>("/api/v1/spaces"),
    fetchJson<Conversation[]>("/api/v1/conversations"),
    fetchJson<Run[]>("/api/v1/runs"),
    fetchJson<Task[]>("/api/v1/tasks"),
    fetchJson<Artifact[]>("/api/v1/artifacts"),
    fetchJson<Memory[]>("/api/v1/memories"),
    fetchJson<Job[]>("/api/v1/jobs"),
    fetchJson<ExtensionRuntimeSnapshot>("/api/v1/extensions/runtime"),
  ]);

  return {
    overview,
    profile,
    agents,
    spaces,
    conversations,
    runs,
    tasks,
    artifacts,
    memories,
    jobs,
    registry,
  };
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

export async function fetchUiRuntime(): Promise<UiRuntime> {
  return fetchJson<UiRuntime>("/api/v1/ui/runtime");
}

export async function fetchUiMessages(
  locale: string,
  namespaces: string[] = [],
): Promise<UiMessagesResponse> {
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

export async function fetchSpaceUiPreferences(spaceId: string) {
  return fetchJson<UiPreferenceRecord | null>(`/api/v1/spaces/${spaceId}/ui-preferences`);
}

export async function saveSpaceUiPreferences(
  spaceId: string,
  payload: {
    locale?: string | null;
    theme_id?: string | null;
    time_zone?: string | null;
    date_style?: string | null;
    density?: string | null;
    motion?: string | null;
  },
) {
  return fetchJson<UiPreferenceRecord>(`/api/v1/spaces/${spaceId}/ui-preferences`, {
    method: "PUT",
    body: JSON.stringify(payload),
  });
}

export async function listConversations() {
  return fetchJson<Conversation[]>("/api/v1/conversations");
}

export async function createConversation(payload: {
  topology: "direct" | "group";
  title?: string;
  space_id?: string;
  agent_ids: string[];
  lane_name?: string;
  lane_type?: string;
  lane_goal?: string;
}) {
  return fetchJson<ConversationCreateResponse>("/api/v1/conversations", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function deleteConversation(conversationId: string) {
  return fetchJson<void>(`/api/v1/conversations/${conversationId}`, {
    method: "DELETE",
  });
}

export async function getConversation(conversationId: string) {
  return fetchJson<ConversationDetailResponse>(`/api/v1/conversations/${conversationId}`);
}

export async function loadConversationMessages(conversationId: string) {
  return fetchJson<Message[]>(`/api/v1/conversations/${conversationId}/messages`);
}

export async function loadConversationRuns(conversationId: string) {
  return fetchJson<Run[]>(`/api/v1/conversations/${conversationId}/runs`);
}

export async function loadConversationLanes(conversationId: string) {
  return fetchJson<Lane[]>(`/api/v1/conversations/${conversationId}/lanes`);
}

export async function sendConversationMessage(
  conversationId: string,
  payload: {
    lane_id?: string;
    body: string;
    goal?: string;
    addressed_agents?: string[];
  },
) {
  return fetchJson<ConversationEnvelope>(`/api/v1/conversations/${conversationId}/messages`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function loadLaneHandoffs(laneId: string) {
  return fetchJson<Handoff[]>(`/api/v1/lanes/${laneId}/handoffs`);
}

export async function createLaneHandoff(
  laneId: string,
  payload: {
    to_lane_id: string;
    from_agent_id?: string;
    to_agent_id?: string;
    summary: string;
    instructions: string;
    status?: string;
  },
) {
  return fetchJson<Handoff>(`/api/v1/lanes/${laneId}/handoffs`, {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

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

export async function createJob(payload: {
  owner_kind: string;
  owner_id: string;
  job_kind?: string;
  schedule_kind: string;
  schedule_value: string;
  payload?: unknown;
}) {
  return fetchJson<Job>("/api/v1/jobs", {
    method: "POST",
    body: JSON.stringify(payload),
  });
}

export async function listLogs(limit = 50) {
  return fetchJson<LogRecord[]>(`/api/v1/logs?limit=${limit}`);
}

export async function listConfig() {
  return fetchJson<ConfigEntry[]>("/api/v1/runtime/config");
}

export async function getConfig(key: string) {
  return fetchJson<ConfigEntry>(`/api/v1/runtime/config/${key}`);
}

export async function putConfig(key: string, payload: unknown, updatedBy?: string) {
  return fetchJson<ConfigEntry>(`/api/v1/runtime/config/${key}`, {
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

export async function getExtensionRuntime() {
  return fetchJson<ExtensionRuntimeSnapshot>("/api/v1/extensions/runtime");
}

export async function listExtensionEvents(limit = 50) {
  return fetchJson<
    Array<{
      event_id: string;
      extension_id?: string | null;
      generation: number;
      event: string;
      health?: string | null;
      summary: string;
      diagnostics: ExtensionDiagnostic[];
      occurred_at: string;
    }>
  >(`/api/v1/extensions/events?limit=${limit}`);
}

export async function getExtensionDetail(extensionId: string) {
  return fetchJson<ExtensionRuntimeExtension>(`/api/v1/extensions/${extensionId}`);
}

export async function getExtensionDiagnostics(extensionId: string) {
  return fetchJson<ExtensionDiagnostic[]>(
    `/api/v1/extensions/${extensionId}/diagnostics`,
  );
}

export function getExtensionFrontendModuleUrl(extensionId: string) {
  return `${API_BASE}/api/v1/extensions/${extensionId}/frontend/module`;
}

export async function getExtensionLogs(extensionId: string) {
  const response = await fetch(`${API_BASE}/api/v1/extensions/${extensionId}/logs`);
  return response.text();
}

export async function attachExtensionWorkspace(path: string) {
  return fetchJson<ExtensionRuntimeExtension>("/api/v1/extensions/attach", {
    method: "POST",
    body: JSON.stringify({ path }),
  });
}

export async function reloadExtension(extensionId: string) {
  return fetchJson<ExtensionRuntimeExtension>(`/api/v1/extensions/${extensionId}/reload`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function restartExtension(extensionId: string) {
  return fetchJson<ExtensionRuntimeExtension>(`/api/v1/extensions/${extensionId}/restart`, {
    method: "POST",
    body: JSON.stringify({}),
  });
}

export async function detachExtensionWorkspace(extensionId: string) {
  return fetchJson<void>(`/api/v1/extensions/attach/${extensionId}`, {
    method: "DELETE",
  });
}
