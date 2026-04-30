import type {
  ExtensionBehaviorContribution,
  ExtensionCapabilityContribution,
  ExtensionDiagnostic,
  ExtensionActionContribution,
  ExtensionLocaleContribution,
  ExtensionMemoryContribution,
  ExtensionPageContribution,
  ExtensionPanelContribution,
  ExtensionProviderContribution,
  ExtensionResourceTypeContribution,
  ExtensionScheduleActionContribution,
  ExtensionSubscriptionContribution,
  ExtensionSurfaceContribution,
  ExtensionThemeContribution,
  LocalizedText,
} from "@ennoia/ui-sdk";

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
  show_command_palette: boolean;
};

export type UiRuntime = {
  ui_config: UiConfig;
  registry: {
    resource_types: ExtensionResourceTypeContribution[];
    capabilities: ExtensionCapabilityContribution[];
    surfaces: ExtensionSurfaceContribution[];
    subscriptions: ExtensionSubscriptionContribution[];
    pages: ExtensionPageContribution[];
    panels: ExtensionPanelContribution[];
    themes: ExtensionThemeContribution[];
    locales: ExtensionLocaleContribution[];
    providers: ExtensionProviderContribution[];
    behaviors: ExtensionBehaviorContribution[];
    memories: ExtensionMemoryContribution[];
    actions: ExtensionActionContribution[];
    schedule_actions: ExtensionScheduleActionContribution[];
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

export type RuntimeProfile = {
  id: string;
  display_name: string;
  locale: string;
  time_zone: string;
  default_space_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ServerConfig = {
  host: string;
  port: number;
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
    dev_console: {
      enabled: boolean;
      level: string;
    };
  };
  body_limit: {
    enabled: boolean;
    max_bytes: number;
    per_path_max: Record<string, number>;
  };
  bootstrap: BootstrapState;
};

export type BootstrapSetupResponse = {
  bootstrap: BootstrapState;
  profile: RuntimeProfile;
  preference: UiPreferenceRecord;
};

export type AgentProfile = {
  id: string;
  display_name: string;
  description: string;
  system_prompt: string;
  provider_id: string;
  model_id: string;
  generation_options: Record<string, string>;
  skills: string[];
  enabled: boolean;
  kind?: string;
  default_model?: string;
  skills_dir?: string;
  working_dir?: string;
  artifacts_dir?: string;
};

export type SkillConfig = {
  id: string;
  display_name: string;
  description: string;
  source: string;
  entry: string;
  docs?: string | null;
  keywords: string[];
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
  model_discovery: {
    manual_allowed: boolean;
  };
  enabled: boolean;
};

export type ProviderModelsResponse = {
  provider_id: string;
  source: string;
  models: string[];
  recommended_model?: string | null;
  manual_allowed: boolean;
  generation_options: ExtensionProviderContribution["provider"]["generation_options"];
};

export type PermissionTarget = {
  kind: string;
  id: string;
  conversation_id?: string | null;
  run_id?: string | null;
  path?: string | null;
  host?: string | null;
};

export type PermissionScope = {
  conversation_id?: string | null;
  run_id?: string | null;
  message_id?: string | null;
  extension_id?: string | null;
  path?: string | null;
  host?: string | null;
};

export type PermissionTrigger = {
  kind: string;
  user_initiated: boolean;
};

export type AgentPermissionRule = {
  id: string;
  effect: string;
  actions: string[];
  extension_scope: string[];
  conversation_scope?: string | null;
  run_scope?: string | null;
  path_include: string[];
  path_exclude: string[];
  host_scope: string[];
};

export type AgentPermissionPolicy = {
  mode: string;
  rules: AgentPermissionRule[];
};

export type PermissionPolicySummary = {
  agent_id: string;
  mode: string;
  allow_count: number;
  ask_count: number;
  deny_count: number;
};

export type PermissionEventRecord = {
  event_id: string;
  agent_id: string;
  action: string;
  decision: string;
  target: PermissionTarget;
  scope: PermissionScope;
  extension_id?: string | null;
  matched_rule_id?: string | null;
  approval_id?: string | null;
  trace_id?: string | null;
  created_at: string;
};

export type PermissionApprovalRecord = {
  approval_id: string;
  status: string;
  agent_id: string;
  action: string;
  target: PermissionTarget;
  scope: PermissionScope;
  trigger: PermissionTrigger;
  matched_rule_id?: string | null;
  reason: string;
  created_at: string;
  expires_at?: string | null;
  resolved_at?: string | null;
  resolution?: string | null;
};

export type ChatThread = {
  id: string;
  topology: "direct" | "group";
  owner: { kind: string; id: string };
  space_id?: string | null;
  title: string;
  participants: string[];
  active_branch_id?: string | null;
  default_lane_id?: string | null;
  created_at: string;
  updated_at: string;
};

export type ChatBranch = {
  id: string;
  conversation_id: string;
  name: string;
  kind: string;
  status: string;
  parent_branch_id?: string | null;
  source_message_id?: string | null;
  source_checkpoint_id?: string | null;
  inherit_mode: string;
  created_at: string;
  updated_at: string;
};

export type ChatCheckpoint = {
  id: string;
  conversation_id: string;
  branch_id: string;
  message_id?: string | null;
  kind: string;
  label: string;
  created_at: string;
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
  branch_id?: string | null;
  lane_id?: string | null;
  sender: string;
  role: "operator" | "agent" | "system" | "tool";
  body: string;
  mentions: string[];
  reply_to_message_id?: string | null;
  rewrite_from_message_id?: string | null;
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
  conversation: ChatThread;
  lanes: ChatLane[];
  branches: ChatBranch[];
  checkpoints: ChatCheckpoint[];
  messages: ChatMessage[];
  runs: ExecutionRun[];
  tasks: ExecutionStep[];
  outputs: RunOutput[];
};

export type ChatSendResponse = {
  conversation: ChatThread;
  lane: ChatLane;
  branch: ChatBranch;
  message: ChatMessage;
  run?: ExecutionRun;
  runs?: ExecutionRun[];
  tasks: ExecutionStep[];
  artifacts: RunOutput[];
};

export type ExtensionRuntimeState = {
  id: string;
  name: string;
  enabled: boolean;
  status: string;
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
  description: string;
  docs?: string | null;
  conversation: {
    inject: boolean;
    resource_types: string[];
    capabilities: string[];
  };
  kind: string;
  source_mode: string;
  source_root: string;
  install_dir: string;
  generation: number;
  health: string;
  ui?: {
    kind: string;
    entry: string;
    hmr: boolean;
    version: string;
  } | null;
  worker?: {
    kind: string;
    entry: string;
    abi: string;
    protocol?: string | null;
    status: string;
  } | null;
  permissions: {
    storage?: string | null;
    sqlite: boolean;
    network: string[];
    events: string[];
    fs: string[];
    env: string[];
  };
  runtime: {
    startup: string;
    timeout_ms: number;
    memory_limit_mb: number;
  };
  capabilities: {
    resource_types: boolean;
    capabilities: boolean;
    surfaces: boolean;
    locales: boolean;
    themes: boolean;
    commands: boolean;
    subscriptions: boolean;
  };
  resource_types: ExtensionResourceTypeContribution["resource_type"][];
  capability_rows: ExtensionCapabilityContribution["capability"][];
  surfaces: ExtensionSurfaceContribution["surface"][];
  pages: ExtensionPageContribution["page"][];
  panels: ExtensionPanelContribution["panel"][];
  themes: ExtensionThemeContribution["theme"][];
  locales: ExtensionLocaleContribution["locale"][];
  commands: {
    id: string;
    title: LocalizedText;
    action: string;
    shortcut?: string | null;
  }[];
  providers: ExtensionProviderContribution["provider"][];
  behaviors: ExtensionBehaviorContribution["behavior"][];
  memories: ExtensionMemoryContribution["memory"][];
  hooks: {
    event: string;
    handler?: string | null;
  }[];
  actions: ExtensionActionContribution["action"][];
  schedule_actions: ExtensionScheduleActionContribution["schedule_action"][];
  subscriptions: ExtensionSubscriptionContribution["subscription"][];
  diagnostics: ExtensionDiagnostic[];
};

export type SystemLog = {
  id: string;
  kind?: string;
  source: string;
  level: string;
  title: string;
  summary: string;
  details?: string | null;
  run_id?: string | null;
  task_id?: string | null;
  at: string;
};

export type ObservationOverview = {
  log_count: number;
  span_count: number;
  trace_count: number;
};

export type ObservationLogEntry = {
  id: string;
  seq: number;
  event: string;
  level: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  request_id?: string | null;
  trace_id?: string | null;
  span_id?: string | null;
  parent_span_id?: string | null;
  message: string;
  attributes: unknown;
  created_at: string;
};

export type ObservationSpanRecord = {
  id: string;
  seq: number;
  trace_id: string;
  span_id: string;
  parent_span_id?: string | null;
  request_id: string;
  sampled: boolean;
  source: string;
  kind: string;
  name: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  status: string;
  attributes: unknown;
  started_at: string;
  ended_at: string;
  duration_ms: number;
};

export type ObservationSpanLinkRecord = {
  id: string;
  seq: number;
  trace_id: string;
  span_id: string;
  linked_trace_id: string;
  linked_span_id: string;
  link_type: string;
  attributes: unknown;
  created_at: string;
};

export type ObservationTraceDetail = {
  trace_id: string;
  spans: ObservationSpanRecord[];
  links: ObservationSpanLinkRecord[];
};

export type ObservationLogQuery = {
  event?: string;
  level?: string;
  component?: string;
  source_kind?: string;
  source_id?: string;
  request_id?: string;
  trace_id?: string;
  cursor?: number;
  limit?: number;
};

export type ObservationTraceQuery = {
  request_id?: string;
  component?: string;
  kind?: string;
  source_kind?: string;
  source_id?: string;
  limit?: number;
};

export type ActionImplementation = {
  extension_id: string;
  capability_id: string;
  method: string;
  phase: string;
  priority: number;
  enabled: boolean;
  result_mode: string;
  when: unknown;
  schema?: string | null;
  extension_status: string;
};

export type ActionStatus = {
  action: string;
  rules: ActionImplementation[];
  execute_rule_count: number;
};

export type ScheduleTrigger =
  | { kind: "once"; at: string }
  | { kind: "interval"; every_seconds: number }
  | { kind: "cron"; expression: string; next_run_at: string };

export type ScheduleExecutor =
  | {
      kind: "command";
      command: {
        command: string;
        cwd?: string | null;
        timeout_ms?: number | null;
      };
    }
  | {
      kind: "agent";
      agent: {
        agent_id: string;
        prompt: string;
        model_id?: string | null;
        max_turns?: number | null;
        context?: {
          conversation_id?: string | null;
        };
      };
    };

export type ScheduleDelivery = {
  conversation_id?: string | null;
  lane_id?: string | null;
  content_mode?: "full" | "summary" | "conclusion" | null;
};

export type ScheduleRetryPolicy = {
  max_attempts?: number;
  backoff_seconds?: number;
};

export type ScheduleRunRecord = {
  id: string;
  started_at: string;
  finished_at: string;
  attempt: number;
  status: string;
  error?: string | null;
  delivered?: boolean;
  delivery_error?: string | null;
  output?: unknown;
};

export type ScheduleRecord = {
  id: string;
  name?: string | null;
  description?: string | null;
  owner: unknown;
  trigger: ScheduleTrigger;
  executor: ScheduleExecutor;
  delivery?: ScheduleDelivery;
  retry?: ScheduleRetryPolicy;
  enabled: boolean;
  next_run_at?: string | null;
  last_run_at?: string | null;
  last_status?: string | null;
  last_error?: string | null;
  last_output?: unknown;
  history?: ScheduleRunRecord[];
  created_at: string;
  updated_at: string;
};

export type SchedulePayload = {
  name?: string | null;
  description?: string | null;
  owner?: unknown;
  trigger: ScheduleTrigger;
  executor: ScheduleExecutor;
  delivery?: ScheduleDelivery;
  retry?: ScheduleRetryPolicy;
  enabled?: boolean;
};

export type SystemLogEntry = {
  id: string;
  seq: number;
  event: string;
  level: string;
  component: string;
  source_kind: string;
  source_id?: string | null;
  summary: string;
  payload: unknown;
  created_at: string;
};



